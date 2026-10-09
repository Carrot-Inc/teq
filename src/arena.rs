//! The arenas of the typer's records (`Symbols`, `Program`) as the parallel typer's workers hold
//! them: the signature phase's records and what the cells
//! add to them are the shared region, read by every worker by reference; what a worker's bodies
//! make is its local chunk, whose ids carry `LOCAL_BASE` so that the two never meet, whatever
//! the shared region grows to; the merge renumbers the chunks into the shared region's tail.
//!
//! With one worker (the signature phase, a watch session's retype, the passes after the merge)
//! the arena is a plain vector (`Arena::plain`) read by the id itself, and an access costs what
//! a vector's does; a forked worker's own records are a vector apart, read by the id less the
//! worker's base, after a first compare against the empty plain vector. Shared
//! records are immutable but for two things: a record pushed under the loader's lock is the
//! lock holder's to fill until the lock is released (staged: nothing published refers to it
//! before then), and a published record changes by a copy that replaces it (`overrides`),
//! made under the loader's lock as a working copy and published when the lock is released or,
//! for a cell's state, at once (`publish`). Readers see the record as it was or as it became,
//! never half-written. `SharedMap` and `Layered` are the same discipline for the tables keyed
//! by an id.

use crate::intern::FxMap;
use crate::shared::{hash_of, lock_depth, CellState, Serial, SlabVec, Table};
use std::alloc::GlobalAlloc;
use std::hash::Hash;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// The first local id: the shared region's ids stay below it. Each worker's own ids start at
/// `LOCAL_BASE + worker * WORKER_SPAN`, so that an id says whose it is (the merge reads that).
pub const LOCAL_BASE: u32 = 1 << 27;
pub const WORKER_SPAN: u32 = 1 << 24;
/// The most workers the ids tell apart.
pub const MAX_WORKERS: usize = ((u32::MAX - LOCAL_BASE) / WORKER_SPAN) as usize;

/// What an arena holds: a record, copied when its buffer grows or when a working copy of it
/// is published. Its completion cells live outside it (`Cells`), so that no copy carries one.
pub trait Record: Clone {}

macro_rules! plain_records {
    ($($t:ty),*) => { $(impl Record for $t {})* };
}
plain_records!(u32, u64, String);

/// A run of one worker's own records, `start..end` of its own vector, that the merge places
/// next: the segments of an arena in canonical order say where every own record goes.
#[derive(Clone, Copy)]
pub struct Segment {
    pub worker: usize,
    pub start: u32,
    pub end: u32,
}

/// A run of one worker's own records, `start..end` of its own vector, placed from `at` on in the
/// merged arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub worker: usize,
    pub start: u32,
    pub end: u32,
    pub at: u32,
}

/// The positional numbering of an arena's merge: after the
/// `shared` records the shared region retains, the segments in their canonical order, then what
/// no segment covers, worker by worker in its order; every run's place by the prefix sums of the
/// runs before it. `covered` is each worker's segments in its own order (sorted here if they are
/// not), `lens` its count of own records. The runs are in the order of their places;
/// `uncovered` counts the records outside every segment.
pub struct Numbering {
    pub runs: Vec<Run>,
    pub shared: u32,
    pub len: u32,
    pub uncovered: usize,
}

impl Numbering {
    pub fn of(shared: u32, segments: &[Segment], covered: Vec<Vec<(u32, u32)>>, lens: &[usize]) -> Numbering {
        let mut runs = Vec::with_capacity(segments.len() + lens.len());
        let mut at = shared;
        for s in segments {
            runs.push(Run { worker: s.worker, start: s.start, end: s.end, at });
            at += s.end - s.start;
        }
        let mut uncovered = 0;
        let mut covered = covered;
        covered.resize_with(lens.len(), Vec::new);
        for (k, mut covered) in covered.into_iter().enumerate().take(lens.len()) {
            if !covered.windows(2).all(|w| w[0] <= w[1]) {
                covered.sort_unstable();
            }
            let len = lens[k] as u32;
            let mut next = 0;
            for (start, end) in covered.into_iter().chain(std::iter::once((len, len))) {
                assert!(start >= next && end <= len, "a record placed twice");
                if start > next {
                    runs.push(Run { worker: k, start: next, end: start, at });
                    at += start - next;
                    uncovered += (start - next) as usize;
                }
                next = end;
            }
        }
        Numbering { runs, shared, len: at, uncovered }
    }

    /// Worker `k`'s table of new ids, by own index, into `out`, which has room for its `len`
    /// records: every slot written once, since the runs of a worker cover its records exactly
    /// (`of`).
    fn table_into(&self, k: usize, out: *mut u32) {
        for r in self.runs.iter().filter(|r| r.worker == k) {
            for i in r.start..r.end {
                // SAFETY: below the worker's count of records, the caller's room.
                unsafe { out.add(i as usize).write(r.at + (i - r.start)) };
            }
        }
    }

    /// The runs that place records in `range` of the merged ids, each cut to the range.
    pub fn runs_in(&self, range: std::ops::Range<u32>) -> impl Iterator<Item = Run> + '_ {
        let first = self.runs.partition_point(|r| r.at + (r.end - r.start) <= range.start);
        self.runs[first..].iter().take_while(move |r| r.at < range.end).map(move |r| {
            let lo = r.at.max(range.start);
            let hi = (r.at + (r.end - r.start)).min(range.end);
            Run { worker: r.worker, start: r.start + (lo - r.at), end: r.start + (hi - r.at), at: lo }
        })
    }
}

/// Moves `owns`' records to their places in `records` (`numbering`, past the `shared` records it
/// holds), each thread of the crew its contiguous share of the places, a copy per run, and makes
/// each worker's table of new ids by own index; the sources are left empty without dropping what
/// was moved out of them.
pub fn gather<T: Send>(records: &mut Vec<T>, mut owns: Vec<Vec<T>>, numbering: &Numbering, crew: &crate::crew::Crew) -> Vec<Vec<u32>> {
    let (shared, len) = (numbering.shared as usize, numbering.len as usize);
    assert_eq!(records.len(), shared);
    assert_eq!(owns.iter().map(|v| v.len()).sum::<usize>(), len - shared);
    records.reserve_exact(len - shared);
    let mut new: Vec<Vec<u32>> = owns.iter().map(|v| Vec::with_capacity(v.len())).collect();
    let dst = crate::crew::Shared(records.as_mut_ptr());
    let srcs: Vec<crate::crew::Shared<T>> = owns.iter_mut().map(|v| crate::crew::Shared(v.as_mut_ptr())).collect();
    let tables: Vec<crate::crew::Shared<u32>> = new.iter_mut().map(|v| crate::crew::Shared(v.as_mut_ptr())).collect();
    let (dst, srcs, tables) = (&dst, &srcs, &tables);
    crew.run(&|k| {
        let mine = crew.share(k, shared..len);
        for r in numbering.runs_in(mine.start as u32..mine.end as u32) {
            // SAFETY: the runs' places are disjoint and within the reserved length, each read once.
            unsafe { std::ptr::copy_nonoverlapping(srcs[r.worker].0.add(r.start as usize), dst.0.add(r.at as usize), (r.end - r.start) as usize) };
        }
        let mut w = k;
        while w < tables.len() {
            numbering.table_into(w, tables[w].0);
            w += crew.threads();
        }
    });
    // SAFETY: every place from `shared` to `len` was written once, and every slot of each table;
    // the sources' records moved.
    unsafe {
        records.set_len(len);
        for (v, t) in owns.iter_mut().zip(new.iter_mut()) {
            t.set_len(v.len());
            v.set_len(0);
        }
    }
    new
}

/// The worker an id belongs to, when it is a worker's own.
#[inline]
pub fn worker_of(id: u32) -> Option<usize> {
    (id >= LOCAL_BASE).then(|| ((id - LOCAL_BASE) / WORKER_SPAN) as usize)
}

#[inline]
pub fn worker_base(worker: usize) -> u32 {
    LOCAL_BASE + worker as u32 * WORKER_SPAN
}

/// The completion cells of an arena's records, outside the records: a
/// record is copied when its buffer grows or when a working copy of it is published, and a claim
/// made on one copy would be unseen through the other, so no copy carries a cell. A cell lives in
/// a chunk that never moves, found by a shift of the record's id in one directory over the whole id
/// space, the shared region's ids and every worker's own ranges alike (`worker_base`); the
/// cell is found by a mask. The directory is 8 MB of zero pages mapped from the system
/// (`alloc::zeroed_pages`), of which a build touches a slot per 4,096 ids it makes; a chunk is made on the first write to
/// it, and a read of a cell without a chunk is `NotStarted`. Every worker's view is the one
/// directory, so a read is the slot, the chunk and the cell. A directory outlives its store: the
/// last handle's drop clears the chunks made and puts the directory by for the next store
/// (`recycled`), so a session's full builds keep one set of chunks instead of freeing and
/// taking 16 KB blocks by the hundred on every build.
pub struct Cells {
    directory: Option<Arc<CellDirectory>>,
    /// The directory's slots, for the read: the `Arc` keeps them.
    slots: *const AtomicPtr<CellChunk>,
}

unsafe impl Send for Cells {}
unsafe impl Sync for Cells {}

const CELL_CHUNK_BITS: u32 = 12;
const CELL_CHUNK: usize = 1 << CELL_CHUNK_BITS;
type CellChunk = [CellState; CELL_CHUNK];
const CELL_SLOTS: usize = 1 << (32 - CELL_CHUNK_BITS);
const CELL_DIRECTORY_BYTES: usize = CELL_SLOTS * std::mem::size_of::<AtomicPtr<CellChunk>>();

struct CellDirectory {
    slots: *mut AtomicPtr<CellChunk>,
    /// The slots with a chunk, in the order made: what a clearing walks.
    made: std::sync::Mutex<Vec<u32>>,
}

static SPARE_DIRECTORIES: std::sync::Mutex<Vec<Arc<CellDirectory>>> = std::sync::Mutex::new(Vec::new());

unsafe impl Send for CellDirectory {}
unsafe impl Sync for CellDirectory {}

impl CellDirectory {
    fn new() -> CellDirectory {
        let slots = crate::alloc::zeroed_pages(CELL_DIRECTORY_BYTES) as *mut AtomicPtr<CellChunk>;
        assert!(!slots.is_null(), "no memory for a cell directory");
        CellDirectory { slots, made: std::sync::Mutex::new(Vec::new()) }
    }

    #[cold]
    fn make_chunk(&self, at: usize) -> *mut CellChunk {
        let slot = unsafe { &*self.slots.add(at) };
        let layout = std::alloc::Layout::new::<CellChunk>();
        let fresh = unsafe { std::alloc::System.alloc_zeroed(layout) } as *mut CellChunk;
        assert!(!fresh.is_null(), "no memory for a cell chunk");
        match slot.compare_exchange(std::ptr::null_mut(), fresh, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {
                self.made.lock().unwrap().push(at as u32);
                fresh
            }
            Err(made) => {
                unsafe { std::alloc::System.dealloc(fresh as *mut u8, layout) };
                made
            }
        }
    }

    /// Every cell `NotStarted` again, by the chunks made: for the directory's next store.
    fn clear(&self) {
        for &at in self.made.lock().unwrap().iter() {
            let chunk = unsafe { (*self.slots.add(at as usize)).load(Ordering::Acquire) };
            unsafe { std::ptr::write_bytes(chunk, 0, 1) };
        }
    }

    fn held(&self) -> usize {
        self.made.lock().unwrap().len() * std::mem::size_of::<CellChunk>()
    }
}

impl Drop for CellDirectory {
    fn drop(&mut self) {
        unsafe {
            for &at in self.made.get_mut().unwrap().iter() {
                let chunk = (*self.slots.add(at as usize)).load(Ordering::Relaxed);
                std::alloc::System.dealloc(chunk as *mut u8, std::alloc::Layout::new::<CellChunk>());
            }
            crate::alloc::free_zeroed_pages(self.slots as *mut u8, CELL_DIRECTORY_BYTES);
        }
    }
}

impl Cells {
    /// A store over a directory of its own, every cell `NotStarted`.
    pub fn new() -> Cells {
        Cells::over(Arc::new(CellDirectory::new()))
    }

    /// A store over a directory put by when its last store dropped, when there is one: its
    /// chunks are made already, and clear.
    pub fn recycled() -> Cells {
        match SPARE_DIRECTORIES.lock().unwrap().pop() {
            Some(directory) => Cells::over(directory),
            None => Cells::new(),
        }
    }

    fn over(directory: Arc<CellDirectory>) -> Cells {
        let slots = directory.slots as *const AtomicPtr<CellChunk>;
        Cells { directory: Some(directory), slots }
    }

    /// Another worker's view: the same cells.
    pub fn attach(&self, _worker: usize) -> Cells {
        Cells { directory: self.directory.clone(), slots: self.slots }
    }

    /// The cell of the record `i`, made with its chunk when `make`.
    #[inline]
    fn find(&self, i: u32, make: bool) -> Option<&CellState> {
        let at = (i >> CELL_CHUNK_BITS) as usize;
        let mut chunk = unsafe { (*self.slots.add(at)).load(Ordering::Acquire) };
        if chunk.is_null() {
            if !make {
                return None;
            }
            chunk = self.directory.as_ref().expect("a store's directory").make_chunk(at);
        }
        Some(unsafe { &(*chunk)[i as usize & (CELL_CHUNK - 1)] })
    }

    /// The chunks' bytes.
    pub fn held(&self) -> usize {
        self.directory.as_ref().map_or(0, |d| d.held())
    }

    #[inline]
    pub fn cell(&self, i: u32) -> CellRef<'_> {
        CellRef { cells: self, id: i }
    }

    #[inline]
    pub fn get(&self, i: u32) -> crate::symbols::Completion {
        self.find(i, false).map_or(crate::symbols::Completion::NotStarted, |c| c.get())
    }

    pub fn set(&self, i: u32, c: crate::symbols::Completion) {
        if c == crate::symbols::Completion::NotStarted && self.find(i, false).is_none() {
            return;
        }
        self.find(i, true).expect("a cell").set(c)
    }

    /// `Done` for a record this thread has just made, which no other thread has an id of and
    /// no thread waits on: stored as it is, not staged under the lock, and no waiter woken.
    #[inline]
    pub fn done_fresh(&self, i: u32) {
        self.find(i, true).expect("a cell").done_fresh()
    }

    /// Claims the cell for this thread: whether it was unclaimed.
    pub fn claim(&self, i: u32) -> bool {
        self.find(i, true).expect("a cell").claim()
    }

    #[inline]
    pub fn mine(&self, i: u32) -> bool {
        self.find(i, false).map_or(false, |c| c.mine())
    }

    #[inline]
    pub fn owner(&self, i: u32) -> Option<usize> {
        self.find(i, false).and_then(|c| c.owner())
    }

    /// The cells of worker `worker`'s own records at the ids the merge gave them (`new`, by
    /// own index): what the passes after the merge read of those records. The worker's range
    /// is cleared for the next fork, whose records start at its base again.
    pub fn carry(&self, worker: usize, new: &[u32]) {
        for (i, &id) in new.iter().enumerate() {
            let own = worker_base(worker) + i as u32;
            let c = self.get(own);
            if c != crate::symbols::Completion::NotStarted {
                self.set(own, crate::symbols::Completion::NotStarted);
                if id != u32::MAX {
                    self.set(id, c);
                }
            }
        }
    }
}

impl Default for Cells {
    fn default() -> Cells {
        Cells::new()
    }
}

impl Drop for Cells {
    fn drop(&mut self) {
        if let Some(directory) = self.directory.take().and_then(Arc::into_inner) {
            directory.clear();
            SPARE_DIRECTORIES.lock().unwrap().push(Arc::new(directory));
        }
    }
}

/// One record's cell, as a handle: what a record's `state` field was.
#[derive(Clone, Copy)]
pub struct CellRef<'a> {
    cells: &'a Cells,
    id: u32,
}

impl<'a> CellRef<'a> {
    #[inline]
    pub fn get(&self) -> crate::symbols::Completion {
        self.cells.get(self.id)
    }
    pub fn set(&self, c: crate::symbols::Completion) {
        self.cells.set(self.id, c)
    }
    /// `Done` for a record just made (`Cells::done_fresh`).
    #[inline]
    pub fn done_fresh(&self) {
        self.cells.done_fresh(self.id)
    }
    #[inline]
    pub fn mine(&self) -> bool {
        self.cells.mine(self.id)
    }
    #[inline]
    pub fn owner(&self) -> Option<usize> {
        self.cells.owner(self.id)
    }
}

impl<'a> PartialEq<crate::symbols::Completion> for CellRef<'a> {
    #[inline]
    fn eq(&self, other: &crate::symbols::Completion) -> bool {
        self.get() == *other
    }
}

/// A register of the program's top-level definitions (`Program::top_funs`, `top_vals`): what
/// was registered before the fork and what a cell registers under the loader's lock is every
/// worker's, appended to a shared list that the interpreters read by id; what a worker's bodies
/// register is its own, merged after. Before the fork and after the
/// merge the register is one list, read as a slice; while it is shared, the two lists are read apart
/// (`shared_len`, `shared_get`, `own`).
pub struct Registers<T: Copy> {
    shared: Option<Arc<SlabVec<T>>>,
    own: Vec<T>,
    /// Whether pushes go to the shared list: the loader's lock holder's.
    pub alloc_shared: bool,
}

impl<T: Copy + Send + Sync> Registers<T> {
    pub fn new() -> Registers<T> {
        Registers { shared: None, own: Vec::new(), alloc_shared: false }
    }

    pub fn push(&mut self, v: T) {
        if self.alloc_shared {
            debug_assert!(lock_depth() > 0, "a shared registration outside the loader's lock");
            self.shared.as_ref().expect("no shared register").push(v);
        } else {
            self.own.push(v);
        }
    }

    #[inline]
    pub fn shared_len(&self) -> usize {
        self.shared.as_ref().map_or(0, |s| s.len())
    }

    #[inline]
    pub fn shared_get(&self, i: usize) -> T {
        *self.shared.as_ref().expect("no shared register").get(i)
    }

    pub fn own(&self) -> &[T] {
        &self.own
    }
    /// Every registration, the shared list's first: for a reader that must see the
    /// prefix's while the register is shared.
    pub fn all(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.shared_len()).map(move |i| self.shared_get(i)).chain(self.own.iter().copied())
    }

    pub fn own_mut(&mut self) -> &mut Vec<T> {
        &mut self.own
    }

    pub fn is_shared(&self) -> bool {
        self.shared.is_some()
    }

    pub fn fork(&mut self) -> Arc<SlabVec<T>> {
        assert!(self.shared.is_none());
        let shared = Arc::new(SlabVec::from_vec(std::mem::take(&mut self.own)));
        self.shared = Some(shared.clone());
        shared
    }

    /// Another worker's register over the same shared list, with nothing of its own yet.
    pub fn attach(&self) -> Registers<T> {
        Registers { shared: self.shared.clone(), own: Vec::new(), alloc_shared: false }
    }

    pub fn take_own(&mut self) -> Vec<T> {
        std::mem::take(&mut self.own)
    }

    /// Takes over another worker's own registrations, after the merge remapped them.
    pub fn absorb(&mut self, mut theirs: Vec<T>) {
        self.own.append(&mut theirs);
    }

    /// Leaves the shared list: one list again, the shared registrations first.
    pub fn merge_own(&mut self) {
        let Some(shared) = self.shared.take() else { return };
        let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("the register is still shared"));
        let mut all = shared.into_vec();
        all.append(&mut self.own);
        self.own = all;
    }

    pub fn held(&self) -> usize {
        crate::held::array(&self.own) + self.shared.as_ref().map_or(0, |s| s.held())
    }
}

impl<T: Copy + Send + Sync> Default for Registers<T> {
    fn default() -> Registers<T> {
        Registers::new()
    }
}

/// The one list, before the fork and after the merge.
impl<T: Copy> std::ops::Deref for Registers<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        debug_assert!(self.shared.is_none(), "a shared register read as one list");
        &self.own
    }
}

impl<T: Copy> std::ops::DerefMut for Registers<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        debug_assert!(self.shared.is_none(), "a shared register read as one list");
        &mut self.own
    }
}

impl<'a, T: Copy> IntoIterator for &'a Registers<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> std::slice::Iter<'a, T> {
        (**self).iter()
    }
}

pub struct SharedArena<T> {
    records: SlabVec<T>,
    /// The slots of the records there were at the fork, one each: never moved, so that a read
    /// of such a record tests its slot inline (`Arena::get`).
    prefix_overrides: Box<[AtomicPtr<T>]>,
    /// One slot per record pushed since the fork, at its index less `fork_len`, pushed before the
    /// record, so that a record always has its slot.
    overrides: SlabVec<AtomicPtr<T>>,
    /// The versions a published copy replaced, freed with the arena: a reader may still hold one.
    superseded: Mutex<Vec<*mut T>>,
    /// How many records are anyone's: the lock holder's pushes past it are staged, its own to
    /// fill until the lock is released, and a reader walking the arena by length stops here.
    published: AtomicUsize,
    /// Every worker's own records, by worker: what another worker reads of a record whose id
    /// reached it. Each is its worker's arena's store, which
    /// lives as long as the body phase; the stores are read while the workers run and never
    /// after the merge took their records.
    peers: Box<[AtomicPtr<SlabVec<T>>]>,
    /// What of each worker's own records has escaped it and what a peer has read
    /// (`EscapeMarks`), shared with the tables parallel to the arena (`Parallel::follow_escapes`).
    #[cfg(debug_assertions)]
    marks: Arc<EscapeMarks>,
    /// How many records there were at the fork: one below it that no copy replaced is the
    /// version the fork left, whose types every worker's view holds.
    fork_len: usize,
}

unsafe impl<T: Send + Sync> Sync for SharedArena<T> {}
unsafe impl<T: Send> Send for SharedArena<T> {}

/// The assertion-enabled builds' marks of a forked arena's records, which the tables parallel to
/// it share: a peer's read of a record or of its entry
/// in such a table is refused before the record escaped its owner and marks it read, and the
/// owner's write of the record or of its entry is refused once it is marked.
#[cfg(debug_assertions)]
pub struct EscapeMarks {
    /// Per worker, how many of its own records may have escaped it: every record it made before
    /// the release of its last hold of the loader's lock, the one point where what it publishes
    /// (a signature, a body, a class's `TClass`) names its records to every worker. A peer reads
    /// one only below the mark, read with acquire: the check that a peer's read acquired the
    /// entry point that gave it the id.
    escaped: Box<[AtomicUsize]>,
    /// The workers' own records a peer has read, by id: their owner never writes one after
    /// (`Arena::check_unescaped`), so a record has one version once it escaped, which the peers'
    /// views keep what they read by.
    peer_read: SharedBits,
    /// The workers' own records a publication's roots reach, by id: set as the publication's
    /// entry is stored, never written by their owner
    /// after, whether or not a peer has read them.
    sealed: SharedBits,
    /// Whether a peer reads a record of this kind only once it is sealed: the program's arenas',
    /// whose records a peer reaches from a publication's roots alone; the symbols' records a
    /// published type may name are read once they escaped.
    peers_read_sealed: std::sync::atomic::AtomicBool,
}

#[cfg(debug_assertions)]
impl EscapeMarks {
    fn new() -> EscapeMarks {
        EscapeMarks {
            escaped: (0..MAX_WORKERS).map(|_| AtomicUsize::new(0)).collect(),
            peer_read: SharedBits::new(),
            sealed: SharedBits::new(),
            peers_read_sealed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// A peer's read of worker `k`'s record `i` (at `j` of its own), or of its entry in a table
    /// parallel to the arena: refused before the record can have escaped, marked read after.
    #[track_caller]
    fn check_escaped(&self, k: usize, j: usize, i: u32, what: &str) {
        let escaped = self.escaped[k].load(Ordering::Acquire);
        assert!(j < escaped, "worker {}'s record {} ({}) read by a peer before it escaped (its first {} have): an id that reached the reader without a publication", k, i, what, escaped);
        if self.peers_read_sealed.load(Ordering::Relaxed) {
            assert!(self.sealed.has(i), "worker {}'s record {} ({}) read by a peer before its publication sealed it: a part of a publication not yet whole", k, i, what);
            crate::typer::bundle::count(crate::typer::bundle::Read::Direct);
        }
        self.peer_read.set(i);
    }

    /// The owner's write of its own record `i` or of its entry in a table parallel to the arena:
    /// refused once a publication sealed it or a peer read it.
    #[track_caller]
    fn check_writable(&self, worker: Option<usize>, i: u32, what: &str) {
        assert!(!self.peer_read.has(i), "worker {:?} changed its record {} ({}) after a peer read it: a published record is never changed before the merge", worker, i, what);
        assert!(!self.sealed.has(i), "worker {:?} changed its record {} ({}) after a publication sealed it: a published record is never changed before the merge", worker, i, what);
    }
}

impl<T> SharedArena<T> {
    fn from_vec(v: Vec<T>) -> SharedArena<T> {
        let n = v.len();
        let prefix_overrides = (0..n).map(|_| AtomicPtr::new(std::ptr::null_mut())).collect();
        let overrides = SlabVec::with_capacity(16);
        let peers = (0..MAX_WORKERS).map(|_| AtomicPtr::new(std::ptr::null_mut())).collect();
        SharedArena {
            records: SlabVec::from_vec(v),
            prefix_overrides,
            overrides,
            superseded: Mutex::new(Vec::new()),
            published: AtomicUsize::new(n),
            peers,
            #[cfg(debug_assertions)]
            marks: Arc::new(EscapeMarks::new()),
            fork_len: n,
        }
    }

    /// How many records there are, the lock holder's staged ones included.
    #[inline]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// How many records every worker may read.
    #[inline]
    pub fn published(&self) -> usize {
        self.published.load(Ordering::Acquire)
    }

    /// Whether the record `i` may be read by a thread that does not hold the loader's lock:
    /// published, or staged by the lock holder, whose release publishes it. A staged record
    /// whose id reached another thread (through a map the holder filled, a subclass list, a
    /// cell) is waited for: the holder's hold never waits on anything,
    /// so it ends.
    #[cold]
    fn wait_published(&self, i: u32) -> bool {
        if (i as usize) >= self.records.len() {
            return false;
        }
        let start = std::time::Instant::now();
        let measured = crate::measure::wait_begin();
        while (i as usize) >= self.published() {
            std::thread::yield_now();
            assert!(start.elapsed().as_secs() < 120, "record {} stayed unpublished for two minutes", i);
        }
        crate::measure::wait_end(measured, crate::measure::Wait::Published, 0);
        true
    }

    /// Worker `k`'s own store, where the worker registered it.
    fn peer(&self, k: usize) -> Option<&SlabVec<T>> {
        let p = self.peers.get(k)?.load(Ordering::Acquire);
        (!p.is_null()).then(|| unsafe { &*p })
    }

    /// Another worker's record `i`, published to this thread by whatever gave it the id (a cell
    /// done, a map entry applied at a lock's release): its maker has pushed it before then.
    #[cold]
    #[inline(never)]
    fn peer_get(&self, i: u32) -> Option<&T> {
        let k = worker_of(i)?;
        let store = self.peer(k)?;
        let j = (i - worker_base(k)) as usize;
        #[cfg(debug_assertions)]
        if j < store.len() {
            self.check_escaped(k, j, i);
        }
        (j < store.len()).then(|| store.get(j))
    }

    #[cfg(debug_assertions)]
    fn check_escaped(&self, k: usize, j: usize, i: u32) {
        self.marks.check_escaped(k, j, i, std::any::type_name::<T>());
    }

    #[cold]
    #[inline(never)]
    fn peer_slice(&self, start: u32, len: u32) -> &[T] {
        let k = worker_of(start).expect("a worker's list");
        let store = self.peer(k).unwrap_or_else(|| panic!("worker {}'s records read before the worker began", k));
        let j = (start - worker_base(k)) as usize;
        #[cfg(debug_assertions)]
        for x in 0..len {
            self.check_escaped(k, j + x as usize, start + x);
        }
        &store.as_slice()[j..j + len as usize]
    }

    #[inline]
    fn readable(&self, i: u32) -> bool {
        (i as usize) < self.published() || (lock_depth() > 0 && (i as usize) < self.records.len()) || self.wait_published(i)
    }

    fn publish_all(&self) {
        self.published.store(self.records.len(), Ordering::Release);
    }

    /// The slot of the record `i`'s published copy.
    #[inline]
    fn slot(&self, i: u32) -> &AtomicPtr<T> {
        match self.prefix_overrides.get(i as usize) {
            Some(slot) => slot,
            None => self.overrides.get(i as usize - self.fork_len),
        }
    }

    #[inline]
    pub fn get(&self, i: u32) -> &T {
        let p = self.slot(i).load(Ordering::Acquire);
        if p.is_null() {
            self.records.get(i as usize)
        } else {
            unsafe { &*p }
        }
    }

    /// The record `i` as a worker's view reads it (`RecordView`): the version the fork left as it
    /// is, a later one through the view.
    #[inline]
    fn get_viewed<'a>(&'a self, i: u32, view: &'a RecordView<T>) -> &'a T {
        let p = self.slot(i).load(Ordering::Acquire);
        if !p.is_null() {
            return view.read(i, unsafe { &*p });
        }
        let r = self.records.get(i as usize);
        if (i as usize) < self.fork_len { r } else { view.read(i, r) }
    }

    /// Publishes `copy` as the record `i`: the loader's lock holder's.
    fn publish(&self, i: u32, copy: Box<T>)
    where
        T: Record,
    {
        let old = self.slot(i).swap(Box::into_raw(copy), Ordering::AcqRel);
        if !old.is_null() {
            self.superseded.lock().unwrap_or_else(|e| e.into_inner()).push(old);
        }
    }

    /// The records with their published copies in place, for the single-threaded phases.
    pub fn into_vec(self) -> Vec<T> {
        let (records, replaced, superseded) = self.into_parts();
        drop((replaced, superseded));
        records
    }

    /// `into_vec`, with what it leaves apart: the versions the published copies replaced (each in
    /// its copy's box, the two swapped) and the ones they superseded before, for a caller that
    /// frees them elsewhere (`Arena::retained`).
    pub fn into_parts(self) -> (Vec<T>, Vec<Box<T>>, Vec<Box<T>>) {
        let this = std::mem::ManuallyDrop::new(self);
        let (records, prefix, overrides, superseded) = unsafe { (std::ptr::read(&this.records), std::ptr::read(&this.prefix_overrides), std::ptr::read(&this.overrides), std::ptr::read(&this.superseded)) };
        // The rest of the arena goes here, `ManuallyDrop` dropping none of it: a resident session
        // lost the peers' table of every arena at every forked build.
        drop(unsafe { std::ptr::read(&this.peers) });
        #[cfg(debug_assertions)]
        drop(unsafe { std::ptr::read(&this.marks) });
        let mut records = records.into_vec();
        let mut replaced = Vec::new();
        for (i, o) in prefix.into_vec().into_iter().chain(overrides.into_vec()).enumerate() {
            let p = o.into_inner();
            if !p.is_null() {
                let mut copy = unsafe { Box::from_raw(p) };
                std::mem::swap(&mut records[i], &mut *copy);
                replaced.push(copy);
            }
        }
        let superseded = superseded.into_inner().unwrap_or_else(|e| e.into_inner()).into_iter().map(|p| unsafe { Box::from_raw(p) }).collect();
        (records, replaced, superseded)
    }
}

impl<T> Drop for SharedArena<T> {
    fn drop(&mut self) {
        for o in self.prefix_overrides.iter().chain(self.overrides.as_slice()) {
            let p = o.load(Ordering::Relaxed);
            if !p.is_null() {
                drop(unsafe { Box::from_raw(p) });
            }
        }
        for p in self.superseded.get_mut().unwrap_or_else(|e| e.into_inner()).drain(..) {
            drop(unsafe { Box::from_raw(p) });
        }
    }
}

pub struct Arena<T> {
    /// Every record while the arena is one worker's, read by its id with nothing to subtract;
    /// empty while it is forked. A read checks the id against this vector's length first, the
    /// one bounds check of a vector.
    plain: Vec<T>,
    /// A forked worker's own records, the first of them at `own_base`: a store that keeps a
    /// record where it is while another worker reads it by id through the shared region's
    /// peers. Empty and unregistered with one worker.
    own: Box<SlabVec<T>>,
    own_base: u32,
    shared: Option<Arc<SharedArena<T>>>,
    /// How many records the shared region had at the fork, none while the arena is one
    /// worker's: such a record no copy replaced is read inline (`get`).
    prefix_len: u32,
    /// The copies of published shared records this worker is changing under the loader's lock.
    /// The working copies, each with the order it was made in: the last made is published
    /// first (`flush`), since a completion finishes what it nests before itself and a reader
    /// that sees the outer record done must find the inner ones done too.
    working: FxMap<u32, (u32, Box<T>)>,
    made: u32,
    /// The first shared id pushed under the lock this worker holds, `u32::MAX` outside it.
    staged_from: u32,
    /// Whether pushes go to the shared region: the loader's lock holder's, for what every
    /// worker reads by id.
    pub alloc_shared: bool,
    /// Told of every read of another worker's record (the type store's overlays' measurement,
    /// `types::note_peer_class`).
    pub peer_reads: Option<fn(u32, &T)>,
    /// The worker's view of the shared records and of its peers' (`RecordView`), while its
    /// worker types with the type store's overlays on.
    view: Option<Box<RecordView<T>>>,
}

/// A worker's view of the shared records of one kind: every read of a published record by
/// the worker outside
/// the loader's lock hands over the record with its types in the worker's view, `in_view`'s
/// copy where one of them moves and the record itself where none does. What is kept per record
/// is its publication token, the address of the version read (a published record is immutable
/// and a new version replaces it at another address, `SharedArena::publish`, the old kept until
/// the arena goes, so an address names one version for the body phase), and the copy made for
/// it: the cache is tested before any traversal, and a record's types are translated once per
/// version. The holder's reads under the lock of the shared records (its working and staged
/// records, whose view is the base) and the worker's own records are not viewed.
///
/// A peer's record, another worker's own, is read through the view by every reader, a worker
/// outside the lock (its types imported, `in_view`) and the holder under it (exported into the
/// base, `in_base`), each keeping what it read by the id in a sparse table of its own: a peer's
/// record has one version once it escapes its owner, which never writes it after
/// (`SharedArena::escaped`, checked in the assertion-enabled builds), so the id names it,
/// whatever address a growth of the owner's store gives it; no table is dense over the workers'
/// ranges.
struct RecordView<T> {
    in_view: fn(&T) -> Option<T>,
    in_base: fn(&T) -> Option<T>,
    /// By the shared id: the token of the version last read and what a read of it hands over,
    /// the copy or the record itself.
    slots: std::cell::UnsafeCell<Vec<(*const T, *const T)>>,
    /// Copies of versions read before, freed at the next point no reference into them lives
    /// (`Arena::sweep_view`).
    retired: std::cell::UnsafeCell<Vec<*mut T>>,
    /// The versions translated: the cache's misses.
    translated: std::cell::Cell<u64>,
    /// A peer's records read, by id, what a read hands over and whether it is a copy the view
    /// made (or the record itself): the worker's outside the lock, then the holder's under it.
    peers: [std::cell::UnsafeCell<FxMap<u32, (*const T, bool)>>; 2],
    /// The worker's own records the loader's lock holder read in the hold under way, by id, what
    /// a read hands over: kept until a write of the record or the hold's end (`read_own_held`).
    own_held: std::cell::UnsafeCell<FxMap<u32, *const T>>,
    /// The reads of a peer's records, and of them the ones translated (the tables' misses).
    peer_reads: std::cell::Cell<u64>,
    peer_translated: std::cell::Cell<u64>,
    /// A copy's bytes, its record and what it holds on the heap.
    bytes: fn(&T) -> usize,
    /// The bytes the copies hold, current and retired, and the most the view held at once.
    held: std::cell::Cell<usize>,
    peak: std::cell::Cell<usize>,
}

/// How a view translates a record of its kind (`Arena::set_view`): into the worker's view, into
/// the base for the loader's lock holder, each `None` where every type of it is there already;
/// and a copy's bytes.
pub struct RecordViewFns<T> {
    pub in_view: fn(&T) -> Option<T>,
    pub in_base: fn(&T) -> Option<T>,
    pub bytes: fn(&T) -> usize,
}

/// What a worker's view of one kind of record holds (`Arena::view_counts`).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ViewCounts {
    /// The records whose version the view keeps, and of them the ones read as a copy.
    pub records: usize,
    pub copies: usize,
    /// The copies of versions read before, not yet freed.
    pub retired: usize,
    /// The versions translated (the cache's misses).
    pub translated: u64,
    /// The ids the table reaches (its length) and the bytes of its allocated capacity.
    pub reach: usize,
    pub capacity: usize,
    /// The bytes the copies hold, current and retired (their records and what those hold on the
    /// heap), and the most the view held at once, table and copies.
    pub payload: usize,
    pub peak: usize,
    /// A peer's records: the ones kept (both tables), of them the copies, the reads and the
    /// translations.
    pub peer_records: usize,
    pub peer_copies: usize,
    pub peer_reads: u64,
    pub peer_translated: u64,
}

// A view is its worker's, read on the worker's thread alone.
unsafe impl<T: Send> Send for RecordView<T> {}
unsafe impl<T: Sync> Sync for RecordView<T> {}

impl<T> RecordView<T> {
    #[inline]
    fn read<'a>(&'a self, i: u32, r: &'a T) -> &'a T {
        let slots = unsafe { &*self.slots.get() };
        match slots.get(i as usize) {
            Some(&(token, read)) if std::ptr::eq(token, r) => unsafe { &*read },
            _ => self.miss(i, r),
        }
    }

    #[cold]
    #[inline(never)]
    fn miss<'a>(&'a self, i: u32, r: &'a T) -> &'a T {
        self.translated.set(self.translated.get() + 1);
        let read = (self.in_view)(r).map_or(r as *const T, |c| {
            self.held.set(self.held.get() + (self.bytes)(&c));
            Box::into_raw(Box::new(c)) as *const T
        });
        let slots = unsafe { &mut *self.slots.get() };
        if slots.len() <= i as usize {
            slots.resize(i as usize + 1, (std::ptr::null(), std::ptr::null()));
        }
        let now = slots.capacity() * std::mem::size_of::<(*const T, *const T)>() + self.held.get();
        if now > self.peak.get() {
            self.peak.set(now);
        }
        let (token, old) = std::mem::replace(&mut slots[i as usize], (r as *const T, read));
        if !std::ptr::eq(old, token) {
            unsafe { &mut *self.retired.get() }.push(old as *mut T);
        }
        unsafe { &*read }
    }

    /// The worker's own record `i`, `r` as its store holds it, as the loader's lock holder reads
    /// it, the base its view: the record where each of its
    /// types is the base's, else a copy with its types exported. What a read handed over is kept
    /// until the holder writes the record (`forget_own_held`) or the hold ends, since the record
    /// may change within it (its owner is the holder); a copy is retired with the view's copies,
    /// freed at the next hold (`Arena::sweep_view`).
    #[cold]
    #[inline(never)]
    fn read_own_held<'a>(&'a self, i: u32, r: &'a T) -> &'a T {
        let table = unsafe { &mut *self.own_held.get() };
        if let Some(&read) = table.get(&i) {
            return unsafe { &*read };
        }
        let read = match (self.in_base)(r) {
            None => r as *const T,
            Some(c) => {
                #[cfg(debug_assertions)]
                crate::types::view::crossed(crate::types::view::Crossing::HolderOwn, 1);
                self.held.set(self.held.get() + (self.bytes)(&c));
                let copy = Box::into_raw(Box::new(c));
                unsafe { &mut *self.retired.get() }.push(copy);
                copy as *const T
            }
        };
        table.insert(i, read);
        unsafe { &*read }
    }

    /// The holder's own record `i` is written: what its reads handed over goes (`read_own_held`).
    #[inline]
    fn forget_own_held(&mut self, i: u32) {
        let table = self.own_held.get_mut();
        if !table.is_empty() {
            table.remove(&i);
        }
    }

    /// A peer's record `i`, `r` as its owner's store holds it, read by the worker outside the
    /// loader's lock or by the holder under it (`hold`): the table's entry, or the record
    /// translated into the reader's view once.
    #[cold]
    #[inline(never)]
    fn read_peer<'a>(&'a self, i: u32, r: &'a T, hold: bool) -> &'a T {
        self.peer_reads.set(self.peer_reads.get() + 1);
        let table = unsafe { &mut *self.peers[hold as usize].get() };
        if let Some(&(read, _)) = table.get(&i) {
            return unsafe { &*read };
        }
        self.peer_translated.set(self.peer_translated.get() + 1);
        let entry = (if hold { self.in_base } else { self.in_view })(r).map_or((r as *const T, false), |c| {
            self.held.set(self.held.get() + (self.bytes)(&c));
            (Box::into_raw(Box::new(c)) as *const T, true)
        });
        table.insert(i, entry);
        unsafe { &*entry.0 }
    }

    fn counts(&self) -> ViewCounts {
        let slots = unsafe { &*self.slots.get() };
        let records = slots.iter().filter(|(t, _)| !t.is_null()).count();
        let copies = slots.iter().filter(|&&(t, r)| !std::ptr::eq(t, r)).count();
        let retired = unsafe { &*self.retired.get() }.len();
        let peers = self.peers.iter().map(|t| unsafe { &*t.get() });
        let peer_capacity: usize = peers.clone().map(|t| t.capacity() * (std::mem::size_of::<(u32, (*const T, bool))>() + 1)).sum();
        let capacity = slots.capacity() * std::mem::size_of::<(*const T, *const T)>() + peer_capacity;
        let (peer_records, peer_copies) = peers.fold((0, 0), |(n, c), t| (n + t.len(), c + t.values().filter(|&&(_, copy)| copy).count()));
        ViewCounts {
            records,
            copies,
            retired,
            translated: self.translated.get(),
            reach: slots.len(),
            capacity,
            payload: self.held.get(),
            peak: self.peak.get().max(capacity + self.held.get()),
            peer_records,
            peer_copies,
            peer_reads: self.peer_reads.get(),
            peer_translated: self.peer_translated.get(),
        }
    }

}

impl<T> Drop for RecordView<T> {
    fn drop(&mut self) {
        for &(t, r) in self.slots.get_mut().iter() {
            if !std::ptr::eq(t, r) {
                drop(unsafe { Box::from_raw(r as *mut T) });
            }
        }
        for table in self.peers.iter_mut() {
            for &(r, copy) in table.get_mut().values() {
                if copy {
                    drop(unsafe { Box::from_raw(r as *mut T) });
                }
            }
        }
        for &c in self.retired.get_mut().iter() {
            drop(unsafe { Box::from_raw(c) });
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Arena<T> {
        Arena::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        Arena { plain: Vec::new(), own: Box::new(SlabVec::with_capacity(0)), own_base: 0, shared: None, prefix_len: 0, working: FxMap::default(), made: 0, staged_from: u32::MAX, alloc_shared: false, peer_reads: None, view: None }
    }

    pub fn with_capacity(n: usize) -> Arena<T> {
        Arena { plain: Vec::with_capacity(n), ..Arena::new() }
    }

    pub fn from_vec(v: Vec<T>) -> Arena<T> {
        Arena { plain: v, ..Arena::new() }
    }

    /// This worker's own records: every record while the arena is one worker's.
    #[inline]
    fn mine(&self) -> &[T] {
        if self.shared.is_none() { &self.plain } else { self.own.as_slice() }
    }

    #[inline]
    fn mine_mut(&mut self) -> &mut [T] {
        if self.shared.is_none() {
            &mut self.plain
        } else {
            self.forget_own_held_all();
            let n = self.own.len();
            unsafe { self.own.slice_mut(0, n) }
        }
    }

    /// The worker's own records are written as a slice: what the holder's reads of them handed
    /// over goes (`RecordView::read_own_held`).
    fn forget_own_held_all(&mut self) {
        if let Some(v) = &mut self.view {
            v.own_held.get_mut().clear();
        }
    }

    /// Whether the arena has a shared region: the body phase of several workers.
    #[inline]
    pub fn is_shared(&self) -> bool {
        self.shared.is_some()
    }

    #[inline]
    pub fn shared(&self) -> &SharedArena<T> {
        self.shared.as_ref().expect("the arena has no shared region")
    }

    #[inline]
    pub fn get(&self, i: u32) -> &T {
        if (i as usize) < self.plain.len() {
            return unsafe { self.plain.get_unchecked(i as usize) };
        }
        self.forked_get(i)
    }

    /// A read of a forked arena: the worker's own record, or a shared one, or another
    /// worker's. The loader's lock holder reads its own worker's records through the view's
    /// export (`RecordView::read_own_held`).
    #[inline(never)]
    fn forked_get(&self, i: u32) -> &T {
        if i >= LOCAL_BASE {
            let j = i.wrapping_sub(self.own_base) as usize;
            if j < self.own.len() {
                let r = self.own.get(j);
                if self.staged_from != u32::MAX {
                    if let Some(v) = &self.view {
                        return v.read_own_held(i, r);
                    }
                }
                return r;
            }
        } else if self.working.is_empty() {
            let read = if i < self.prefix_len { Some(self.prefix_get(i)) } else { self.later_get(i) };
            if let Some(read) = read {
                #[cfg(debug_assertions)]
                self.check_published_read(i, read);
                return read.0;
            }
        }
        self.shared_get(i)
    }

    /// A record there was at the fork, with no working copy of the worker's (`shared_get`'s
    /// answer): the version the fork left, which no view translates, unless a copy replaced it;
    /// the worker's view of the copy outside the loader's lock, the copy itself under it. With
    /// the copy's slot as it was read and the record's address, for the check.
    #[inline]
    fn prefix_get(&self, i: u32) -> (&T, *mut T, *const T) {
        let s = unsafe { self.shared.as_ref().unwrap_unchecked() };
        let p = unsafe { s.prefix_overrides.get_unchecked(i as usize) }.load(Ordering::Acquire);
        if p.is_null() {
            let r = s.records.get(i as usize);
            return (r, p, r);
        }
        (self.viewed(i, unsafe { &*p }), p, p)
    }

    /// A record of the shared region made since the fork and published, with no working copy of
    /// the worker's (`shared_get`'s answer): the record or the copy that replaced it, through the
    /// worker's view outside the loader's lock; `None` for one not yet published.
    #[inline]
    fn later_get(&self, i: u32) -> Option<(&T, *mut T, *const T)> {
        // An arena of one worker's comes here only for an id it does not have: the panic's path.
        let s = self.shared.as_ref()?;
        if (i as usize) >= s.published() {
            return None;
        }
        let p = s.overrides.get(i as usize - s.fork_len).load(Ordering::Acquire);
        let record: &T = if p.is_null() { s.records.get(i as usize) } else { unsafe { &*p } };
        Some((self.viewed(i, record), p, record))
    }

    /// A published record or copy as the worker reads it: through its view outside the loader's
    /// lock, as it is under it.
    #[inline]
    fn viewed<'a>(&'a self, i: u32, record: &'a T) -> &'a T {
        match &self.view {
            Some(v) if self.staged_from == u32::MAX => v.read(i, record),
            _ => record,
        }
    }

    /// The assertion-enabled builds' check of `published_get`: its answer is the shared region's
    /// read's, unless a copy was published between the two or the region's records moved to a
    /// larger buffer (the lock holder's pushes), whose copy of the record is the same version.
    #[cfg(debug_assertions)]
    fn check_published_read(&self, i: u32, (r, copy, record): (&T, *mut T, *const T)) {
        if std::ptr::eq(r, self.shared_get(i)) {
            return;
        }
        let s = self.shared.as_ref().expect("a forked arena");
        let published = s.slot(i).load(Ordering::Acquire) != copy;
        let moved = copy.is_null() && !std::ptr::eq(s.records.get(i as usize), record);
        assert!(published || moved, "record {} read as a published one otherwise than by the shared region's read", i);
    }

    #[cold]
    #[inline(never)]
    fn shared_get(&self, i: u32) -> &T {
        if !self.working.is_empty() {
            if let Some((_, w)) = self.working.get(&i) {
                return w;
            }
        }
        match &self.shared {
            Some(s) if i >= LOCAL_BASE => {
                let r = s.peer_get(i).unwrap_or_else(|| panic!("no record {} of worker {:?} readable by worker {:?}", i, worker_of(i), worker_of(self.own_base)));
                if let Some(note) = self.peer_reads {
                    note(i, r);
                }
                match &self.view {
                    Some(v) => v.read_peer(i, r, self.staged_from != u32::MAX),
                    None => r,
                }
            }
            Some(s) if s.readable(i) => match &self.view {
                // Outside the loader's lock: the worker's view.
                Some(v) if self.staged_from == u32::MAX => s.get_viewed(i, v),
                _ => s.get(i),
            },
            _ => panic!("no record {} in an arena of {} own records from {}", i, self.mine().len(), self.own_base),
        }
    }

    /// The record `i` as it is published, whoever reads it: the raw traversal of a shared
    /// record's types by design, or of its ids alone (a metadata-only read), which no view
    /// translates.
    #[inline]
    pub fn raw(&self, i: u32) -> &T {
        if (i as usize) < self.plain.len() {
            return unsafe { self.plain.get_unchecked(i as usize) };
        }
        self.forked_raw(i)
    }

    #[inline(never)]
    fn forked_raw(&self, i: u32) -> &T {
        if i < self.prefix_len && self.working.is_empty() {
            return unsafe { self.shared.as_ref().unwrap_unchecked() }.get(i);
        }
        let j = i.wrapping_sub(self.own_base) as usize;
        if j < self.own.len() {
            return self.own.get(j);
        }
        if let Some((_, w)) = self.working.get(&i) {
            return w;
        }
        match &self.shared {
            Some(s) if i >= LOCAL_BASE => s.peer_get(i).unwrap_or_else(|| panic!("no record {} of worker {:?}", i, worker_of(i))),
            Some(s) if s.readable(i) => s.get(i),
            _ => panic!("no record {} in an arena of {} own records from {}", i, self.mine().len(), self.own_base),
        }
    }

    /// Whether this worker holds the loader's lock: its pushes are staged from `staged_from`.
    #[inline]
    pub fn holding(&self) -> bool {
        self.staged_from != u32::MAX
    }

    /// The records the loader's lock holder is about to publish: the staged ones and the working
    /// copies, which the release makes every worker's.
    pub fn unpublished(&self) -> impl Iterator<Item = &T> {
        let staged = match &self.shared {
            Some(s) if self.staged_from != u32::MAX => self.staged_from as usize..s.len(),
            _ => 0..0,
        };
        staged.map(move |i| self.shared().records.get(i)).chain(self.working.values().map(|(_, w)| &**w))
    }

    /// Views the shared records and the peers' from here (`RecordView`), or no more: `in_view`
    /// the worker's translation, `in_base` the loader's lock holder's, `bytes` a copy's size.
    pub fn set_view(&mut self, view: Option<RecordViewFns<T>>) {
        self.view = view.map(|f| {
            Box::new(RecordView {
                in_view: f.in_view,
                in_base: f.in_base,
                slots: Default::default(),
                retired: Default::default(),
                translated: Default::default(),
                peers: Default::default(),
                own_held: Default::default(),
                peer_reads: Default::default(),
                peer_translated: Default::default(),
                bytes: f.bytes,
                held: Default::default(),
                peak: Default::default(),
            })
        });
    }

    /// Frees the copies of the versions read before the last: no reference into them lives
    /// while the arena is borrowed for a change.
    pub fn sweep_view(&mut self) {
        if let Some(v) = &mut self.view {
            v.own_held.get_mut().clear();
            for c in v.retired.get_mut().drain(..) {
                let c = unsafe { Box::from_raw(c) };
                v.held.set(v.held.get() - (v.bytes)(&c));
            }
        }
    }

    /// What the view holds and did (`ViewCounts`).
    pub fn view_counts(&self) -> ViewCounts {
        self.view.as_ref().map_or_else(ViewCounts::default, |v| v.counts())
    }

    /// The record as every worker sees it: never this worker's working copy. A cell is
    /// claimed here, so that one claim stands for every worker.
    pub fn canonical(&self, i: u32) -> &T {
        if (i as usize) < self.plain.len() {
            return unsafe { self.plain.get_unchecked(i as usize) };
        }
        let j = i.wrapping_sub(self.own_base) as usize;
        if j < self.own.len() {
            return self.own.get(j);
        }
        match &self.shared {
            Some(s) if i >= LOCAL_BASE => s.peer_get(i).unwrap_or_else(|| panic!("no record {} of worker {:?}", i, worker_of(i))),
            Some(s) if s.readable(i) => s.get(i),
            _ => panic!("no record {} in an arena of {} own records from {}", i, self.mine().len(), self.own_base),
        }
    }

    /// The record for a change: a local one, or a shared one this worker holds the loader's
    /// lock for (staged, or a working copy published when the lock is released).
    #[inline]
    pub fn get_mut(&mut self, i: u32) -> &mut T
    where
        T: Record,
    {
        if (i as usize) < self.plain.len() {
            return unsafe { self.plain.get_unchecked_mut(i as usize) };
        }
        self.forked_get_mut(i)
    }

    #[inline(never)]
    fn forked_get_mut(&mut self, i: u32) -> &mut T
    where
        T: Record,
    {
        let j = i.wrapping_sub(self.own_base) as usize;
        if j < self.own.len() {
            #[cfg(debug_assertions)]
            self.check_unescaped(j, i);
            if let Some(v) = &mut self.view {
                v.forget_own_held(i);
            }
            return unsafe { self.own.get_mut(j) };
        }
        self.shared_get_mut(i)
    }

    /// The owner's write of its own record `i` (at `j` of its own), refused once a peer has
    /// read it: a published record has one version until the merge, which the peers' views keep
    /// what they read by.
    #[cfg(debug_assertions)]
    #[track_caller]
    fn check_unescaped(&self, _j: usize, i: u32) {
        if let Some(s) = &self.shared {
            s.marks.check_writable(worker_of(self.own_base), i, std::any::type_name::<T>());
        }
    }

    /// Seals this worker's own record `i`, a publication's roots reaching it: whether it was not
    /// sealed before, which a walk stops at. Any other record is not this worker's to seal.
    #[cfg(debug_assertions)]
    pub fn seal(&self, i: u32) -> bool {
        let j = i.wrapping_sub(self.own_base) as usize;
        match &self.shared {
            Some(s) if j < self.own.len() => {
                if s.marks.sealed.has(i) {
                    return false;
                }
                s.marks.sealed.set(i);
                true
            }
            _ => false,
        }
    }

    /// Whether this worker's own record `i` is sealed (`seal`).
    #[cfg(debug_assertions)]
    pub fn sealed(&self, i: u32) -> bool {
        self.shared.as_ref().map_or(false, |s| s.marks.sealed.has(i))
    }

    /// Whether `i` is another worker's own record, which this worker reads through its peer.
    #[inline]
    pub fn is_peer(&self, i: u32) -> bool {
        self.shared.is_some() && i >= LOCAL_BASE && worker_of(i) != worker_of(self.own_base)
    }

    /// Whether `i` is a record of this worker's own.
    #[inline]
    pub fn is_own(&self, i: u32) -> bool {
        self.shared.is_some() && (i.wrapping_sub(self.own_base) as usize) < self.own.len()
    }

    /// A peer reads a record of this kind only once a publication sealed it (`EscapeMarks`).
    #[cfg(debug_assertions)]
    pub fn peers_read_sealed(&self) {
        if let Some(s) = &self.shared {
            s.marks.peers_read_sealed.store(true, Ordering::Relaxed);
        }
    }

    /// The owner's writes through a mutable view of all its own records (`iter_mut`), each
    /// refused once a peer has read it.
    #[cfg(debug_assertions)]
    #[track_caller]
    fn check_unescaped_all(&self) {
        if self.shared.is_some() {
            for j in 0..self.own.len() {
                self.check_unescaped(j, self.own_base + j as u32);
            }
        }
    }

    /// The owner's write of its own record `i`, refused once it has escaped (the assertion-
    /// enabled builds'; `check_unescaped`): for a table parallel to the arena, whose entries
    /// are the record's.
    #[cfg(debug_assertions)]
    #[track_caller]
    pub fn check_writable_own(&self, i: u32) {
        let j = i.wrapping_sub(self.own_base) as usize;
        if self.shared.is_some() && j < self.own.len() {
            self.check_unescaped(j, i);
        }
    }

    /// Every record this worker made so far has escaped it: the release of its outermost hold of
    /// the loader's lock, where what it published names them (the assertion-enabled builds').
    #[cfg(debug_assertions)]
    pub fn escape_own(&self) {
        if let (Some(s), Some(k)) = (&self.shared, worker_of(self.own_base)) {
            s.marks.escaped[k].store(self.own.len(), Ordering::Release);
        }
    }

    /// The marks of what of the workers' own records escaped and what a peer read, for a table
    /// parallel to the arena (`Parallel::follow_escapes`).
    #[cfg(debug_assertions)]
    pub fn escape_marks(&self) -> Option<Arc<EscapeMarks>> {
        self.shared.as_ref().map(|s| s.marks.clone())
    }

    #[cold]
    #[inline(never)]
    fn shared_get_mut(&mut self, i: u32) -> &mut T
    where
        T: Record,
    {
        let s = self.shared.as_ref().expect("no such record");
        assert!(i < LOCAL_BASE, "record {} of worker {:?} changed by worker {:?}: another worker's records are read, never changed", i, worker_of(i), worker_of(self.own_base));
        assert!(lock_depth() > 0, "a shared record ({}) changed outside the loader's lock", i);
        if i >= self.staged_from {
            return unsafe { s.records.get_mut(i as usize) };
        }
        let current = s.get(i);
        let made = &mut self.made;
        &mut self
            .working
            .entry(i)
            .or_insert_with(|| {
                *made += 1;
                (*made, Box::new(current.clone()))
            })
            .1
    }

    #[inline]
    pub fn push(&mut self, v: T) -> u32 {
        if self.shared.is_none() {
            self.plain.push(v);
            return (self.plain.len() - 1) as u32;
        }
        self.push_forked(v)
    }

    /// A push into a forked arena: the worker's own record, or a shared one under the lock.
    #[inline(never)]
    fn push_forked(&mut self, v: T) -> u32 {
        if self.alloc_shared {
            return self.push_shared(v);
        }
        let j = self.own.push(v);
        if j + 1 >= WORKER_SPAN as usize {
            Self::span_exceeded();
        }
        self.own_base + j as u32
    }

    /// A push into the shared region: the loader's lock holder's.
    #[cold]
    #[inline(never)]
    fn push_shared(&mut self, v: T) -> u32 {
        let s = self.shared.as_ref().expect("no shared region to allocate in");
        assert!(lock_depth() > 0, "a shared record pushed outside the loader's lock");
        s.overrides.push(AtomicPtr::new(std::ptr::null_mut()));
        let i = s.records.push(v);
        debug_assert!(i as u32 >= self.staged_from);
        i as u32
    }

    #[cold]
    #[inline(never)]
    fn span_exceeded() -> ! {
        panic!("a worker made more than {} records of one kind", WORKER_SPAN)
    }

    /// Appends the items as one list: its start id and length.
    pub fn push_slice(&mut self, items: &[T]) -> crate::ast::ListRef
    where
        T: Copy,
    {
        if items.is_empty() {
            return crate::ast::ListRef::EMPTY;
        }
        if self.shared.is_none() {
            let start = self.plain.len() as u32;
            self.plain.extend_from_slice(items);
            return crate::ast::ListRef { start, len: items.len() as u32 };
        }
        self.forked_push_slice(items)
    }

    #[inline(never)]
    fn forked_push_slice(&mut self, items: &[T]) -> crate::ast::ListRef
    where
        T: Copy,
    {
        if self.alloc_shared {
            let start = self.push(items[0]);
            for &x in &items[1..] {
                self.push(x);
            }
            return crate::ast::ListRef { start, len: items.len() as u32 };
        }
        let start = self.own_base + self.own.len() as u32;
        for &x in items {
            self.own.push(x);
        }
        crate::ast::ListRef { start, len: items.len() as u32 }
    }

    /// The `len` records from `start`, one list: a list lies in one region.
    #[inline]
    pub fn slice(&self, start: u32, len: u32) -> &[T] {
        if let Some(records) = self.plain.get(start as usize..(start + len) as usize) {
            return records;
        }
        self.forked_slice(start, len)
    }

    #[inline(never)]
    fn forked_slice(&self, start: u32, len: u32) -> &[T] {
        if len == 0 {
            return &[];
        }
        let j = start.wrapping_sub(self.own_base) as usize;
        if j < self.own.len() {
            return &self.own.as_slice()[j..j + len as usize];
        }
        let s = self.shared.as_ref().expect("no such list");
        if start >= LOCAL_BASE {
            return s.peer_slice(start, len);
        }
        &s.records.as_slice()[start as usize..(start + len) as usize]
    }

    /// The record `i` when there is one this worker may read.
    pub fn try_get(&self, i: u32) -> Option<&T> {
        if (i as usize) < self.plain.len() {
            return Some(&self.plain[i as usize]);
        }
        let j = i.wrapping_sub(self.own_base) as usize;
        if j < self.own.len() {
            return Some(self.own.get(j));
        }
        let s = self.shared.as_ref()?;
        if i >= LOCAL_BASE {
            return if worker_of(i) == worker_of(self.own_base) { None } else { s.peer_get(i) };
        }
        s.readable(i).then(|| self.shared_get(i))
    }

    /// Every record this worker may read, with its id: the shared region's published ones,
    /// then the worker's own.
    pub fn entries(&self) -> impl Iterator<Item = (u32, &T)> {
        self.entries_since((0, 0))
    }

    /// How far a scan of the entries got: the shared and the own count, for `entries_since`.
    pub fn marks(&self) -> (usize, usize) {
        (self.shared_len(), self.mine().len())
    }

    /// The entries added since `marks`, with their ids.
    pub fn entries_since(&self, (shared_seen, own_seen): (usize, usize)) -> impl Iterator<Item = (u32, &T)> {
        let shared = self.shared_len() as u32;
        (shared_seen as u32..shared)
            .map(move |i| (i, self.shared_get(i)))
            .chain(self.mine().iter().enumerate().skip(own_seen).map(move |(k, t)| (self.own_base + k as u32, t)))
    }

    /// The id the next push gives, which `first..len()` ranges over the pushes since.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.shared {
            None => self.plain.len(),
            Some(s) if self.alloc_shared => s.len(),
            Some(_) => self.own_base as usize + self.own.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.mine().is_empty() && self.shared.as_ref().map_or(true, |s| s.len() == 0)
    }

    /// How many shared records this worker may read: the published ones, and under the
    /// loader's lock its own staged ones as well.
    pub fn shared_len(&self) -> usize {
        match &self.shared {
            Some(s) if lock_depth() > 0 => s.len(),
            Some(s) => s.published(),
            None => 0,
        }
    }

    /// The local records, in order: the whole arena with one worker.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.mine().iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        #[cfg(debug_assertions)]
        self.check_unescaped_all();
        self.mine_mut().iter_mut()
    }

    pub fn own(&self) -> &[T] {
        self.mine()
    }

    /// The records with one worker, for a change of the whole arena.
    pub fn own_mut(&mut self) -> &mut Vec<T> {
        assert!(self.shared.is_none(), "a forked arena changed as a vector");
        &mut self.plain
    }

    /// The last record `pred` holds for: among the shared ones under the loader's lock, where
    /// a push goes then, else among the worker's own.
    pub fn rfind_mut(&mut self, pred: impl Fn(&T) -> bool) -> Option<&mut T>
    where
        T: Record,
    {
        if self.alloc_shared {
            let n = self.shared.as_ref().map_or(0, |s| s.len()) as u32;
            let found = (0..n).rev().find(|&i| pred(self.get(i)));
            return found.map(|i| self.get_mut(i));
        }
        let j = self.mine().iter().rposition(|t| pred(t))?;
        #[cfg(debug_assertions)]
        self.check_unescaped(j, self.own_base + j as u32);
        self.mine_mut().get_mut(j)
    }

    /// The record pushed last: a staged shared one under the loader's lock, else the last of
    /// the worker's own.
    pub fn last_mut(&mut self) -> Option<&mut T>
    where
        T: Record,
    {
        if self.alloc_shared {
            let n = self.shared.as_ref().map_or(0, |s| s.len());
            return (n > 0).then(|| self.get_mut(n as u32 - 1));
        }
        #[cfg(debug_assertions)]
        if let Some(j) = self.mine().len().checked_sub(1) {
            self.check_unescaped(j, self.own_base + j as u32);
        }
        self.mine_mut().last_mut()
    }

    pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        debug_assert!(self.shared.is_none());
        self.plain.retain(f);
    }

    pub fn truncate(&mut self, n: usize) {
        debug_assert!(self.shared.is_none());
        self.plain.truncate(n);
    }

    /// Makes the records so far the shared region, and this arena worker 0's over it.
    pub fn fork(&mut self) -> Arc<SharedArena<T>> {
        assert!(self.shared.is_none() && self.own_base == 0);
        let shared = Arc::new(SharedArena::from_vec(std::mem::take(&mut self.plain)));
        self.shared = Some(shared.clone());
        self.prefix_len = shared.fork_len as u32;
        self.own_base = worker_base(0);
        self.own = Box::new(SlabVec::with_capacity(1024));
        shared.peers[0].store(&mut *self.own, Ordering::Release);
        shared
    }

    /// The shared region, for another worker's arena over it.
    pub fn shared_arc(&self) -> Arc<SharedArena<T>> {
        self.shared.clone().expect("the arena has no shared region")
    }

    /// Worker `worker`'s arena over a shared region.
    pub fn attach(shared: Arc<SharedArena<T>>, worker: usize) -> Arena<T> {
        let mut own = Box::new(SlabVec::with_capacity(1024));
        shared.peers[worker].store(&mut *own, Ordering::Release);
        let prefix_len = shared.fork_len as u32;
        Arena { plain: Vec::new(), own, own_base: worker_base(worker), shared: Some(shared), prefix_len, working: FxMap::default(), made: 0, staged_from: u32::MAX, alloc_shared: false, peer_reads: None, view: None }
    }

    /// The loader's lock was taken at depth one: what is pushed from here on is staged.
    pub fn lock_taken(&mut self) {
        self.staged_from = self.shared_len() as u32;
    }

    /// The loader's lock is released: the working copies are published, the staged records
    /// are anyone's.
    pub fn lock_released(&mut self)
    where
        T: Record,
    {
        self.publish_staged();
        self.flush();
    }

    /// The staged records are every worker's: the first step of a release, before the maps
    /// that name them are applied and the working copies that hold their cells' states are
    /// published (`Worker::lock_released`).
    pub fn publish_staged(&mut self) {
        self.staged_from = u32::MAX;
        if let Some(s) = &self.shared {
            s.publish_all();
        }
    }

    /// Publishes the working copies made so far, under the lock.
    pub fn flush(&mut self)
    where
        T: Record,
    {
        if self.working.is_empty() {
            return;
        }
        let s = self.shared.as_ref().expect("no shared region");
        let mut copies: Vec<(u32, (u32, Box<T>))> = self.working.drain().collect();
        copies.sort_by_key(|(_, (made, _))| std::cmp::Reverse(*made));
        for (i, (_, copy)) in copies {
            s.publish(i, copy);
        }
    }

    /// Publishes the working copy of `i` alone, under the lock: a cell's state.
    pub fn publish(&mut self, i: u32)
    where
        T: Record,
    {
        if let Some((_, copy)) = self.working.remove(&i) {
            self.shared.as_ref().expect("no shared region").publish(i, copy);
        }
    }

    /// Takes the local records out, for the merge, once no other worker reads them.
    pub fn take_own(&mut self) -> Vec<T> {
        if self.shared.is_none() {
            return std::mem::take(&mut self.plain);
        }
        if let Some(s) = &self.shared {
            if let Some(k) = worker_of(self.own_base) {
                s.peers[k].store(std::ptr::null_mut(), Ordering::Release);
            }
        }
        std::mem::replace(&mut self.own, Box::new(SlabVec::with_capacity(0))).into_vec()
    }

    /// Leaves the shared region with the shared records followed by this worker's own: the
    /// arena is one worker's again, and every id from `LOCAL_BASE` moved down by the returned
    /// amount less `LOCAL_BASE`, which is the shared region's length.
    pub fn merge_own(&mut self) -> u32 {
        let shared = self.shared.take().expect("no shared region to merge into");
        let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("the shared region is still shared"));
        let mut records = shared.into_vec();
        let n = records.len() as u32;
        records.extend(std::mem::replace(&mut self.own, Box::new(SlabVec::with_capacity(0))).into_vec());
        self.join(records);
        n
    }

    /// The shared region's records, `drop`'s taken out, the region left (the retention):
    /// nothing holds a dropped shared record's index across the merge but
    /// the indexes rebuilt after it. Every other worker's arena over the region is dropped by now.
    pub fn retained(&mut self, drop: impl Fn(&T) -> bool, crew: &crate::crew::Crew) -> Vec<T>
    where
        T: Send + 'static,
    {
        assert!(self.own.len() == 0, "the arena's own records were not taken");
        let shared = self.shared.take().expect("no shared region to merge into");
        let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("the shared region is still shared"));
        let (mut records, replaced, superseded) = shared.into_parts();
        // The versions the published copies replaced, freed off the merge's main thread.
        if !replaced.is_empty() || !superseded.is_empty() {
            crew.give_back((replaced, superseded));
        }
        records.retain(|r| !drop(r));
        records
    }

    /// Leaves the shared region with every worker's own records after the retained ones at their
    /// places (`numbering` over `owns`, each worker's own records, this arena's taken as well),
    /// moved by the crew (`gather`); the new id of each own record, per worker.
    pub fn place(&mut self, records: Vec<T>, owns: Vec<Vec<T>>, numbering: &Numbering, crew: &crate::crew::Crew) -> Vec<Vec<u32>>
    where
        T: Send,
    {
        let mut records = records;
        let new = gather(&mut records, owns, numbering, crew);
        self.join(records);
        new
    }

    /// Leaves the shared region: the arena is one worker's again over `records`.
    pub fn join(&mut self, records: Vec<T>) {
        self.shared = None;
        self.prefix_len = 0;
        self.plain = records;
        self.own = Box::new(SlabVec::with_capacity(0));
        self.own_base = 0;
        self.working.clear();
        self.staged_from = u32::MAX;
        self.alloc_shared = false;
    }

    #[inline]
    pub fn own_base(&self) -> u32 {
        self.own_base
    }

    /// Whether this worker may change the record `i` or the entries kept for it: its own, or a
    /// shared one it staged under the loader's lock. Any other is read by other threads.
    #[inline]
    pub fn writable(&self, i: u32) -> bool {
        self.shared.is_none() || self.forked_writable(i)
    }

    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn forked_writable(&self, i: u32) -> bool {
        let j = i.wrapping_sub(self.own_base) as usize;
        if j < self.own.len() {
            // Asked before a write: a sealed record is never written.
            #[cfg(debug_assertions)]
            self.check_unescaped(j, i);
            return true;
        }
        lock_depth() > 0 && i >= self.staged_from && i < LOCAL_BASE
    }
}

impl<T> std::ops::Index<std::ops::Range<usize>> for Arena<T> {
    type Output = [T];
    #[inline]
    fn index(&self, r: std::ops::Range<usize>) -> &[T] {
        if let Some(records) = self.plain.get(r.clone()) {
            return records;
        }
        self.forked_range(r)
    }
}

impl<T> Arena<T> {
    #[inline(never)]
    fn forked_range(&self, r: std::ops::Range<usize>) -> &[T] {
        let s = self.shared.as_ref().expect("no such record");
        if r.start >= LOCAL_BASE as usize {
            let base = self.own_base as usize;
            if worker_of(r.start as u32) == worker_of(self.own_base) {
                return &self.own.as_slice()[r.start - base..r.end - base];
            }
            return s.peer_slice(r.start as u32, (r.end - r.start) as u32);
        }
        assert!(lock_depth() > 0 || r.end <= s.published(), "the shared region's staged records read as a slice");
        &s.records.as_slice()[r]
    }
}

/// The records from `start` in the region `start` names: this worker's own from the mark,
/// or, under the loader's lock, the shared region's from the mark (the ones this holder
/// staged, and what it read `len` as).
impl<T> std::ops::Index<std::ops::RangeFrom<usize>> for Arena<T> {
    type Output = [T];
    #[inline]
    fn index(&self, r: std::ops::RangeFrom<usize>) -> &[T] {
        if self.shared.is_none() {
            return &self.plain[r];
        }
        if r.start >= self.own_base as usize {
            return &self.own.as_slice()[r.start - self.own_base as usize..];
        }
        let s = self.shared.as_ref().expect("no such record");
        assert!(lock_depth() > 0, "the shared region's tail read outside the loader's lock");
        &s.records.as_slice()[r.start..]
    }
}

impl<T> std::ops::IndexMut<std::ops::RangeFrom<usize>> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, r: std::ops::RangeFrom<usize>) -> &mut [T] {
        if self.shared.is_none() {
            return &mut self.plain[r];
        }
        let base = self.own_base as usize;
        if r.start >= base {
            self.forget_own_held_all();
            let n = self.own.len();
            #[cfg(debug_assertions)]
            for j in r.start - base..n {
                self.check_unescaped(j, (base + j) as u32);
            }
            return unsafe { self.own.slice_mut(r.start - base, n) };
        }
        let s = self.shared.as_ref().expect("no such record");
        assert!(lock_depth() > 0 && r.start >= self.staged_from as usize, "the shared region's published records changed as a slice");
        let len = s.records.len();
        unsafe { std::slice::from_raw_parts_mut(s.records.get_mut(r.start), len - r.start) }
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> std::slice::Iter<'a, T> {
        self.mine().iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> std::slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T> std::ops::Index<usize> for Arena<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        self.get(i as u32)
    }
}

impl<T: Record> std::ops::IndexMut<usize> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        self.get_mut(i as u32)
    }
}

/// A directory of fixed chunks over the whole id space, the shared region's ids and every
/// worker's own ranges alike (`worker_base`): a chunk is made by the first write into it and
/// never moves, and it is found by a shift of the id, so that every worker reads and writes
/// every id through the one directory and no entry is ever copied while another thread may be
/// writing it. The directory's slots are zero
/// pages mapped from the system (`alloc::zeroed_pages`), of which a build touches the ones its
/// ids reach.
struct ChunkDir<C> {
    slots: *mut AtomicPtr<C>,
    count: usize,
    shift: u32,
}

unsafe impl<C: Send + Sync> Send for ChunkDir<C> {}
unsafe impl<C: Send + Sync> Sync for ChunkDir<C> {}

impl<C> ChunkDir<C> {
    fn new(shift: u32) -> ChunkDir<C> {
        let count = 1usize << (32 - shift);
        let slots = crate::alloc::zeroed_pages(count * std::mem::size_of::<AtomicPtr<C>>()) as *mut AtomicPtr<C>;
        assert!(!slots.is_null(), "no memory for a chunk directory");
        ChunkDir { slots, count, shift }
    }

    #[inline]
    fn find(&self, i: u32) -> Option<&C> {
        let p = unsafe { (*self.slots.add((i >> self.shift) as usize)).load(Ordering::Acquire) };
        (!p.is_null()).then(|| unsafe { &*p })
    }

    /// The chunk of `i`, made with `fresh` when there is none: the first of two threads making
    /// it at once puts its chunk in, the other drops its own.
    #[inline]
    fn get_or_make(&self, i: u32, fresh: impl FnOnce() -> Box<C>) -> &C {
        match self.find(i) {
            Some(c) => c,
            None => self.make(i, fresh),
        }
    }

    #[cold]
    #[inline(never)]
    fn make(&self, i: u32, fresh: impl FnOnce() -> Box<C>) -> &C {
        let slot = unsafe { &*self.slots.add((i >> self.shift) as usize) };
        let made = Box::into_raw(fresh());
        crate::shake::point(crate::shake::Point::Grown);
        match slot.compare_exchange(std::ptr::null_mut(), made, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => unsafe { &*made },
            Err(theirs) => {
                drop(unsafe { Box::from_raw(made) });
                unsafe { &*theirs }
            }
        }
    }

    /// The chunks made, with the first id each covers.
    fn chunks(&self) -> impl Iterator<Item = (u32, &C)> {
        (0..self.count).filter_map(move |k| {
            let p = unsafe { (*self.slots.add(k)).load(Ordering::Acquire) };
            (!p.is_null()).then(|| ((k as u32) << self.shift, unsafe { &*p }))
        })
    }
}

impl<C> Drop for ChunkDir<C> {
    fn drop(&mut self) {
        for k in 0..self.count {
            let p = unsafe { (*self.slots.add(k)).load(Ordering::Relaxed) };
            if !p.is_null() {
                drop(unsafe { Box::from_raw(p) });
            }
        }
        unsafe { crate::alloc::free_zeroed_pages(self.slots as *mut u8, self.count * std::mem::size_of::<AtomicPtr<C>>()) };
    }
}

const ENTRY_CHUNK_BITS: u32 = 14;
const ENTRY_CHUNK: usize = 1 << ENTRY_CHUNK_BITS;

/// The entries of a `Parallel` table while it is shared: a chunk of `ENTRY_CHUNK` entries, the
/// default in those never set.
pub struct Entries<T: Copy> {
    dir: ChunkDir<[std::cell::UnsafeCell<T>; ENTRY_CHUNK]>,
    default: T,
}

unsafe impl<T: Copy + Send> Send for Entries<T> {}
unsafe impl<T: Copy + Send + Sync> Sync for Entries<T> {}

impl<T: Copy> Entries<T> {
    fn new(default: T) -> Entries<T> {
        Entries { dir: ChunkDir::new(ENTRY_CHUNK_BITS), default }
    }

    #[inline]
    fn get(&self, i: u32) -> Option<T> {
        self.dir.find(i).map(|c| unsafe { *c[i as usize & (ENTRY_CHUNK - 1)].get() })
    }

    /// Sets the entry of `i`: the writer is the id's own, which no other thread writes (the
    /// worker that made the record, or the loader's lock holder for a staged shared one), and
    /// what another thread reads of it is published after the write.
    #[inline]
    fn set(&self, i: u32, v: T) {
        let c = self.dir.get_or_make(i, || Self::chunk(&[], self.default));
        unsafe { *c[i as usize & (ENTRY_CHUNK - 1)].get() = v };
    }

    /// A chunk holding `from` and the default after it.
    fn chunk(from: &[T], default: T) -> Box<[std::cell::UnsafeCell<T>; ENTRY_CHUNK]> {
        let chunk: Vec<std::cell::UnsafeCell<T>> = (0..ENTRY_CHUNK).map(|k| std::cell::UnsafeCell::new(from.get(k).copied().unwrap_or(default))).collect();
        let chunk: Box<[std::cell::UnsafeCell<T>]> = chunk.into_boxed_slice();
        unsafe { Box::from_raw(Box::into_raw(chunk) as *mut [std::cell::UnsafeCell<T>; ENTRY_CHUNK]) }
    }

    /// The entries from `first` on into `out`, a chunk's run at a time, the default where no
    /// chunk is.
    fn copy_run(&self, first: u32, out: &mut [std::mem::MaybeUninit<T>]) {
        let mut done = 0;
        while done < out.len() {
            let i = first + done as u32;
            let at = i as usize & (ENTRY_CHUNK - 1);
            let n = (ENTRY_CHUNK - at).min(out.len() - done);
            match self.dir.find(i) {
                // SAFETY: the entries are no longer written: the workers are joined.
                Some(c) => {
                    for (o, e) in out[done..done + n].iter_mut().zip(&c[at..at + n]) {
                        o.write(unsafe { *e.get() });
                    }
                }
                None => {
                    for o in &mut out[done..done + n] {
                        o.write(self.default);
                    }
                }
            }
            done += n;
        }
    }

    /// The chunk from `first`, which none holds yet, made of `block`.
    fn fill(&self, first: u32, block: &[T]) {
        self.dir.get_or_make(first, || Self::chunk(block, self.default));
    }

    fn held(&self) -> usize {
        self.dir.count * std::mem::size_of::<usize>() + self.dir.chunks().count() * ENTRY_CHUNK * std::mem::size_of::<T>()
    }
}

/// A table parallel to an arena's records, one entry per id and a default for the ids it has
/// not reached (`Program::expr_types`, `expr_spans`). With one worker a vector by the id; while
/// the workers share it, the entries of every id in chunks that never move (`Entries`), set by
/// the record's maker and read by every worker, another worker's records included.
pub struct Parallel<T: Copy> {
    own: Vec<T>,
    shared: Option<Arc<Entries<T>>>,
    default: T,
    /// Whether the entries set are the loader's lock holder's, of staged shared records.
    pub alloc_shared: bool,
    /// Every read while the entries are shared goes through this, which hands the reader the
    /// entry in its view and never changes the entry (`types::expr_type_in_view`).
    pub shared_reads: Option<fn(u32, T) -> T>,
    /// The marks of the arena the entries parallel and the reader's own range's base: a peer's
    /// entry is read as its record is, after the record escaped, and marks it read
    /// (`follow_escapes`, the assertion-enabled builds').
    #[cfg(debug_assertions)]
    marks: Option<(Arc<EscapeMarks>, u32)>,
}

impl<T: Copy + Send + Sync> Parallel<T> {
    pub fn new(default: T) -> Parallel<T> {
        Parallel {
            own: Vec::new(),
            shared: None,
            default,
            alloc_shared: false,
            shared_reads: None,
            #[cfg(debug_assertions)]
            marks: None,
        }
    }

    #[inline]
    pub fn get(&self, i: u32) -> Option<T> {
        match self.own.get(i as usize) {
            Some(&v) => Some(v),
            None if self.shared.is_some() => self.shared_get(i),
            None => None,
        }
    }

    /// An entry while the workers share the entries (the vector is empty then).
    #[inline(never)]
    fn shared_get(&self, i: u32) -> Option<T> {
        #[cfg(debug_assertions)]
        if let (Some((marks, own_base)), Some(k)) = (&self.marks, worker_of(i)) {
            if worker_base(k) != *own_base {
                marks.check_escaped(k, (i - worker_base(k)) as usize, i, std::any::type_name::<T>());
            }
        }
        let v = self.shared.as_ref()?.get(i);
        match (self.shared_reads, v) {
            (Some(read), Some(v)) => Some(read(i, v)),
            _ => v,
        }
    }

    #[inline]
    pub fn set(&mut self, i: u32, v: T) {
        if self.shared.is_some() {
            return self.set_shared(i, v);
        }
        let j = i as usize;
        if j < self.own.len() {
            self.own[j] = v;
        } else if j == self.own.len() {
            self.own.push(v);
        } else {
            self.grow_own(j, v);
        }
    }

    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn set_shared(&mut self, i: u32, v: T) {
        assert!(i >= LOCAL_BASE || lock_depth() > 0, "a shared record's entry set outside the loader's lock");
        #[cfg(debug_assertions)]
        if let Some((marks, own_base)) = &self.marks {
            check_own_entry(marks, *own_base, i, std::any::type_name::<T>());
        }
        self.shared.as_ref().expect("no shared region").set(i, v);
    }

    #[cold]
    #[inline(never)]
    fn grow_own(&mut self, j: usize, v: T) {
        self.own.resize(j, self.default);
        self.own.push(v);
    }

    /// The entries, with one worker: the whole table.
    pub fn own(&self) -> &[T] {
        &self.own
    }

    pub fn own_mut(&mut self) -> &mut Vec<T> {
        &mut self.own
    }

    /// The entries so far become the shared ones, which every worker reads and sets.
    pub fn fork(&mut self) -> Arc<Entries<T>> {
        assert!(self.shared.is_none());
        let entries = Entries::new(self.default);
        for (k, block) in std::mem::take(&mut self.own).chunks(ENTRY_CHUNK).enumerate() {
            entries.fill((k * ENTRY_CHUNK) as u32, block);
        }
        let shared = Arc::new(entries);
        self.shared = Some(shared.clone());
        shared
    }

    pub fn shared_arc(&self) -> Arc<Entries<T>> {
        self.shared.clone().expect("no shared region")
    }

    pub fn attach(shared: Arc<Entries<T>>, default: T, _worker: usize) -> Parallel<T> {
        Parallel {
            own: Vec::new(),
            shared: Some(shared),
            default,
            alloc_shared: false,
            shared_reads: None,
            #[cfg(debug_assertions)]
            marks: None,
        }
    }

    /// A peer's entry is read from here as the record it parallels is, in the arena whose marks
    /// these are, by a reader whose own records start at `own_base`: refused before the record
    /// escaped its owner, and marking it read, so that the owner's write of the entry is refused
    /// after (the assertion-enabled builds').
    #[cfg(debug_assertions)]
    pub fn follow_escapes(&mut self, marks: Arc<EscapeMarks>, own_base: u32) {
        self.marks = Some((marks, own_base));
    }

    /// Leaves the shared entries with those of the first `numbering.shared` ids followed by every
    /// worker's at the places of the records they parallel (the arena's `numbering`), each thread
    /// of the crew its contiguous share, a chunk's run copied at a time. Every other worker's view
    /// of the entries is dropped by now.
    pub fn place(&mut self, numbering: &Numbering, crew: &crate::crew::Crew) {
        let shared = self.shared.take().expect("no shared region");
        let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("the shared table is still shared"));
        let len = numbering.len as usize;
        let mut entries: Vec<T> = Vec::with_capacity(len);
        let dst = crate::crew::Shared(entries.as_mut_ptr());
        let (dst, shared) = (&dst, &shared);
        crew.run(&|k| {
            // SAFETY: the threads' shares of the places are disjoint, each written once.
            let out = |at: u32, n: usize| unsafe { std::slice::from_raw_parts_mut(dst.0.add(at as usize) as *mut std::mem::MaybeUninit<T>, n) };
            let mine = crew.share(k, 0..len);
            let head = mine.start..mine.end.min(numbering.shared as usize);
            if head.start < head.end {
                shared.copy_run(head.start as u32, out(head.start as u32, head.end - head.start));
            }
            let tail = mine.start.max(numbering.shared as usize)..mine.end;
            for r in numbering.runs_in(tail.start as u32..tail.end as u32) {
                shared.copy_run(worker_base(r.worker) + r.start, out(r.at, (r.end - r.start) as usize));
            }
        });
        // SAFETY: every entry below `len` was written once.
        unsafe { entries.set_len(len) };
        self.join(entries);
    }

    pub fn join(&mut self, entries: Vec<T>) {
        self.shared = None;
        self.own = entries;
        self.alloc_shared = false;
    }
}

/// Sets in `dst` from bit `at` on the `n` bits of the words `src` gives from bit `from` on, a
/// word or a part of one at a time.
fn or_bits(dst: &mut [u64], at: usize, src: &dyn Fn(usize) -> u64, from: usize, n: usize) {
    let mut done = 0;
    while done < n {
        let (s, d) = (from + done, at + done);
        let take = (64 - s % 64).min(64 - d % 64).min(n - done);
        let mask = if take == 64 { u64::MAX } else { (1u64 << take) - 1 };
        let bits = (src(s / 64) >> (s % 64)) & mask;
        if bits != 0 {
            dst[d / 64] |= bits << (d % 64);
        }
        done += take;
    }
}

const BIT_CHUNK_BITS: u32 = 18;
const BIT_CHUNK_WORDS: usize = 1 << (BIT_CHUNK_BITS - 6);

/// A set of ids as bits, one word per 64 (`Program::expansion_bits`, `leaf_bits`). With one
/// worker a vector of words; while the workers share it, every id's word in chunks that never
/// move, set atomically by any worker and read by every worker (`ChunkDir`).
#[derive(Default)]
pub struct Bits {
    own: Vec<u64>,
    shared: Option<Arc<SharedBits>>,
    pub alloc_shared: bool,
    /// The marks of the arena the bits parallel and the reader's own range's base, as
    /// `Parallel`'s (`follow_escapes`).
    #[cfg(debug_assertions)]
    marks: Option<(Arc<EscapeMarks>, u32)>,
}

/// The owner's write of the entry of record `i` in a table parallel to an arena with these marks,
/// by a worker whose own records start at `own_base`: refused once the record is sealed or a peer
/// read it, and for a record of another worker's.
#[cfg(debug_assertions)]
#[track_caller]
fn check_own_entry(marks: &EscapeMarks, own_base: u32, i: u32, what: &str) {
    if let Some(k) = worker_of(i) {
        assert!(worker_base(k) == own_base, "worker {:?} set the entry ({}) of worker {}'s record {}: another worker's records are read, never changed", worker_of(own_base), what, k, i);
        marks.check_writable(Some(k), i, what);
    }
}

pub struct SharedBits(ChunkDir<[std::sync::atomic::AtomicU64; BIT_CHUNK_WORDS]>);

impl SharedBits {
    pub(crate) fn new() -> SharedBits {
        SharedBits(ChunkDir::new(BIT_CHUNK_BITS))
    }

    #[inline]
    fn word(&self, i: u32) -> Option<&std::sync::atomic::AtomicU64> {
        self.0.find(i).map(|c| &c[(i as usize >> 6) & (BIT_CHUNK_WORDS - 1)])
    }

    #[inline]
    pub(crate) fn has(&self, i: u32) -> bool {
        self.word(i).map_or(false, |w| w.load(Ordering::Acquire) & (1u64 << (i % 64)) != 0)
    }

    pub(crate) fn set(&self, i: u32) {
        let c = self.0.get_or_make(i, || Self::chunk(&[]));
        c[(i as usize >> 6) & (BIT_CHUNK_WORDS - 1)].fetch_or(1u64 << (i % 64), Ordering::AcqRel);
    }

    /// A chunk holding the words `from` and nothing set after them.
    fn chunk(from: &[u64]) -> Box<[std::sync::atomic::AtomicU64; BIT_CHUNK_WORDS]> {
        let words: Vec<std::sync::atomic::AtomicU64> = (0..BIT_CHUNK_WORDS).map(|k| std::sync::atomic::AtomicU64::new(from.get(k).copied().unwrap_or(0))).collect();
        let words: Box<[std::sync::atomic::AtomicU64]> = words.into_boxed_slice();
        unsafe { Box::from_raw(Box::into_raw(words) as *mut [std::sync::atomic::AtomicU64; BIT_CHUNK_WORDS]) }
    }

    fn held(&self) -> usize {
        self.0.count * std::mem::size_of::<usize>() + self.0.chunks().count() * BIT_CHUNK_WORDS * 8
    }
}

impl Bits {
    #[inline]
    pub fn has(&self, i: u32) -> bool {
        match self.own.get((i / 64) as usize) {
            Some(w) => w & (1u64 << (i % 64)) != 0,
            None => self.shared.is_some() && self.shared_has(i),
        }
    }

    /// A bit while the workers share the bits (the vector is empty then).
    #[inline(never)]
    fn shared_has(&self, i: u32) -> bool {
        #[cfg(debug_assertions)]
        if let (Some((marks, own_base)), Some(k)) = (&self.marks, worker_of(i)) {
            if worker_base(k) != *own_base {
                marks.check_escaped(k, (i - worker_base(k)) as usize, i, "a mark");
            }
        }
        self.shared.as_ref().map_or(false, |s| s.has(i))
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set(&mut self, i: u32) {
        if let Some(s) = &self.shared {
            #[cfg(debug_assertions)]
            if let Some((marks, own_base)) = &self.marks {
                check_own_entry(marks, *own_base, i, "a mark");
            }
            return s.set(i);
        }
        let word = (i / 64) as usize;
        if self.own.len() <= word {
            self.grow_own(word);
        }
        self.own[word] |= 1u64 << (i % 64);
    }

    #[cold]
    #[inline(never)]
    fn grow_own(&mut self, word: usize) {
        self.own.resize(word + 1, 0);
    }

    /// The bits so far become the shared ones, which every worker reads and sets.
    pub fn fork(&mut self) -> Arc<SharedBits> {
        assert!(self.shared.is_none());
        let bits = SharedBits::new();
        for (k, block) in std::mem::take(&mut self.own).chunks(BIT_CHUNK_WORDS).enumerate() {
            bits.0.get_or_make((k << BIT_CHUNK_BITS) as u32, || SharedBits::chunk(block));
        }
        let shared = Arc::new(bits);
        self.shared = Some(shared.clone());
        shared
    }

    pub fn shared_arc(&self) -> Arc<SharedBits> {
        self.shared.clone().expect("no shared region")
    }

    pub fn attach(shared: Arc<SharedBits>, _worker: usize) -> Bits {
        Bits {
            own: Vec::new(),
            shared: Some(shared),
            alloc_shared: false,
            #[cfg(debug_assertions)]
            marks: None,
        }
    }

    /// A peer's bit is read as the record it marks is, and the owner's write of it refused once
    /// the record is sealed (`Parallel::follow_escapes`, the assertion-enabled builds').
    #[cfg(debug_assertions)]
    pub fn follow_escapes(&mut self, marks: Arc<EscapeMarks>, own_base: u32) {
        self.marks = Some((marks, own_base));
    }

    pub fn join(&mut self, words: Vec<u64>) {
        self.shared = None;
        self.own = words;
        self.alloc_shared = false;
    }

    /// Leaves the shared bits with every worker's after the first `numbering.shared` at the
    /// places of the records they mark (the arena's `numbering`), a word at a time. Every other
    /// worker's view is dropped by now.
    pub fn place(&mut self, numbering: &Numbering) {
        let shared = self.shared.take().expect("no shared region");
        let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("the shared bits are still shared"));
        let word = |w: usize| shared.word((w * 64) as u32).map_or(0, |a| a.load(Ordering::Relaxed));
        let mut words: Vec<u64> = vec![0; (numbering.len as usize).div_ceil(64)];
        or_bits(&mut words, 0, &word, 0, numbering.shared as usize);
        for r in &numbering.runs {
            or_bits(&mut words, r.at as usize, &word, (worker_base(r.worker) + r.start) as usize, (r.end - r.start) as usize);
        }
        let shared_words = (numbering.shared as usize).div_ceil(64);
        while words.len() > shared_words && words.last() == Some(&0) {
            words.pop();
        }
        self.join(words);
    }
}

/// A table keyed by an id of an arena with two regions, for the caches the interpreter keeps
/// per expression, class or function: an entry it has not reached reads as the default, and a
/// write past the end grows the table. The ids of other workers' records, which a worker's
/// interpreter reads through their chunks, are kept apart in a map: they size no table.
pub struct Dense<T> {
    shared: Vec<T>,
    own: Vec<T>,
    foreign: FxMap<usize, T>,
    default: T,
    /// The first of the worker's own ids (`worker_base`).
    own_base: usize,
}

thread_local! {
    /// The worker this thread types for (`Worker::work`): whose own ids the tables it makes
    /// hold densely.
    static THREAD_WORKER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The cell directories kept for reuse and the worker this thread types for, for
/// `TEQ_SESSION_INVENTORY`.
pub fn inventory() -> Vec<(&'static str, usize)> {
    vec![("spare cell directories", SPARE_DIRECTORIES.lock().map_or(0, |d| d.len())), ("thread worker", THREAD_WORKER.with(|w| w.get()))]
}

/// Makes `worker` the one this thread types for.
pub fn set_thread_worker(worker: usize) {
    THREAD_WORKER.with(|w| w.set(worker));
}

impl<T: Clone> Dense<T> {
    /// A table for the worker this thread types for: its own ids dense, other workers' apart.
    pub fn new(default: T) -> Dense<T> {
        let own_base = worker_base(THREAD_WORKER.with(|w| w.get())) as usize;
        Dense { shared: Vec::new(), own: Vec::new(), foreign: FxMap::default(), default, own_base }
    }

    /// Whether `i` is an id of another worker's own records.
    #[inline]
    fn is_foreign(&self, i: usize) -> bool {
        i >= LOCAL_BASE as usize && !(self.own_base..self.own_base + WORKER_SPAN as usize).contains(&i)
    }

    /// The shared entries end below `own_base`, so an index within them needs no compare
    /// against the base: the one bounds check of a vector, which is what the interpreter's
    /// reads of one worker's program pay.
    #[inline]
    pub fn get(&self, i: usize) -> Option<&T> {
        if let Some(v) = self.shared.get(i) {
            return Some(v);
        }
        self.get_apart(i)
    }

    #[inline(never)]
    fn get_apart(&self, i: usize) -> Option<&T> {
        // The worker's own ids first, dense from its base: an id below the base wraps past any
        // length, and another worker's lies past the own span.
        if let Some(v) = self.own.get(i.wrapping_sub(self.own_base)) {
            return Some(v);
        }
        if self.is_foreign(i) {
            return self.foreign.get(&i);
        }
        None
    }

    #[inline]
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i < self.shared.len() {
            return self.shared.get_mut(i);
        }
        if self.is_foreign(i) {
            return self.foreign.get_mut(&i);
        }
        self.own.get_mut(i.checked_sub(self.own_base)?)
    }

    /// The entry `i`, made with the default when it is past the end.
    #[inline]
    pub fn at(&mut self, i: usize) -> &mut T {
        if i >= LOCAL_BASE as usize {
            return self.at_apart(i);
        }
        if self.shared.len() <= i {
            self.shared.resize(i + 1, self.default.clone());
        }
        &mut self.shared[i]
    }

    #[inline(never)]
    fn at_apart(&mut self, i: usize) -> &mut T {
        let j = i.wrapping_sub(self.own_base);
        if j < self.own.len() {
            return &mut self.own[j];
        }
        if self.is_foreign(i) {
            return self.foreign.entry(i).or_insert_with(|| self.default.clone());
        }
        let j = i - self.own_base;
        if self.own.len() <= j {
            self.own.resize(j + 1, self.default.clone());
        }
        &mut self.own[j]
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.shared.iter().chain(self.own.iter()).chain(self.foreign.values())
    }

    /// Every entry with its id.
    pub fn iter_mut_indexed(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        let base = self.own_base;
        self.shared.iter_mut().enumerate().chain(self.own.iter_mut().enumerate().map(move |(j, t)| (base + j, t))).chain(self.foreign.iter_mut().map(|(&i, t)| (i, t)))
    }

    pub fn clear(&mut self) {
        self.shared.clear();
        self.own.clear();
        self.foreign.clear();
    }
}

impl<T: Clone> std::ops::Index<usize> for Dense<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        self.get(i).unwrap_or(&self.default)
    }
}

impl<T: Clone> std::ops::IndexMut<usize> for Dense<T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        self.at(i)
    }
}

/// A table with one entry per file, as a worker holds it: the program's files' entries are the
/// worker's own (a copy of the signature phase's, with the memos it fills), the pseudo files'
/// (library bodies converted during the bodies, under the loader's lock) are shared and read
/// by every worker. A pseudo file's entry is a shared arena's record: pushed staged, readable
/// by the other workers once the lock is released, and changed afterwards by a copy that
/// replaces it, never in place under a reader.
pub struct FileVec<T> {
    own: Vec<T>,
    ext: Option<Arena<T>>,
    /// The program files' entries published under the loader's lock (a std file entered
    /// late): what every worker reads in place of its own copy.
    late: Option<Arc<SharedMap<u32, T>>>,
}

impl<T> FileVec<T> {
    pub fn from_vec(v: Vec<T>) -> FileVec<T> {
        FileVec { own: v, ext: None, late: None }
    }

    pub fn len(&self) -> usize {
        self.own.len() + self.ext.as_ref().map_or(0, |e| e.shared_len())
    }

    #[inline]
    pub fn get(&self, i: usize) -> Option<&T> {
        if i < self.own.len() && self.late.as_ref().map_or(true, |late| late.is_empty()) {
            return Some(&self.own[i]);
        }
        self.get_forked(i)
    }

    /// An entry while the workers share the table: a program file's as published, or a
    /// pseudo file's.
    #[inline(never)]
    fn get_forked(&self, i: usize) -> Option<&T> {
        if i < self.own.len() {
            if let Some(v) = self.late.as_ref().and_then(|late| late.get(&(i as u32))) {
                return Some(v);
            }
            return Some(&self.own[i]);
        }
        self.ext.as_ref()?.try_get((i - self.own.len()) as u32)
    }

    /// Publishes this worker's entry of the program file `i` to every worker, under the
    /// loader's lock.
    pub fn publish(&mut self, i: usize)
    where
        T: Clone,
    {
        let Some(late) = &self.late else { return };
        assert!(lock_depth() > 0, "a file's entry published outside the loader's lock");
        late.replace(i as u32, self.own[i].clone());
    }

    pub fn push(&mut self, v: T) {
        match &mut self.ext {
            Some(e) => {
                assert!(lock_depth() > 0, "a pseudo file's entry pushed outside the loader's lock");
                e.push(v);
            }
            None => self.own.push(v),
        }
    }

    pub fn fork(&mut self) {
        assert!(self.ext.is_none());
        let mut ext = Arena::new();
        ext.fork();
        ext.alloc_shared = true;
        self.ext = Some(ext);
        self.late = Some(Arc::new(SharedMap::new()));
    }

    /// Another worker's table over the same pseudo files, with its own copy of the program
    /// files' entries.
    pub fn attach(&self, worker: usize) -> FileVec<T>
    where
        T: Clone,
    {
        let ext = self.ext.as_ref().map(|e| {
            let mut a = Arena::attach(e.shared_arc(), worker);
            a.alloc_shared = true;
            a
        });
        FileVec { own: self.own.clone(), ext, late: self.late.clone() }
    }

    /// The loader's lock was taken at depth one: the pseudo files' entries pushed from here on
    /// are staged.
    pub fn lock_taken(&mut self) {
        if let Some(e) = &mut self.ext {
            e.lock_taken();
        }
    }

    /// The staged entries are every worker's (`Arena::publish_staged`).
    pub fn publish_staged(&mut self) {
        if let Some(e) = &mut self.ext {
            e.publish_staged();
        }
    }

    /// The entries changed under the lock replace the published ones.
    pub fn flush(&mut self)
    where
        T: Record,
    {
        if let Some(e) = &mut self.ext {
            e.flush();
        }
    }

    /// Takes over another worker's program-file entries where this worker's are absent.
    pub fn absorb_some<U>(&mut self, other: Vec<Option<U>>)
    where
        T: OptionLike<U>,
    {
        for (mine, theirs) in self.own.iter_mut().zip(other) {
            if mine.is_none() {
                if let Some(v) = theirs {
                    mine.set(v);
                }
            }
        }
    }

    pub fn take_own(&mut self) -> Vec<T> {
        std::mem::take(&mut self.own)
    }

    /// Leaves the shared region: the entries one vector again, the published ones in place of
    /// this worker's copies. Every other worker's view is dropped by now.
    pub fn merge_own(&mut self)
    where
        T: Clone,
    {
        if let Some(late) = self.late.take() {
            for (i, v) in late.iter() {
                self.own[*i as usize] = v.clone();
            }
        }
        let Some(mut ext) = self.ext.take() else { return };
        ext.merge_own();
        self.own.extend(ext.take_own());
    }

    pub fn own(&self) -> &[T] {
        &self.own
    }

    /// The entries one worker's again (`merge_own`), for the merge's renumbering.
    pub fn own_mut(&mut self) -> &mut [T] {
        &mut self.own
    }
}

impl<T> std::ops::Index<usize> for FileVec<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        self.get(i).expect("no such file")
    }
}

impl<T: Record> std::ops::IndexMut<usize> for FileVec<T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        if i < self.own.len() {
            return &mut self.own[i];
        }
        let e = self.ext.as_mut().expect("no such file");
        e.get_mut((i - self.own.len()) as u32)
    }
}

/// An `Option` cell of a `FileVec`, for `absorb_some`.
pub trait OptionLike<U> {
    fn is_none(&self) -> bool;
    fn set(&mut self, v: U);
}

impl<U> OptionLike<U> for Option<U> {
    fn is_none(&self) -> bool {
        Option::is_none(self)
    }
    fn set(&mut self, v: U) {
        *self = Some(v);
    }
}

/// The definitions of every file mapped to what the namer entered for them, as a worker holds
/// them: the program's files' maps are the worker's own (the signature phase's, with what its
/// bodies add: their local and anonymous classes), and what is entered under the loader's
/// lock (a pseudo file's definitions, a std file entered late) is shared. An attached worker
/// reads the program's files' maps as the fork left them (`prefix`, one copy every attached
/// worker shares) until it first writes a file's, which it copies then (the attachment).
pub struct FileMaps<V> {
    own: Vec<FxMap<crate::ast::DefId, V>>,
    /// The first worker's: its maps as the fork left them, for the attached workers.
    snapshot: Option<Arc<Vec<FxMap<crate::ast::DefId, V>>>>,
    /// An attached worker's: the fork's maps, read for every file it has not written.
    prefix: Option<Arc<Vec<FxMap<crate::ast::DefId, V>>>>,
    /// An attached worker's: per file, whether `own` holds its copy of the file's map.
    copied: Vec<bool>,
    shared: Option<Arc<SharedMap<(u32, crate::ast::DefId), V>>>,
    pub to_shared: bool,
}

impl<V: Copy> FileMaps<V> {
    pub fn new(files: usize) -> FileMaps<V> {
        FileMaps { own: (0..files).map(|_| FxMap::default()).collect(), snapshot: None, prefix: None, copied: Vec::new(), shared: None, to_shared: false }
    }

    /// The program file's map this worker reads: its own, or the fork's where it has not
    /// written the file's.
    #[inline]
    fn map(&self, file: usize) -> Option<&FxMap<crate::ast::DefId, V>> {
        match &self.prefix {
            Some(p) if !self.copied.get(file).copied().unwrap_or(false) => p.get(file),
            _ => self.own.get(file),
        }
    }

    /// The file's map, this worker's own to write: an attached worker's copy of the fork's made
    /// on its first write.
    fn map_mut(&mut self, file: usize) -> &mut FxMap<crate::ast::DefId, V> {
        if self.own.len() <= file {
            self.own.resize_with(file + 1, FxMap::default);
        }
        if let Some(p) = &self.prefix {
            if self.copied.len() <= file {
                self.copied.resize(file + 1, false);
            }
            if !self.copied[file] {
                self.copied[file] = true;
                if let Some(m) = p.get(file) {
                    self.own[file] = m.clone();
                }
            }
        }
        &mut self.own[file]
    }

    #[inline]
    pub fn get(&self, file: usize, d: &crate::ast::DefId) -> Option<&V> {
        if let Some(v) = self.map(file).and_then(|m| m.get(d)) {
            return Some(v);
        }
        self.shared.as_ref()?.get(&(file as u32, *d))
    }

    pub fn insert(&mut self, file: usize, d: crate::ast::DefId, v: V) {
        if self.to_shared {
            if let Some(s) = &self.shared {
                s.insert((file as u32, d), v);
                return;
            }
        }
        self.map_mut(file).insert(d, v);
    }

    pub fn remove(&mut self, file: usize, d: &crate::ast::DefId) {
        if self.map(file).is_some_and(|m| m.contains_key(d)) {
            self.map_mut(file).remove(d);
        }
    }

    /// The values of one file's entries.
    pub fn values_in(&self, file: usize) -> Vec<V> {
        let mut out: Vec<V> = self.map(file).map_or(Vec::new(), |m| m.values().copied().collect());
        if let Some(s) = &self.shared {
            out.extend(s.iter().filter(|((f, _), _)| *f as usize == file).map(|(_, v)| *v));
        }
        out
    }

    /// The entries of one file, as pairs.
    pub fn entries_in(&self, file: usize) -> Vec<(crate::ast::DefId, V)> {
        let mut out: Vec<(crate::ast::DefId, V)> = self.map(file).map_or(Vec::new(), |m| m.iter().map(|(d, v)| (*d, *v)).collect());
        if let Some(s) = &self.shared {
            out.extend(s.iter().filter(|((f, _), _)| *f as usize == file).map(|((_, d), v)| (*d, *v)));
        }
        out
    }

    pub fn own(&self) -> &[FxMap<crate::ast::DefId, V>] {
        &self.own
    }

    pub fn own_mut(&mut self) -> &mut Vec<FxMap<crate::ast::DefId, V>> {
        &mut self.own
    }

    /// The shared entries begin; with `attaching`, the program files' maps are kept as the fork
    /// leaves them for the workers that attach.
    pub fn fork(&mut self, attaching: bool) -> Arc<SharedMap<(u32, crate::ast::DefId), V>> {
        let shared = Arc::new(SharedMap::new());
        self.shared = Some(shared.clone());
        if attaching {
            self.snapshot = Some(Arc::new(self.own.clone()));
        }
        shared
    }

    /// Another worker's table over the same shared entries, reading the program files' maps as
    /// the fork left them until it writes one (`map_mut`).
    pub fn attach(&self) -> FileMaps<V> {
        match &self.snapshot {
            Some(s) => FileMaps { own: Vec::new(), snapshot: None, prefix: Some(s.clone()), copied: Vec::new(), shared: self.shared.clone(), to_shared: false },
            None => FileMaps { own: self.own.clone(), snapshot: None, prefix: None, copied: Vec::new(), shared: self.shared.clone(), to_shared: false },
        }
    }

    /// Takes over another worker's program-file entries (`take_own`: its copies of the files it
    /// wrote): the definitions the bodies it typed entered, disjoint from this worker's.
    pub fn absorb(&mut self, other: Vec<FxMap<crate::ast::DefId, V>>) {
        for (mine, theirs) in self.own.iter_mut().zip(other) {
            if theirs.len() > mine.len() {
                let old = std::mem::replace(mine, theirs);
                mine.extend(old);
            } else {
                mine.extend(theirs);
            }
        }
    }

    pub fn take_own(&mut self) -> Vec<FxMap<crate::ast::DefId, V>> {
        std::mem::take(&mut self.own)
    }

    /// Leaves the shared entries: one table again.
    pub fn merge_own(&mut self) {
        self.snapshot = None;
        let Some(shared) = self.shared.take() else { return };
        for ((f, d), v) in shared.iter() {
            let f = *f as usize;
            if self.own.len() <= f {
                self.own.resize_with(f + 1, FxMap::default);
            }
            self.own[f].entry(*d).or_insert(*v);
        }
        self.to_shared = false;
    }
}

/// A map every worker reads without a lock (the publications' carrier): written under
/// the loader's lock by its holder's release, or outside it by a worker's publication
/// (`Layered::publish`), each insert under the map's own short lock. Its four surfaces have
/// contracts of their own: a lookup (`get`), an insert that keeps a key's entry (`insert`), a
/// replacement (`replace`) and the append history (`iter`, `keys`, `values`, `entry_at`, `len`).
pub struct SharedMap<K, V> {
    entries: SlabVec<(K, V)>,
    index: Table,
    serial: Serial,
}

unsafe impl<K: Send + Sync, V: Send + Sync> Sync for SharedMap<K, V> {}
unsafe impl<K: Send, V: Send> Send for SharedMap<K, V> {}

impl<K: Hash + Eq + Copy, V> SharedMap<K, V> {
    pub fn new() -> SharedMap<K, V> {
        SharedMap { entries: SlabVec::with_capacity(16), index: Table::with_capacity(16), serial: Serial::new(false, crate::measure::Wait::SharedMap) }
    }

    pub fn from_map(m: FxMap<K, V>) -> SharedMap<K, V> {
        let out = SharedMap { entries: SlabVec::with_capacity(m.len()), index: Table::with_capacity(m.len()), serial: Serial::new(false, crate::measure::Wait::SharedMap) };
        for (k, v) in m {
            out.insert(k, v);
        }
        out
    }

    /// The key's entry: one probe of one generation of the index. An entry found is whole, and
    /// so is everything its writer wrote before storing it (the index's slot is stored with
    /// release after the entry is pushed, and read with acquire); a miss says that no entry was
    /// indexed when the probe passed, never that none will be.
    #[inline]
    pub fn get(&self, k: &K) -> Option<&V> {
        let h = hash_of(k);
        self.index.find(h, |i| self.entries.get(i as usize).0 == *k).map(|i| &self.entries.get(i as usize).1)
    }

    #[inline]
    pub fn contains_key(&self, k: &K) -> bool {
        self.get(k).is_some()
    }

    /// The append history's keys (`iter`).
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.as_slice().iter().map(|(k, _)| k)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    /// Puts an entry in place of the key's, or adds one: the new version is appended before the
    /// index moves to it, so a reader sees the old version or the new one, each whole; the old
    /// stays in the history for whoever holds it, out of the index.
    pub fn replace(&self, k: K, v: V) {
        let h = hash_of(&k);
        let _held = self.serial.hold();
        let i = self.entries.push((k, v)) as u32;
        if !self.index.update(h, |j| self.entries.get(j as usize).0 == k, i) {
            self.index.insert(h, i);
        }
    }

    /// Adds an entry unless the key has one, which is kept; whether it was added. A reader sees no
    /// entry or the whole one, with what the writer wrote before the call.
    pub fn insert(&self, k: K, v: V) -> bool {
        let h = hash_of(&k);
        let _held = self.serial.hold();
        if self.index.find(h, |i| self.entries.get(i as usize).0 == k).is_some() {
            return false;
        }
        let i = self.entries.push((k, v)) as u32;
        self.index.insert(h, i);
        true
    }

    /// The append history's length, read with acquire: the entries below it are whole.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The entry `i` of the append history: a version `replace` superseded keeps its place, the
    /// new one is appended.
    pub fn entry_at(&self, i: usize) -> (&K, &V) {
        let e = self.entries.get(i);
        (&e.0, &e.1)
    }

    /// The append history in push order, superseded versions included, each entry whole; an
    /// entry met here may not be indexed yet. No reader enters a publication by iterating a
    /// registry: the iterations are the loader's registries' under the
    /// release's order and the merge's after the join, which takes each key's last version.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.as_slice().iter().map(|(k, v)| (k, v))
    }
}

impl<K: Hash + Eq + Copy, V> std::ops::Index<&K> for SharedMap<K, V> {
    type Output = V;
    fn index(&self, k: &K) -> &V {
        self.get(k).expect("no such entry")
    }
}

impl<K: Hash + Eq + Copy, V> Default for SharedMap<K, V> {
    fn default() -> SharedMap<K, V> {
        SharedMap::new()
    }
}

/// A table keyed by an id, as a worker holds it: the shared entries every worker reads (the
/// signature phase's, and what the loader's lock holder adds), and the worker's own, merged
/// after the bodies.
pub struct Layered<K, V> {
    pub local: FxMap<K, V>,
    shared: Option<Arc<SharedMap<K, V>>>,
    /// Whether inserts go to the shared entries: the loader's lock holder's.
    pub to_shared: bool,
    /// The shared entries made under the loader's lock, applied when it is released
    /// (`apply_pending`), once the records they name are every worker's.
    pending: FxMap<K, V>,
}

impl<K: Hash + Eq + Copy, V> Default for Layered<K, V> {
    fn default() -> Layered<K, V> {
        Layered { local: FxMap::default(), shared: None, to_shared: false, pending: FxMap::default() }
    }
}

impl<K: Hash + Eq + Copy, V> Layered<K, V> {
    #[inline]
    
    pub fn get(&self, k: &K) -> Option<&V> {
        if let Some(v) = self.local.get(k) {
            return Some(v);
        }
        // One worker without a fork has neither: the miss costs the one lookup it always did.
        if self.shared.is_none() {
            return None;
        }
        self.get_shared_layers(k)
    }

    #[inline(never)]
    fn get_shared_layers(&self, k: &K) -> Option<&V> {
        if !self.pending.is_empty() {
            if let Some(v) = self.pending.get(k) {
                return Some(v);
            }
        }
        self.shared.as_ref()?.get(k)
    }

    #[inline]
    pub fn contains_key(&self, k: &K) -> bool {
        self.get(k).is_some()
    }

    /// Adds an entry: the worker's own, or a shared one under the loader's lock, which is kept
    /// if the key has one.
    pub fn insert(&mut self, k: K, v: V) {
        if self.to_shared {
            self.insert_shared(k, v);
        } else {
            self.local.insert(k, v);
        }
    }

    /// Adds a shared entry, under the loader's lock, in place of the key's if it has one; the
    /// worker's own entry of the key stays.
    pub fn insert_shared(&mut self, k: K, v: V) {
        debug_assert!(lock_depth() > 0);
        match &self.shared {
            Some(_) => {
                self.pending.insert(k, v);
            }
            None => {
                self.local.insert(k, v);
            }
        }
    }

    /// A worker's own entry made every worker's at once, outside the loader's lock: an entry
    /// point of a publication, stored once everything a
    /// reader entering by it can need is published. A key keeps the entry it has; whether this one
    /// was added.
    pub fn publish(&self, k: K, v: V) -> bool {
        debug_assert!(lock_depth() == 0, "a publication's entry stored under the loader's lock, whose release stores its entries");
        self.shared.as_ref().expect("no shared entries").insert(k, v)
    }

    /// The shared entries the loader's lock holder is about to publish (`apply_pending`), which the
    /// assertion-enabled build's checks of a release read.
    #[cfg(debug_assertions)]
    pub fn pending(&self) -> impl Iterator<Item = (&K, &V)> {
        self.pending.iter()
    }

    /// Makes the shared entries made under the lock every worker's: the last step of a
    /// release.
    pub fn apply_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let s = self.shared.as_ref().expect("no shared entries");
        for (k, v) in self.pending.drain() {
            s.replace(k, v);
        }
    }

    /// Removes the worker's own entry.
    pub fn remove(&mut self, k: &K) -> Option<V> {
        self.local.remove(k)
    }

    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.local.retain(f);
    }

    /// The local entries, then the shared ones.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.local.iter().chain(self.shared.iter().flat_map(|s| s.iter()))
    }

    pub fn is_empty(&self) -> bool {
        self.local.is_empty() && self.shared.as_ref().map_or(true, |s| s.len() == 0)
    }

    pub fn fork(&mut self) -> Arc<SharedMap<K, V>> {
        let shared = Arc::new(SharedMap::from_map(std::mem::take(&mut self.local)));
        self.shared = Some(shared.clone());
        shared
    }

    /// Another worker's layer over the same shared entries.
    pub fn attach(&self) -> Layered<K, V> {
        Layered { local: FxMap::default(), shared: self.shared.clone(), to_shared: false, pending: FxMap::default() }
    }

    /// The first entry that `wanted` holds of, among this worker's own, its pending and the
    /// shared ones: a lookup by value over a small table.
    pub fn find(&self, wanted: impl Fn(&K, &V) -> bool) -> Option<(K, &V)> {
        if let Some((k, v)) = self.local.iter().chain(self.pending.iter()).find(|(k, v)| wanted(k, v)) {
            return Some((*k, v));
        }
        let shared = self.shared.as_ref()?;
        shared.iter().find(|(k, v)| wanted(k, v)).map(|(k, v)| (*k, v))
    }

    /// The shared entries so far, by their insertion index (`entry_at`).
    pub fn shared_len(&self) -> usize {
        self.shared.as_ref().map_or(0, |s| s.len())
    }

    pub fn shared_entry_at(&self, i: usize) -> (&K, &V) {
        self.shared.as_ref().expect("no shared entries").entry_at(i)
    }

    /// Takes over another worker's own entries.
    pub fn absorb(&mut self, other: FxMap<K, V>) {
        self.local.extend(other);
    }

    pub fn take_local(&mut self) -> FxMap<K, V> {
        std::mem::take(&mut self.local)
    }

    /// Leaves the shared entries: one map again, the shared entries and the worker's own
    /// together (an own entry wins).
    pub fn merge_own(&mut self)
    where
        V: Clone,
    {
        let Some(shared) = self.shared.take() else { return };
        let own = std::mem::take(&mut self.local);
        // A later entry of a key replaced an earlier one: the map takes the last.
        let mut entries: FxMap<K, V> = FxMap::default();
        for (k, v) in shared.iter() {
            entries.insert(*k, v.clone());
        }
        entries.extend(own);
        self.join(entries);
    }

    /// Leaves the shared entries: one map again.
    pub fn join(&mut self, entries: FxMap<K, V>) {
        self.shared = None;
        self.local = entries;
        self.to_shared = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::Completion;

    /// The merge's placement as it was built first, record by record: the shared records, then
    /// the segments' in their order, then what no segment covers, worker by worker.
    fn placed_one_by_one<T: Clone>(shared: &[T], owns: &[Vec<T>], segments: &[Segment]) -> (Vec<T>, Vec<Vec<u32>>) {
        let mut records = shared.to_vec();
        let mut new: Vec<Vec<u32>> = owns.iter().map(|v| vec![u32::MAX; v.len()]).collect();
        for s in segments {
            for i in s.start..s.end {
                new[s.worker][i as usize] = records.len() as u32;
                records.push(owns[s.worker][i as usize].clone());
            }
        }
        for (k, own) in owns.iter().enumerate() {
            for (i, r) in own.iter().enumerate() {
                if new[k][i] == u32::MAX {
                    new[k][i] = records.len() as u32;
                    records.push(r.clone());
                }
            }
        }
        (records, new)
    }

    /// Segments over three workers' records in an order of their own, with gaps no segment covers.
    fn scattered() -> (Vec<usize>, Vec<Segment>) {
        let lens = vec![7, 0, 12];
        let seg = |worker, start, end| Segment { worker, start, end };
        (lens, vec![seg(2, 5, 9), seg(0, 0, 2), seg(2, 0, 3), seg(0, 4, 7), seg(2, 11, 12)])
    }

    /// Each worker's segments, in an order of its own (`Numbering::of` sorts them).
    fn covered(lens: &[usize], segments: &[Segment]) -> Vec<Vec<(u32, u32)>> {
        (0..lens.len()).map(|k| segments.iter().filter(|s| s.worker == k).map(|s| (s.start, s.end)).collect()).collect()
    }

    #[test]
    fn the_numbering_places_by_prefix_sums_as_one_by_one() {
        let (lens, segments) = scattered();
        let n = Numbering::of(4, &segments, covered(&lens, &segments), &lens);
        let at: Vec<(usize, u32, u32, u32)> = n.runs.iter().map(|r| (r.worker, r.start, r.end, r.at)).collect();
        assert_eq!(at, [(2, 5, 9, 4), (0, 0, 2, 8), (2, 0, 3, 10), (0, 4, 7, 13), (2, 11, 12, 16), (0, 2, 4, 17), (2, 3, 5, 19), (2, 9, 11, 21)]);
        assert_eq!((n.len, n.uncovered), (23, 6));
        let cut: Vec<Run> = n.runs_in(9..14).collect();
        assert_eq!(cut, [Run { worker: 0, start: 1, end: 2, at: 9 }, Run { worker: 2, start: 0, end: 3, at: 10 }, Run { worker: 0, start: 4, end: 5, at: 13 }]);
    }

    #[test]
    fn the_crews_placement_moves_every_record_once_where_one_by_one_puts_it() {
        let (lens, segments) = scattered();
        let shared: Vec<String> = (0..4).map(|i| format!("s{}", i)).collect();
        let owns: Vec<Vec<String>> = lens.iter().enumerate().map(|(k, &n)| (0..n).map(|i| format!("w{}r{}", k, i)).collect()).collect();
        let (want, want_new) = placed_one_by_one(&shared, &owns, &segments);
        for threads in [1, 2, 3, 8] {
            crate::crew::Crew::with(threads, |crew| {
                let n = Numbering::of(shared.len() as u32, &segments, covered(&lens, &segments), &lens);
                let mut records = shared.clone();
                let new = gather(&mut records, owns.clone(), &n, crew);
                assert_eq!((&records, &new), (&want, &want_new), "{} threads", threads);
            });
        }
    }

    #[test]
    fn an_arenas_placement_keeps_the_retained_shared_records_first() {
        let (lens, segments) = scattered();
        crate::crew::Crew::with(3, |crew| {
            let mut main = Arena::from_vec((0..4u32).map(|i| 100 + i).collect());
            let shared = main.fork();
            let mut workers: Vec<Arena<u32>> = (1..3).map(|k| Arena::attach(shared.clone(), k)).collect();
            drop(shared);
            workers.insert(0, main);
            for (k, a) in workers.iter_mut().enumerate() {
                for i in 0..lens[k] {
                    a.push((k * 1000 + i) as u32);
                }
            }
            let owns: Vec<Vec<u32>> = workers.iter_mut().map(|a| a.take_own()).collect();
            let mut main = workers.remove(0);
            drop(workers);
            let records = main.retained(|&r| r == 101, crew);
            let (want, want_new) = placed_one_by_one(&records, &owns, &segments);
            let n = Numbering::of(records.len() as u32, &segments, covered(&lens, &segments), &lens);
            let new = main.place(records, owns, &n, crew);
            assert_eq!(new, want_new);
            assert_eq!((0..n.len).map(|i| *main.get(i)).collect::<Vec<u32>>(), want);
        });
    }

    #[test]
    fn the_side_tables_and_the_bits_follow_their_records() {
        let (lens, segments) = scattered();
        let shared_ids = 70;
        let mut entries: Parallel<u32> = Parallel::new(7);
        let mut bits = Bits::default();
        for i in 0..shared_ids as u32 {
            entries.set(i, i * 3);
            if i % 3 == 0 {
                bits.set(i);
            }
        }
        let shared_entries = entries.fork();
        let shared_bits = bits.fork();
        let mut want_entries: Vec<u32> = (0..shared_ids as u32).map(|i| i * 3).collect();
        let mut want_bits: Vec<bool> = (0..shared_ids as u32).map(|i| i % 3 == 0).collect();
        let own_entry = |k: usize, i: u32| (k as u32 + 1) * 10_000 + i;
        let own_bit = |k: usize, i: u32| (i + k as u32) % 2 == 0 || i == 11;
        for (k, &n) in lens.iter().enumerate() {
            for i in 0..n as u32 {
                if i != 4 {
                    shared_entries.set(worker_base(k) + i, own_entry(k, i));
                }
                if own_bit(k, i) {
                    shared_bits.set(worker_base(k) + i);
                }
            }
        }
        drop((shared_entries, shared_bits));
        let owns: Vec<Vec<(usize, u32)>> = lens.iter().enumerate().map(|(k, &n)| (0..n as u32).map(|i| (k, i)).collect()).collect();
        let (order, _) = placed_one_by_one(&[], &owns, &segments);
        for (k, i) in order {
            want_entries.push(if i == 4 { 7 } else { own_entry(k, i) });
            want_bits.push(own_bit(k, i));
        }
        crate::crew::Crew::with(4, |crew| {
            let n = Numbering::of(shared_ids as u32, &segments, covered(&lens, &segments), &lens);
            entries.place(&n, crew);
            bits.place(&n);
        });
        assert_eq!(entries.own(), &want_entries[..]);
        assert_eq!((0..want_bits.len() as u32).map(|i| bits.has(i)).collect::<Vec<bool>>(), want_bits);
    }

    #[test]
    fn a_shared_maps_surfaces_keep_their_contracts() {
        let m: SharedMap<u32, &str> = SharedMap::new();
        assert!(m.insert(1, "a"));
        assert!(!m.insert(1, "b"), "an insert keeps the key's entry");
        assert_eq!(m.get(&1), Some(&"a"));
        m.replace(1, "c");
        assert_eq!(m.get(&1), Some(&"c"), "a lookup finds the version the index moved to");
        m.replace(2, "d");
        assert_eq!(m.len(), 3, "the history holds the superseded version");
        let history: Vec<(u32, &str)> = m.iter().map(|(&k, &v)| (k, v)).collect();
        assert_eq!(history, [(1, "a"), (1, "c"), (2, "d")]);
        assert_eq!(m.entry_at(1), (&1, &"c"));
        assert_eq!(m.get(&3), None);
    }

    #[test]
    fn an_entry_found_carries_what_its_writer_wrote_before_it() {
        // Its threads' allocations would move the allocator's counts its own tests compare.
        let _serial = crate::alloc::serial_test();
        const N: usize = 20_000;
        let data: Arc<Vec<std::sync::atomic::AtomicUsize>> = Arc::new((0..N).map(|_| std::sync::atomic::AtomicUsize::new(0)).collect());
        let m: Arc<SharedMap<usize, usize>> = Arc::new(SharedMap::new());
        let readers: Vec<_> = (0..3)
            .map(|r| {
                let (data, m) = (data.clone(), m.clone());
                std::thread::spawn(move || {
                    let start = std::time::Instant::now();
                    for seen in 0..N {
                        let k = (seen * 7 + r) % N;
                        let v = loop {
                            if let Some(&v) = m.get(&k) {
                                break v;
                            }
                            assert!(start.elapsed().as_secs() < 30, "entry {} never found", k);
                            std::hint::spin_loop();
                        };
                        assert_eq!(data[v].load(Ordering::Relaxed), v + 1, "an entry found before what its writer wrote ahead of it");
                    }
                })
            })
            .collect();
        for i in 0..N {
            data[i].store(i + 1, Ordering::Relaxed);
            if i % 2 == 0 {
                m.insert(i, i);
            } else {
                m.replace(i, i);
            }
        }
        for r in readers {
            r.join().unwrap();
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_nested_publication_seals_itself_and_not_the_record_enclosing_it() {
        let mut a: Arena<u32> = Arena::from_vec(vec![0; 4]);
        let shared = a.fork();
        let enclosing = a.push(1);
        let nested = a.push(2);
        *a.get_mut(nested) = 3;
        a.escape_own();
        assert!(a.seal(nested) && !a.seal(nested), "a record is sealed once");
        *a.get_mut(enclosing) = 4;
        assert!(a.writable(enclosing), "the enclosing record is finished after the nested one's publication");
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| *a.get_mut(nested) = 5));
        assert!(refused.is_err(), "a change to a sealed record is refused, no peer having read it");
        assert!(!shared.marks.peer_read.has(nested));
        let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a.writable(nested)));
        assert!(asked.is_err(), "a sealed record answers writable with the refusal");
        let peer = Arena::attach(shared.clone(), 1);
        assert_eq!(*peer.get(nested), 3, "a peer reads the sealed record");
        let early = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            shared.marks.peers_read_sealed.store(true, Ordering::Relaxed);
            *peer.get(enclosing)
        }));
        assert!(early.is_err(), "a peer's read of an escaped record no publication sealed is refused where reads need the seal");
    }

    #[test]
    fn a_missing_record_of_one_workers_arena_is_refused() {
        let a: Arena<u32> = Arena::from_vec(vec![7]);
        assert_eq!(*a.get(0), 7);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| *a.get(1))).is_err());
    }

    #[test]
    fn a_claim_outlives_the_records_growth_and_publication() {
        let cells = Cells::new();
        let arena: SharedArena<u32> = SharedArena::from_vec((0..8).collect());
        assert!(cells.claim(5));
        for i in 8..5000 {
            arena.overrides.push(AtomicPtr::new(std::ptr::null_mut()));
            arena.records.push(i);
        }
        arena.publish_all();
        arena.publish(5, Box::new(55));
        assert!(!cells.claim(5));
        assert!(cells.mine(5));
        assert_eq!(*arena.get(5), 55);
        assert_eq!(cells.get(5), Completion::InProgress);
    }

    #[test]
    fn attached_cells_share_the_region_and_own_their_workers() {
        let a = Cells::new();
        let b = a.attach(1);
        assert!(a.claim(7));
        assert!(!b.claim(7));
        assert_eq!(b.get(7), Completion::InProgress);
        let own = worker_base(1) + 3;
        assert!(b.claim(own));
        assert!(b.mine(own));
        assert_eq!(b.get(own + 1), Completion::NotStarted);
        b.set(own, Completion::Done);
        assert_eq!(b.get(own), Completion::Done);
        assert_eq!(a.get(own), Completion::Done);
        a.carry(1, &[u32::MAX, u32::MAX, u32::MAX, 40]);
        assert_eq!(a.get(40), Completion::Done);
        assert_eq!(b.get(own), Completion::NotStarted);
    }

    #[test]
    fn a_directory_is_cleared_for_its_next_store() {
        let cells = Cells::new();
        cells.set(5, Completion::Done);
        assert!(cells.claim(CELL_CHUNK as u32 * 3 + 1));
        let directory = cells.directory.as_ref().unwrap();
        assert_eq!(directory.held(), 2 * std::mem::size_of::<CellChunk>());
        directory.clear();
        assert_eq!(cells.get(5), Completion::NotStarted);
        assert_eq!(cells.get(CELL_CHUNK as u32 * 3 + 1), Completion::NotStarted);
        assert_eq!(directory.held(), 2 * std::mem::size_of::<CellChunk>());
    }

    #[test]
    fn a_chunk_is_made_once_under_contention() {
        let cells = Arc::new(Cells::new());
        let threads: Vec<_> = (0..8u32).map(|k| { let c = cells.clone(); std::thread::spawn(move || c.claim(4096 * 3 + k)) }).collect();
        assert!(threads.into_iter().all(|t| t.join().unwrap()));
        for k in 0..8 {
            assert_eq!(cells.get(4096 * 3 + k), Completion::InProgress);
        }
    }

    /// Eight threads set bits of the shared region and of every worker's range at once while
    /// the chunks they fall in are being made: no bit is lost, and every thread reads every
    /// other's. With the words in a vector that one thread grows by copying, a bit set in the
    /// old words while the copy is made is lost.
    #[test]
    fn bits_set_at_once_by_many_threads_all_stand() {
        let bits = Arc::new(SharedBits::new());
        let ids = |k: u32| (0..20_000u32).map(move |n| if n % 2 == 0 { n * 37 + k } else { worker_base((n % 4) as usize) + n * 13 + k });
        let start = Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8u32)
            .map(|k| {
                let (bits, start) = (bits.clone(), start.clone());
                std::thread::spawn(move || {
                    start.wait();
                    for i in ids(k) {
                        bits.set(i);
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        for k in 0..8 {
            assert!(ids(k).all(|i| bits.has(i)), "a bit set by thread {} was lost", k);
        }
        assert!(!bits.has(worker_base(7) + 5));
    }

    /// A pseudo file's entry pushed under the loader's lock is the holder's until the release,
    /// and one changed later is a copy that replaces it: a reader holding the entry as it was
    /// reads it whole.
    #[test]
    fn a_pseudo_files_entry_is_staged_then_replaced_by_a_copy() {
        let lock = Arc::new(crate::shared::ReentrantLock::new());
        let mut table: FileVec<String> = FileVec::from_vec(vec!["a".to_string()]);
        table.fork();
        let other = table.attach(1);
        lock.lock();
        table.lock_taken();
        table.push("b".to_string());
        assert_eq!(table[1], "b");
        let seen = std::thread::scope(|s| s.spawn(|| other.ext.as_ref().unwrap().shared().published()).join().unwrap());
        assert_eq!(seen, 0, "a staged entry read by another thread");
        table.publish_staged();
        table.flush();
        lock.unlock();
        let before: *const String = &other[1];
        lock.lock();
        table.lock_taken();
        table[1].push_str("c");
        table.publish_staged();
        table.flush();
        lock.unlock();
        assert_eq!(unsafe { &*before }, "b", "the published entry was changed in place");
        assert_eq!(other[1], "bc");
    }

    #[test]
    fn local_ids_stay_apart_from_shared_growth() {
        let mut a: Arena<u32> = Arena::new();
        for i in 0..10 {
            a.push(i);
        }
        let shared = a.fork();
        let local = a.push(100);
        assert_eq!(local, LOCAL_BASE);
        assert_eq!(*a.get(local), 100);
        // The shared region grows past the old length: a local id is still local.
        for i in 10..100 {
            shared.overrides.push(AtomicPtr::new(std::ptr::null_mut()));
            shared.records.push(i);
        }
        shared.publish_all();
        assert_eq!(*a.get(local), 100);
        assert_eq!(*a.get(50), 50);
        // Another worker's ids are its own range: the two never meet, however far the
        // shared region grows.
        let mut b: Arena<u32> = Arena::attach(shared.clone(), 1);
        let other = b.push(7);
        assert_eq!(other, worker_base(1));
        assert_eq!(worker_of(other), Some(1));
        assert_eq!(worker_of(local), Some(0));
        assert_eq!(worker_of(50), None);
        assert_eq!(*b.get(3), 3);
        assert_eq!(*b.get(other), 7);
    }
}

// What the stores hold on the heap, for a session's `stats` (`src/held.rs`): the arrays of a
// worker's own records and, once, the shared region's.

impl<T> SharedArena<T> {
    pub fn held(&self) -> usize {
        self.records.held() + self.prefix_overrides.len() * std::mem::size_of::<AtomicPtr<T>>() + self.overrides.held()
    }
}

impl<T> Arena<T> {
    pub fn held(&self) -> usize {
        crate::held::array(&self.plain) + self.own.held() + self.shared.as_ref().map_or(0, |s| s.held())
    }
}

impl<T: Copy + Send + Sync> Parallel<T> {
    pub fn held(&self) -> usize {
        crate::held::array(&self.own) + self.shared.as_ref().map_or(0, |s| s.held())
    }
}

impl Bits {
    pub fn held(&self) -> usize {
        crate::held::array(&self.own) + self.shared.as_ref().map_or(0, |s| s.held())
    }
}

impl<T> FileVec<T> {
    pub fn held(&self) -> usize {
        crate::held::array(&self.own) + self.ext.as_ref().map_or(0, |e| e.held()) + self.late.as_ref().map_or(0, |l| l.held())
    }
}

impl<K, V> SharedMap<K, V> {
    pub fn held(&self) -> usize {
        self.entries.held() + self.index.held()
    }
}

impl<K, V> Layered<K, V> {
    pub fn held(&self) -> usize {
        crate::held::table(&self.local) + crate::held::table(&self.pending) + self.shared.as_ref().map_or(0, |s| s.held())
    }
}
