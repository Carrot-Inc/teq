//! The nodes of a resident's interpreter that can close a cycle of reference counts, and the
//! sweep that breaks every cycle at a boundary where nothing of the interpreter is live.
//!
//! A macro's run makes frames, objects, arrays and maps that hold each other by `Rc`: a closure
//! keeps the frame it was made in and the frame keeps the closure through a binding, an
//! object keeps a closure of its own field that keeps the object as `this`, an array is stored
//! into itself, a tree's nodes point at their parents. Nothing frees such a cycle, and a
//! resident session that expands macros grows by them with every full build (95 MB on the
//! application; docs/SPEED.md, "A session's memory"). A one-shot build ends with the process
//! and pays nothing here.
//!
//! Every cycle runs through an edge that was written after its node was made: a node can only
//! point at nodes that existed when it was made, unless it is written to later. So the nodes
//! registered are the holders of such edges: a frame when a closure or an object captures it,
//! with its ancestors up to the first one already registered (a closure over a nested scope is
//! kept by the scope around it, and a pooled frame reused under a younger parent registers
//! the parent likewise, `fast.rs`); an object when a closure captures it as `this` and when a
//! reference is written into one of its fields by anything but the binding of its
//! constructor's arguments (an initialiser in the class body counts, since `val b = new B(this)`
//! closes a cycle while the constructor runs); an array and a map when they are made, since
//! what makes them has no way of telling which will be written into later. The registration is
//! intrusive: the node's `slot` is the index of its entry plus one, taken out again by the
//! node's `Drop`, so that every occupied entry points at a live node of this thread. No weak
//! reference is kept: one would keep the block allocated and stop the frame pool
//! (`fast.rs`) from recycling a frame that a closure captured.
//!
//! The sweep runs on the interpreter's thread where nothing of the interpreter is live: at the
//! start of a full build, after the caches of the previous program were dropped
//! (`typer::Typer::run`), and before a thread that ran the interpreter ends (`dispose`). It
//! empties every registered node in two passes: the contents of every node are moved into one
//! list, then the list is dropped, so that no node is freed while the entries are walked; a
//! node freed by the drop takes its entry out. What survives (nothing, at a full build) stays
//! registered. A registry is the thread's own, so an interpreter per worker thread owns one
//! each; the roots a sweep keeps (a macro's promised objects) are the next step.
//!
//! The invariant every entry stands on: a node is registered, swept and dropped on one
//! thread, the one that made it. `Rc` is not `Send`, so a value cannot move to another
//! thread; what could is a whole structure of values moved by a `transmute` or an `unsafe
//! impl Send`, and none exists. A node dropped on another thread would take an entry out of
//! that thread's registry (`vacate`), which a debug build checks by the thread that made the
//! registry.

use super::value::{ArrayCell, HMap, MapCell, Object, Value};
use super::Frame;
use crate::types::SymId;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// An entry: a registered node, or a vacant entry chained to the next vacant one.
enum Node {
    Frame(*const Frame),
    Object(*const Object),
    Array(*const ArrayCell),
    Map(*const MapCell),
    Vacant(u32),
}

struct Registry {
    nodes: Vec<Node>,
    /// The first vacant entry plus one, or zero.
    free: u32,
    live: usize,
    /// The last sweep: the nodes emptied, the nodes freed by that, the microseconds it took.
    last: (usize, usize, u64),
    #[cfg(debug_assertions)]
    owner: Option<std::thread::ThreadId>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = const {
        RefCell::new(Registry {
            nodes: Vec::new(),
            free: 0,
            live: 0,
            last: (0, 0, 0),
            #[cfg(debug_assertions)]
            owner: None,
        })
    };
}

/// A debug build's check that the registry is used by the thread that made it.
#[cfg(debug_assertions)]
fn check_owner(r: &mut Registry) {
    let me = std::thread::current().id();
    match r.owner {
        Some(owner) => assert!(owner == me, "the interpreter's registry is used by another thread than the one that made it"),
        None => r.owner = Some(me),
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn check_owner(_: &mut Registry) {}

/// What the registry holds and what its last sweep did.
#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub registered: usize,
    pub frames: usize,
    pub objects: usize,
    pub arrays: usize,
    pub maps: usize,
    pub swept: usize,
    pub freed: usize,
    pub sweep_us: u64,
}

/// Whether this process registers. A batch run pays this check at each site; what registers
/// is out of line.
#[inline(always)]
pub(super) fn on() -> bool {
    crate::alloc::is_resident()
}

fn push(node: Node) -> u32 {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        check_owner(&mut r);
        r.live += 1;
        if r.free != 0 {
            let at = r.free - 1;
            let Node::Vacant(next) = r.nodes[at as usize] else { unreachable!("the vacant chain names an occupied entry") };
            r.free = next;
            r.nodes[at as usize] = node;
            at + 1
        } else {
            r.nodes.push(node);
            r.nodes.len() as u32
        }
    })
}

/// Takes a node's entry out, from the node's `Drop`, on the thread that registered it (the
/// module's header). The registry is never borrowed while a node can drop: registration and
/// the sweep's first pass move nothing that owns a node.
pub(super) fn vacate(slot: u32) {
    // A node dropped by a thread-local's destructor after the registry's is not registered
    // any more.
    let _ = REGISTRY.try_with(|r| {
        let mut r = r.borrow_mut();
        check_owner(&mut r);
        let at = slot - 1;
        let free = r.free;
        r.nodes[at as usize] = Node::Vacant(free);
        r.free = slot;
        r.live -= 1;
    });
}

/// A frame that a closure or an object captured, with its ancestors up to the first one
/// registered already.
#[cold]
#[inline(never)]
pub(super) fn frame(f: &Rc<Frame>) {
    let mut at = f;
    loop {
        if at.slot.get() != 0 {
            return;
        }
        at.slot.set(push(Node::Frame(Rc::as_ptr(at))));
        match &at.parent {
            Some(parent) => at = parent,
            None => return,
        }
    }
}

/// An object that a closure captured as `this`, or that a reference was written into.
#[cold]
#[inline(never)]
pub(super) fn object(o: &Rc<Object>) {
    if o.slot.get() == 0 {
        o.slot.set(push(Node::Object(Rc::as_ptr(o))));
    }
}

#[cold]
#[inline(never)]
pub(super) fn array(a: &Rc<ArrayCell>) {
    if a.slot.get() == 0 {
        a.slot.set(push(Node::Array(Rc::as_ptr(a))));
    }
}

#[cold]
#[inline(never)]
pub(super) fn map(m: &Rc<MapCell>) {
    if m.slot.get() == 0 {
        m.slot.set(push(Node::Map(Rc::as_ptr(m))));
    }
}

/// A closure is about to keep `env` and `this`.
#[cold]
#[inline(never)]
pub(super) fn captured(env: &Rc<Frame>, this: &Value) {
    frame(env);
    if let Value::Obj(o) = this {
        object(o);
    }
}

/// A reference `v` is about to be written into a field of `obj`.
#[cold]
#[inline(never)]
pub(super) fn written(obj: &Value, v: &Value) {
    if let Value::Obj(o) = obj {
        if refers(v) {
            object(o);
        }
    }
}

/// Whether a value can hold a reference to a node: what a write of it can close a cycle with.
pub(super) fn refers(v: &Value) -> bool {
    matches!(v, Value::Obj(_) | Value::Fun(_) | Value::Array(_) | Value::Map(_) | Value::Trie(_))
}

/// The contents of a node, taken out of it. Nothing reads them: they are kept to drop after the
/// registry's borrow ends (`sweep`), since dropping a value can drop a node, whose `Drop` takes
/// its entry out of the registry.
enum Taken {
    Vals { _kept: Vec<(SymId, Value)> },
    Fields { _kept: Vec<Value> },
    Items { _kept: Vec<Value> },
    Map { _kept: HMap },
}

/// Empties every registered node, where nothing of the interpreter is live on this thread.
pub fn sweep() {
    let t0 = Instant::now();
    let (taken, before): (Vec<Taken>, usize) = REGISTRY.with(|r| {
        let r = r.borrow();
        let taken = r
            .nodes
            .iter()
            .filter_map(|node| {
                // SAFETY: an occupied entry points at a live node of this thread, since a node
                // takes its entry out in its `Drop`; the node's cell is not borrowed, since no
                // interpreter runs on the thread; and taking the contents drops nothing, so no
                // node drops while the registry is borrowed.
                unsafe {
                    match *node {
                        Node::Frame(f) => Some(Taken::Vals { _kept: (*f).vals.take() }),
                        Node::Object(o) => Some(Taken::Fields { _kept: (*o).fields.take() }),
                        Node::Array(a) => Some(Taken::Items { _kept: (*a).take() }),
                        Node::Map(m) => Some(Taken::Map { _kept: std::mem::take(&mut *(*m).borrow_mut()) }),
                        Node::Vacant(_) => None,
                    }
                }
            })
            .collect();
        (taken, r.live)
    });
    let swept = taken.len();
    drop(taken);
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        r.last = (swept, before - r.live, t0.elapsed().as_micros() as u64);
        if r.live == 0 {
            r.nodes = Vec::new();
            r.free = 0;
        }
    });
}

pub fn stats() -> Stats {
    REGISTRY.with(|r| {
        let r = r.borrow();
        let mut s = Stats { registered: r.live, swept: r.last.0, freed: r.last.1, sweep_us: r.last.2, ..Stats::default() };
        for node in &r.nodes {
            match node {
                Node::Frame(_) => s.frames += 1,
                Node::Object(_) => s.objects += 1,
                Node::Array(_) => s.arrays += 1,
                Node::Map(_) => s.maps += 1,
                Node::Vacant(_) => {}
            }
        }
        s
    })
}

#[cfg(test)]
mod tests {
    use super::super::value::{ArrayCell, ClosureKind, Closure, MapCell, Object, Value};
    use super::super::Frame;
    use super::*;
    use crate::types::{ClassId, SymId};
    use std::cell::Cell;
    use std::rc::Weak;

    /// The tests register in a resident process, one at a time with the allocator's own
    /// (`alloc::serial_test`): the resident flag and the centre's counts are the process's.
    fn resident() -> std::sync::MutexGuard<'static, ()> {
        crate::alloc::serial_test()
    }

    fn an_object() -> Rc<Object> {
        Rc::new(Object { class: ClassId(0), fields: RefCell::new(Vec::new()), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(0) })
    }

    fn closure(env: &Rc<Frame>, this: Value) -> Value {
        Value::Fun(Rc::new(Closure { kind: ClosureKind::Miss, env: env.clone(), this, def_key: 0, class: None, hash: Cell::new(0) }))
    }

    /// The cycles of every registered kind, each as a weak reference to one of its nodes.
    fn cycles() -> Vec<Weak<dyn std::any::Any>> {
        let mut weak: Vec<Weak<dyn std::any::Any>> = Vec::new();
        // outer -> closure -> inner, whose parent is outer.
        let outer = Frame::new(None);
        let inner = Frame::new(Some(outer.clone()));
        frame(&inner);
        outer.bind(SymId(1), closure(&inner, Value::Unit));
        weak.push(Rc::downgrade(&outer) as Weak<dyn std::any::Any>);
        // An object kept by a closure of its own field.
        let o = an_object();
        object(&o);
        o.fields.borrow_mut().push(closure(&Frame::new(None), Value::Obj(o.clone())));
        weak.push(Rc::downgrade(&o) as Weak<dyn std::any::Any>);
        // An array that holds itself, a map that holds itself.
        let a = ArrayCell::new(Vec::new());
        a.borrow_mut().push(Value::Array(a.clone()));
        weak.push(Rc::downgrade(&a) as Weak<dyn std::any::Any>);
        let m = MapCell::new(HMap::default());
        m.borrow_mut().entries.push(Some((Value::Int(1), Value::Map(m.clone()))));
        weak.push(Rc::downgrade(&m) as Weak<dyn std::any::Any>);
        weak
    }

    #[test]
    fn a_sweep_frees_every_kind_of_cycle() {
        let _serial = resident();
        let weak = cycles();
        assert!(weak.iter().all(|w| w.upgrade().is_some()), "the cycles hold themselves");
        let registered = stats().registered;
        assert!(registered >= 5, "{registered} registered");
        sweep();
        assert!(weak.iter().all(|w| w.upgrade().is_none()), "a cycle survived the sweep");
        let after = stats();
        assert_eq!(after.freed, after.swept);
        assert_eq!(after.registered, registered - after.freed);
    }

    #[test]
    fn a_node_dropped_takes_its_entry_out_and_the_entry_is_reused() {
        let _serial = resident();
        sweep();
        let before = stats().registered;
        let a = ArrayCell::new(Vec::new());
        let slot = a.slot.get();
        assert!(slot != 0);
        drop(a);
        assert_eq!(stats().registered, before);
        let b = ArrayCell::new(Vec::new());
        assert_eq!(b.slot.get(), slot, "the vacant entry is taken again");
    }

    /// The registry's part of a worker thread's end: the cycles are made by the
    /// constructors the interpreter uses, since a macro's run needs a typed program, and
    /// `dispose` is what the thread calls last.
    #[test]
    fn a_thread_that_ran_the_interpreter_leaves_nothing_behind() {
        let _serial = resident();
        let (registered, freed) = std::thread::spawn(|| {
            let weak = cycles();
            super::super::dispose();
            (stats().registered, weak.iter().filter(|w| w.upgrade().is_none()).count())
        })
        .join()
        .unwrap();
        assert_eq!(registered, 0);
        assert_eq!(freed, 4);
    }
}
