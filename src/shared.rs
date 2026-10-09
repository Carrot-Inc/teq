//! The structures the parallel typer's workers share by reference: append-only stores whose
//! readers take no lock and whose writers take one.
//!
//! `SlabVec` is a vector that never moves a published entry: a grown one keeps its old buffer
//! until it is dropped, so a reference handed out stays valid, and readers load the buffer's
//! base with every access, which after a growth is the new buffer or an old one holding the
//! same entries. `Table` is an open-addressing index over such a vector (the hash-consing maps
//! of the type store and the interner, the loader's maps): readers probe the table they load,
//! which a resize retires without changing, and a miss is confirmed under the lock before an
//! insert. `Serial` is the writers' lock, skipped while one thread owns the store
//! (`exclusive`), which is the single-worker path's cost: none. `ReentrantLock` is the loader's
//! lock, held across the loader's work by one thread at a time and
//! reentered by it; `lock_depth` says whether this thread holds it, which a wait on a cell asserts
//! against (a wait under the lock is the deadlock the cell graph cannot see).

use std::cell::{Cell, UnsafeCell};
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

/// The writers' lock of a shared store: taken by an insert, unless one thread owns the store.
pub struct Serial {
    lock: Mutex<()>,
    exclusive: AtomicBool,
    /// What a wait for the lock counts as (`measure::Wait`).
    what: crate::measure::Wait,
}

impl Serial {
    pub fn new(exclusive: bool, what: crate::measure::Wait) -> Serial {
        Serial { lock: Mutex::new(()), exclusive: AtomicBool::new(exclusive), what }
    }

    /// Whether inserts go without the lock: one thread owns the store.
    pub fn set_exclusive(&self, exclusive: bool) {
        self.exclusive.store(exclusive, Ordering::Release);
    }

    /// The lock for an insert, or `None` while one thread owns the store: a miss confirmed
    /// before the call stands then, and the caller skips looking again.
    #[inline]
    pub fn hold(&self) -> Option<Held<'_>> {
        if self.exclusive.load(Ordering::Relaxed) {
            None
        } else {
            Some(self.hold_shared())
        }
    }

    /// Whether one thread owns the store.
    #[inline(always)]
    pub fn is_exclusive(&self) -> bool {
        self.exclusive.load(Ordering::Relaxed)
    }

    /// The lock, whoever owns the store.
    pub fn lock(&self) -> Held<'_> {
        self.hold_shared()
    }

    fn hold_shared(&self) -> Held<'_> {
        let guard = match self.lock.try_lock() {
            Ok(g) => g,
            Err(_) => self.hold_waiting(),
        };
        Held { guard: Some(guard), since: crate::measure::serial_hold_begin(self.what) }
    }

    /// The lock taken by another thread: the wait is counted (`measure`).
    #[cold]
    #[inline(never)]
    fn hold_waiting(&self) -> MutexGuard<'_, ()> {
        let start = crate::measure::wait_begin();
        let g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        crate::measure::wait_end(start, self.what, 0);
        g
    }
}

/// A `Serial`'s lock held for an insert; with `TEQ_WORKERS_TYPES=1` its hold is timed
/// (`measure::serial_held`).
pub struct Held<'a> {
    guard: Option<MutexGuard<'a, ()>>,
    since: Option<(std::time::Instant, crate::measure::Wait)>,
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        // The lock is given up first: the hold's time takes in the release.
        drop(self.guard.take());
        if let Some((start, what)) = self.since {
            crate::measure::serial_held(what, start);
        }
    }
}

/// A vector whose entries never move: readers take no lock and keep their references across
/// growth. Pushes are the writer's, serialised by the owner (`Serial`); `len` is published
/// after the entry is written.
pub struct SlabVec<T> {
    base: AtomicPtr<T>,
    cap: UnsafeCell<usize>,
    len: AtomicUsize,
    retired: UnsafeCell<Vec<(*mut T, usize)>>,
    /// The system's mapping the entries moved into (`map_regions`), none while the buffer is
    /// the allocator's; its regions share it, and the pages go when the last of them does.
    mapped: UnsafeCell<Option<std::sync::Arc<Mapping>>>,
    /// The bytes of the mapping from its start that the vector's own pushes may write, committed
    /// (Windows', whose mapping is reserved address space: `commit_ahead`); unbounded for the
    /// allocator's buffer.
    #[cfg(windows)]
    committed: UnsafeCell<usize>,
}

unsafe impl<T: Send + Sync> Sync for SlabVec<T> {}
unsafe impl<T: Send> Send for SlabVec<T> {}

impl<T> SlabVec<T> {
    pub fn with_capacity(cap: usize) -> SlabVec<T> {
        let cap = cap.max(1);
        SlabVec {
            base: AtomicPtr::new(Self::buffer(cap)),
            cap: UnsafeCell::new(cap),
            len: AtomicUsize::new(0),
            retired: UnsafeCell::new(Vec::new()),
            mapped: UnsafeCell::new(None),
            #[cfg(windows)]
            committed: UnsafeCell::new(usize::MAX),
        }
    }

    pub fn from_vec(mut v: Vec<T>) -> SlabVec<T> {
        if v.capacity() == 0 {
            return SlabVec::with_capacity(16);
        }
        let (ptr, len, cap) = (v.as_mut_ptr(), v.len(), v.capacity());
        std::mem::forget(v);
        SlabVec {
            base: AtomicPtr::new(ptr),
            cap: UnsafeCell::new(cap),
            len: AtomicUsize::new(len),
            retired: UnsafeCell::new(Vec::new()),
            mapped: UnsafeCell::new(None),
            #[cfg(windows)]
            committed: UnsafeCell::new(usize::MAX),
        }
    }

    fn buffer(cap: usize) -> *mut T {
        let mut v: Vec<MaybeUninit<T>> = Vec::with_capacity(cap);
        let ptr = v.as_mut_ptr() as *mut T;
        std::mem::forget(v);
        ptr
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    /// The entry `i`, which has been published: one of the vector's own, or, once the entries
    /// are mapped (`map_regions`), one a region's writer published.
    #[inline]
    pub fn get(&self, i: usize) -> &T {
        debug_assert!(i < self.len() || self.mapped_bytes() > i * std::mem::size_of::<T>(), "slab index {} past the length {}", i, self.len());
        unsafe { &*self.read_base().add(i) }
    }

    /// The entries published so far, as a slice of the current buffer.
    pub fn as_slice(&self) -> &[T] {
        let len = self.len();
        unsafe { std::slice::from_raw_parts(self.read_base(), len) }
    }

    /// The buffer's base for a read of published entries. A reader that took the length (or
    /// the id) before a growth and the base after it reads the new buffer's copy of an entry,
    /// which what it took does not order; the growth's release of the base does, to a load
    /// that the entry's read depends on. The base is loaded with consume ordering: on AArch64
    /// and x86-64 a plain load, the entry's address computed from it keeping the read of the
    /// entry after it (as crossbeam's `load_consume` and the Linux kernel's `rcu_dereference`
    /// do). An acquiring load there costs an instruction more at every read of the type store.
    #[inline(always)]
    fn read_base(&self) -> *mut T {
        #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
        {
            self.base.load(Ordering::Relaxed)
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            self.base.load(Ordering::Acquire)
        }
    }

    /// Appends an entry: the writer's, under the owner's lock.
    #[inline(always)]
    pub fn push(&self, value: T) -> usize {
        let len = self.len.load(Ordering::Relaxed);
        unsafe {
            let cap = *self.cap.get();
            if len == cap {
                self.grow(cap * 2, len);
            }
            #[cfg(windows)]
            if (len + 1) * std::mem::size_of::<T>() > *self.committed.get() {
                commit_ahead(self.base.load(Ordering::Relaxed) as *mut u8, &mut *self.committed.get(), (len + 1) * std::mem::size_of::<T>(), cap * std::mem::size_of::<T>());
            }
            self.base.load(Ordering::Relaxed).add(len).write(value);
        }
        self.len.store(len + 1, Ordering::Release);
        len
    }

    /// The entry `i` for the writer's mutation, before it is published to other threads.
    pub unsafe fn get_mut(&self, i: usize) -> &mut T {
        &mut *self.base.load(Ordering::Relaxed).add(i)
    }

    /// The entries `from..to` for the writer's mutation, none of them published.
    pub unsafe fn slice_mut(&self, from: usize, to: usize) -> &mut [T] {
        std::slice::from_raw_parts_mut(self.base.load(Ordering::Relaxed).add(from), to - from)
    }

    unsafe fn grow(&self, new_cap: usize, len: usize) {
        if (*self.mapped.get()).is_some() {
            Self::past_the_mapping(len);
        }
        let old = self.base.load(Ordering::Relaxed);
        let new = Self::buffer(new_cap);
        std::ptr::copy_nonoverlapping(old, new, len);
        crate::shake::point(crate::shake::Point::Grown);
        (*self.retired.get()).push((old, *self.cap.get()));
        *self.cap.get() = new_cap;
        self.base.store(new, Ordering::Release);
    }

    #[cold]
    #[inline(never)]
    fn past_the_mapping(len: usize) -> ! {
        panic!("a mapped store ran past its {} entries", len)
    }

    /// Moves the entries into `entries` slots of zeroed pages mapped from the system, which the
    /// vector's own pushes fill up to `own` of (its capacity from here on: a push past it is an
    /// error, never a move) and the regions past `own` are other writers' (`Region`), read by
    /// `get` as the vector's are, by their position in the one mapping. The pages are taken
    /// from the system as they are written. `false`, and nothing changed, when the system
    /// gives no mapping. The owner's, before any reader of the regions exists.
    pub fn map_regions(&self, entries: usize, own: usize) -> bool {
        match Mapping::new(Self::mapping_bytes(entries)) {
            Some(m) => {
                self.adopt(m, own);
                true
            }
            None => false,
        }
    }

    /// The bytes of a mapping of `entries` slots.
    pub fn mapping_bytes(entries: usize) -> usize {
        entries * std::mem::size_of::<T>().max(1)
    }

    /// Moves the entries into the mapping `m` (`map_regions`), which the vector holds from here
    /// with the regions it makes.
    pub fn adopt(&self, m: Mapping, own: usize) {
        unsafe {
            debug_assert!((*self.mapped.get()).is_none());
            let at = m.at as *mut T;
            let old = self.base.load(Ordering::Relaxed);
            #[cfg(windows)]
            {
                *self.committed.get() = 0;
                let len = self.len() * std::mem::size_of::<T>();
                commit_ahead(m.at, &mut *self.committed.get(), len, own * std::mem::size_of::<T>());
            }
            std::ptr::copy_nonoverlapping(old, at, self.len());
            (*self.retired.get()).push((old, *self.cap.get()));
            *self.cap.get() = own;
            *self.mapped.get() = Some(std::sync::Arc::new(m));
            self.base.store(at, Ordering::Release);
        }
    }

    fn mapped_bytes(&self) -> usize {
        unsafe { (*self.mapped.get()).as_ref().map_or(0, |m| m.bytes) }
    }

    /// Whether the entries live in a mapping (`map_regions`).
    pub fn is_mapped(&self) -> bool {
        unsafe { (*self.mapped.get()).is_some() }
    }

    /// The region of `cap` slots from slot `start` of the mapping, for one writer of its own. It
    /// holds the mapping with the vector, so that either may go first: a region's entries are
    /// dropped by the region, the vector's by the vector, and the pages by the last of them.
    pub fn region(&self, start: usize, cap: usize) -> Region<T> {
        let pages = unsafe { (*self.mapped.get()).clone() }.expect("a region of a vector not mapped");
        assert!((start + cap) * std::mem::size_of::<T>().max(1) <= pages.bytes, "a region outside the mapping");
        Region {
            at: unsafe { self.base.load(Ordering::Relaxed).add(start) },
            len: AtomicUsize::new(0),
            cap,
            _pages: pages,
            #[cfg(windows)]
            committed: UnsafeCell::new(0),
        }
    }

    /// Takes the entries out as a vector, for the single-threaded phases after a shared one.
    pub fn into_vec(self) -> Vec<T> {
        if self.is_mapped() {
            let len = self.len();
            let mut out = Vec::with_capacity(len);
            unsafe {
                std::ptr::copy_nonoverlapping(self.base.load(Ordering::Relaxed), out.as_mut_ptr(), len);
                out.set_len(len);
            }
            self.len.store(0, Ordering::Relaxed);
            return out;
        }
        let mut this = std::mem::ManuallyDrop::new(self);
        let len = *this.len.get_mut();
        let base = *this.base.get_mut();
        let cap = *this.cap.get_mut();
        for (ptr, cap) in this.retired.get_mut().drain(..) {
            drop(unsafe { Vec::from_raw_parts(ptr as *mut MaybeUninit<T>, 0, cap) });
        }
        drop(std::mem::take(this.retired.get_mut()));
        unsafe { Vec::from_raw_parts(base, len, cap) }
    }
}

impl<T> SlabVec<T> {
    /// Room for `n` entries at least, taken once: a store sized from an earlier build's count
    /// grows through no doubling and retires nothing but its first buffer.
    pub fn reserve(&self, n: usize) {
        unsafe {
            if *self.cap.get() < n && (*self.mapped.get()).is_none() {
                self.grow(n.next_power_of_two(), self.len());
            }
        }
    }

    /// Frees the buffers outgrown, for a moment when no reader holds a reference into them:
    /// between two typing steps, once the workers are joined.
    pub unsafe fn release_retired(&self) {
        for (ptr, cap) in (*self.retired.get()).drain(..) {
            drop(Vec::from_raw_parts(ptr as *mut MaybeUninit<T>, 0, cap));
        }
    }

    /// The bytes of the buffers outgrown, which live as long as the vector or until
    /// `release_retired`.
    pub fn retired_bytes(&self) -> usize {
        unsafe { (*self.retired.get()).iter().map(|&(_, cap)| cap).sum::<usize>() * std::mem::size_of::<T>() }
    }

    /// The bytes of the buffer and of the retired ones, which live as long as the vector; of a
    /// mapping, the entries' own (the regions' are their writers' to count).
    pub fn held(&self) -> usize {
        unsafe {
            let own = if (*self.mapped.get()).is_some() { self.len() } else { *self.cap.get() };
            (own + (*self.retired.get()).iter().map(|&(_, cap)| cap).sum::<usize>()) * std::mem::size_of::<T>()
        }
    }
}

impl<T> Drop for SlabVec<T> {
    fn drop(&mut self) {
        let len = *self.len.get_mut();
        let base = *self.base.get_mut();
        let cap = *self.cap.get_mut();
        if let Some(pages) = self.mapped.get_mut().take() {
            unsafe { std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(base, len)) };
            drop(pages);
        } else {
            drop(unsafe { Vec::from_raw_parts(base, len, cap) });
        }
        for (ptr, cap) in self.retired.get_mut().drain(..) {
            drop(unsafe { Vec::from_raw_parts(ptr as *mut MaybeUninit<T>, 0, cap) });
        }
    }
}

/// Zeroed pages taken from the system for a `SlabVec` (`SlabVec::adopt`), given back when
/// dropped: unadopted, when several vectors mapped together take their mappings first and adopt
/// them only once every one is taken, so that a refusal leaves nothing taken and no vector
/// changed; adopted, when the vector and its regions are gone.
pub struct Mapping {
    at: *mut u8,
    bytes: usize,
}

unsafe impl Send for Mapping {}
unsafe impl Sync for Mapping {}

#[cfg(test)]
thread_local! {
    /// The mapping a test makes the system refuse, counted from 0 across this thread's asks.
    static REFUSE_AT: Cell<Option<usize>> = const { Cell::new(None) };
    /// The bytes this thread's mappings hold, taken less given back.
    static MAPPED_HERE: Cell<isize> = const { Cell::new(0) };
}

/// Makes the `n`th mapping this thread asks for from here a refusal (a test's).
#[cfg(test)]
pub fn refuse_mapping(n: Option<usize>) {
    REFUSE_AT.with(|r| r.set(n));
}

/// The bytes this thread's mappings hold (a test's).
#[cfg(test)]
pub fn mapped_here() -> isize {
    MAPPED_HERE.with(|m| m.get())
}

impl Mapping {
    pub fn new(bytes: usize) -> Option<Mapping> {
        #[cfg(test)]
        {
            let refuse = REFUSE_AT.with(|r| match r.get() {
                Some(0) => {
                    r.set(None);
                    true
                }
                Some(n) => {
                    r.set(Some(n - 1));
                    false
                }
                None => false,
            });
            if refuse {
                return None;
            }
        }
        #[cfg(not(windows))]
        let at = crate::alloc::zeroed_pages(bytes);
        #[cfg(windows)]
        let at = crate::alloc::reserve_pages(bytes);
        if at.is_null() {
            return None;
        }
        #[cfg(test)]
        MAPPED_HERE.with(|m| m.set(m.get() + bytes as isize));
        Some(Mapping { at, bytes })
    }

    unsafe fn release(at: *mut u8, bytes: usize) {
        #[cfg(not(windows))]
        crate::alloc::free_zeroed_pages(at, bytes);
        #[cfg(windows)]
        {
            // A release takes the whole reservation, whatever its size.
            let _ = bytes;
            crate::alloc::unreserve_pages(at);
        }
        #[cfg(test)]
        MAPPED_HERE.with(|m| m.set(m.get() - bytes as isize));
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe { Mapping::release(self.at, self.bytes) };
    }
}

/// On Windows a mapping is reserved address space (`alloc::reserve_pages`), which its writers
/// commit ahead of their writes, a step at a time, as Linux and macOS take a page of theirs on
/// its first write: of the `limit` bytes from `at` a writer may write, `*committed` are, and the
/// bytes up to `need` are after the call. The commit limit's refusal ends the build, as the
/// system's out of memory does elsewhere.
#[cfg(windows)]
#[cold]
#[inline(never)]
unsafe fn commit_ahead(at: *mut u8, committed: &mut usize, need: usize, limit: usize) {
    const STEP: usize = 1 << 20;
    let to = need.div_ceil(STEP).saturating_mul(STEP).min(limit).max(need);
    if to <= *committed {
        return;
    }
    if !crate::alloc::commit_pages(at.add(*committed), to - *committed) {
        panic!("the system's commit limit refused {} bytes more of the type store", to - *committed);
    }
    *committed = to;
}

/// A run of slots of a mapped `SlabVec` (`SlabVec::region`) that one writer appends to without
/// a lock, the vector's readers reading them by their position in the mapping: a worker's
/// overlay of the type store. The region drops what it holds and keeps the pages until then.
pub struct Region<T> {
    at: *mut T,
    len: AtomicUsize,
    cap: usize,
    _pages: std::sync::Arc<Mapping>,
    /// The bytes from `at` committed, the one writer's (`commit_ahead`).
    #[cfg(windows)]
    committed: UnsafeCell<usize>,
}

unsafe impl<T: Send + Sync> Sync for Region<T> {}
unsafe impl<T: Send> Send for Region<T> {}

impl<T> Region<T> {
    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    /// Appends an entry: the region's one writer.
    #[inline]
    pub fn push(&self, value: T) -> usize {
        let len = self.len.load(Ordering::Relaxed);
        if len + 1 >= self.cap {
            Self::full(self.cap);
        }
        #[cfg(windows)]
        unsafe {
            if (len + 1) * std::mem::size_of::<T>() > *self.committed.get() {
                commit_ahead(self.at as *mut u8, &mut *self.committed.get(), (len + 1) * std::mem::size_of::<T>(), self.cap * std::mem::size_of::<T>());
            }
        }
        unsafe { self.at.add(len).write(value) };
        self.len.store(len + 1, Ordering::Release);
        len
    }

    #[cold]
    #[inline(never)]
    fn full(cap: usize) -> ! {
        panic!("a worker made more than {} entries of one kind of the type store", cap - 1)
    }

    /// The entry at `i` of the region, which its writer published.
    #[inline]
    pub fn get(&self, i: usize) -> &T {
        debug_assert!(i < self.len());
        unsafe { &*self.at.add(i) }
    }

    /// The bytes the entries take.
    pub fn held(&self) -> usize {
        self.len() * std::mem::size_of::<T>()
    }
}

impl<T> Drop for Region<T> {
    fn drop(&mut self) {
        let len = *self.len.get_mut();
        // The pages are this region's to keep until the field drops, after the entries; the ones
        // its entries took go back to the system now, the region's slots no reader's from here.
        unsafe {
            std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(self.at, len));
            let size = std::mem::size_of::<T>();
            crate::alloc::release_pages(self.at as *mut u8, len * size, self.cap * size);
        }
    }
}

impl<T> std::ops::Index<usize> for SlabVec<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        self.get(i)
    }
}

/// An open-addressing index from a hash to the position of an entry in a `SlabVec`: readers
/// probe without a lock, the writer inserts under the owner's lock and replaces the table when
/// it is half full, keeping the old one for the readers still probing it. A probe starts at the
/// top bits of the hash multiplied once more (`mixed`: the low bits of a multiplicative hash are
/// a function of a few input bits, and its top bits keep the arithmetic of consecutive keys,
/// either of which makes runs under linear probing), and compares the entry only where a tag of
/// seven other bits agrees. The writer keeps the top half of each position's mixed hash, which
/// is what a resize places the entries by, without reading and hashing them again.
pub struct Table {
    current: AtomicPtr<TableBuf>,
    count: UnsafeCell<usize>,
    /// Per position, the top 32 bits of its entry's mixed hash: the writer's alone.
    hashes: UnsafeCell<Vec<u32>>,
    retired: UnsafeCell<Vec<Box<TableBuf>>>,
}

unsafe impl Sync for Table {}
unsafe impl Send for Table {}

struct TableBuf {
    mask: usize,
    /// How far a mixed hash is shifted down for its first slot: its top `log2(cap)` bits.
    shift: u32,
    /// Per slot the entry's position plus one, `cap` of them: written before the slot's tag
    /// and read after it, so a slot whose tag is 0 is never read, and the memory starts
    /// unwritten (the tags are the zeroing a table costs: a byte per slot).
    slots: *mut AtomicU32,
    /// Per slot the tag of its entry's hash; 0 for an empty slot.
    tags: Box<[AtomicU8]>,
}

impl TableBuf {
    fn new(cap: usize) -> Box<TableBuf> {
        let mut slots: Vec<MaybeUninit<AtomicU32>> = Vec::with_capacity(cap);
        let slots_ptr = slots.as_mut_ptr() as *mut AtomicU32;
        std::mem::forget(slots);
        // Zeroed memory is a zeroed tag table: an `AtomicU8` is a `u8`.
        let zeroed: Box<[u8]> = vec![0u8; cap].into_boxed_slice();
        let tags: Box<[AtomicU8]> = unsafe { Box::from_raw(Box::into_raw(zeroed) as *mut [AtomicU8]) };
        Box::new(TableBuf { mask: cap - 1, shift: 64 - cap.trailing_zeros(), slots: slots_ptr, tags })
    }

    #[inline]
    fn first(&self, mixed: u64) -> usize {
        (mixed >> self.shift) as usize
    }

    #[inline]
    fn tag(&self, i: usize) -> &AtomicU8 {
        unsafe { self.tags.get_unchecked(i) }
    }

    /// The slot `i`, whose tag was seen set.
    #[inline]
    fn slot(&self, i: usize) -> &AtomicU32 {
        unsafe { &*self.slots.add(i) }
    }

    /// Writes the slot `i`, whose tag is 0 and is set after this.
    #[inline]
    fn write_slot(&self, i: usize, s: u32) {
        unsafe { self.slots.add(i).write(AtomicU32::new(s)) }
    }
}

impl Drop for TableBuf {
    fn drop(&mut self) {
        drop(unsafe { Vec::from_raw_parts(self.slots as *mut MaybeUninit<AtomicU32>, 0, self.mask + 1) });
    }
}

/// The hash as the table reads it: multiplied by an odd constant near the golden ratio's, so that
/// the top bits spread keys that came out of `FxHasher` as consecutive multiples of its own
/// constant (two lockstep families of ids, `Var(n)` beside `Class(c, n + k)`, landed in runs
/// of 80 slots on the `gview_1` variant; 0.1 extra probes per insert with the multiply).
#[inline]
fn mixed(hash: u64) -> u64 {
    hash.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Seven bits of the mixed hash below the ones a slot is chosen by (in a table of up to 2^25
/// slots), with the top bit set so that no entry's tag is an empty slot's. They are in the top
/// half the writer keeps (`Table::hashes`).
#[inline]
fn tag_of(mixed: u64) -> u8 {
    (mixed >> 32) as u8 | 0x80
}

impl Table {
    pub fn with_capacity(entries: usize) -> Table {
        let cap = (entries * 2).max(64).next_power_of_two();
        Table { current: AtomicPtr::new(Box::into_raw(TableBuf::new(cap))), count: UnsafeCell::new(0), hashes: UnsafeCell::new(Vec::with_capacity(entries)), retired: UnsafeCell::new(Vec::new()) }
    }

    /// The position of the entry with this hash that `matches`, if one is indexed.
    #[inline(always)]
    pub fn find(&self, hash: u64, matches: impl Fn(u32) -> bool) -> Option<u32> {
        let buf = unsafe { &*self.current.load(Ordering::Acquire) };
        let hash = mixed(hash);
        let tag = tag_of(hash);
        let mut i = buf.first(hash);
        loop {
            let t = buf.tag(i).load(Ordering::Acquire);
            if t == 0 {
                return None;
            }
            if t == tag {
                let s = buf.slot(i).load(Ordering::Acquire);
                if matches(s - 1) {
                    return Some(s - 1);
                }
            }
            i = (i + 1) & buf.mask;
        }
    }

    /// Indexes the entry at `pos` under `hash`: the writer's, under the owner's lock, after the
    /// entry is published.
    #[inline]
    pub fn insert(&self, hash: u64, pos: u32) {
        unsafe {
            let count = &mut *self.count.get();
            let mut buf = &*self.current.load(Ordering::Relaxed);
            if (*count + 1) * 2 > buf.mask + 1 {
                self.resize((buf.mask + 1) * 2);
                buf = &*self.current.load(Ordering::Relaxed);
            }
            let mixed = mixed(hash);
            self.keep_hash(pos, mixed);
            Self::place(buf, mixed, pos + 1);
            *count += 1;
        }
    }

    /// Records the top half of the mixed hash of the entry at `pos`: the writer's.
    #[inline]
    unsafe fn keep_hash(&self, pos: u32, mixed: u64) {
        let hashes = &mut *self.hashes.get();
        let (pos, top) = (pos as usize, (mixed >> 32) as u32);
        if pos == hashes.len() {
            hashes.push(top);
        } else {
            Self::keep_hash_apart(hashes, pos, top);
        }
    }

    /// A hash kept for a position other than the next: one replaced, or past a gap of entries
    /// the table does not index.
    #[cold]
    #[inline(never)]
    fn keep_hash_apart(hashes: &mut Vec<u32>, pos: usize, top: u32) {
        if pos >= hashes.len() {
            hashes.resize(pos + 1, 0);
        }
        hashes[pos] = top;
    }

    /// A table for `entries` at least (`with_capacity`'s size), taken once: the writer's.
    pub fn reserve(&self, entries: usize) {
        let cap = (entries * 2).max(64).next_power_of_two();
        unsafe {
            if cap > (*self.current.load(Ordering::Relaxed)).mask + 1 {
                self.resize(cap);
            }
            let hashes = &mut *self.hashes.get();
            hashes.reserve(entries.saturating_sub(hashes.len()));
        }
    }

    /// The entries placed anew in a table of `cap` slots by the hashes kept for them, the old
    /// table kept for its readers.
    #[inline(never)]
    unsafe fn resize(&self, cap: usize) {
        let buf = &*self.current.load(Ordering::Relaxed);
        let hashes = &*self.hashes.get();
        let bigger = TableBuf::new(cap);
        for i in 0..=buf.mask {
            if buf.tag(i).load(Ordering::Relaxed) != 0 {
                let s = buf.slot(i).load(Ordering::Relaxed);
                Self::place(&bigger, (*hashes.get_unchecked(s as usize - 1) as u64) << 32, s);
            }
        }
        let old = self.current.swap(Box::into_raw(bigger), Ordering::AcqRel);
        (*self.retired.get()).push(Box::from_raw(old));
    }

    /// Points the slot of the entry with this hash that `matches` at `pos` instead: a
    /// replacement, which readers see as the old entry or the new one. Whether one was found.
    pub fn update(&self, hash: u64, matches: impl Fn(u32) -> bool, pos: u32) -> bool {
        let buf = unsafe { &*self.current.load(Ordering::Acquire) };
        let hash = mixed(hash);
        let tag = tag_of(hash);
        let mut i = buf.first(hash);
        loop {
            let t = buf.tag(i).load(Ordering::Acquire);
            if t == 0 {
                return false;
            }
            if t == tag {
                let s = buf.slot(i).load(Ordering::Acquire);
                if matches(s - 1) {
                    unsafe { self.keep_hash(pos, hash) };
                    buf.slot(i).store(pos + 1, Ordering::Release);
                    return true;
                }
            }
            i = (i + 1) & buf.mask;
        }
    }

    /// The slot is written before its tag, and a reader takes the tag before the slot, so a
    /// tag seen is a slot written. What places an entry is the top half of its mixed hash.
    #[inline]
    fn place(buf: &TableBuf, mixed: u64, s: u32) {
        let mut i = buf.first(mixed);
        while buf.tag(i).load(Ordering::Relaxed) != 0 {
            i = (i + 1) & buf.mask;
        }
        buf.write_slot(i, s);
        buf.tag(i).store(tag_of(mixed), Ordering::Release);
    }
}

impl Table {
    /// Trades what the two tables index: for a moment when neither has a reader or a writer
    /// (the type store's fork and merge, the workers not started or joined).
    pub unsafe fn exchange(&self, other: &Table) {
        let mine = self.current.load(Ordering::Relaxed);
        self.current.store(other.current.swap(mine, Ordering::AcqRel), Ordering::Release);
        std::ptr::swap(self.count.get(), other.count.get());
        std::ptr::swap(self.hashes.get(), other.hashes.get());
        std::ptr::swap(self.retired.get(), other.retired.get());
    }

    /// Frees the tables outgrown, for a moment when no reader probes them (`SlabVec::release_retired`).
    pub unsafe fn release_retired(&self) {
        (*self.retired.get()).clear();
    }

    /// The bytes of the tables outgrown.
    pub fn retired_bytes(&self) -> usize {
        unsafe { (*self.retired.get()).iter().map(|b| b.mask + 1).sum::<usize>() * 5 }
    }

    /// The bytes of the slots and tags, the retired tables' included, and of the hashes kept.
    pub fn held(&self) -> usize {
        unsafe { ((*self.current.load(Ordering::Relaxed)).mask + 1 + (*self.retired.get()).iter().map(|b| b.mask + 1).sum::<usize>()) * 5 + (*self.hashes.get()).capacity() * 4 }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(*self.current.get_mut()) });
    }
}

#[inline]
pub fn hash_of<K: Hash + ?Sized>(k: &K) -> u64 {
    let mut h = crate::intern::FxHasher::default();
    k.hash(&mut h);
    h.finish()
}

/// The loader's lock's mutex: std's futex protocol (unlocked, locked, contended; a waiter parks until a
/// release wakes one), with a longer spin before each park. Its holds are
/// mostly a few microseconds and nearly every release at twelve and sixteen workers finds threads waiting:
/// a waiter that parks pays a wake on every hand-over, from a CPU gone idle, and often loses the lock again
/// to the releaser or to a thread arriving, where a waiter still spinning takes it at once. Its own, so
/// that the measurement sees each waiter's spins, parks and wakes and each release's waiters: the wait a
/// futex on Linux, a condition elsewhere.
struct ParkingMutex {
    state: AtomicU32,
    /// The threads in `lock_contended`, which a release reports to the measurement.
    waiting: AtomicU32,
    /// How long a waiter spins before it parks, and after each wake; zero, std's short spin alone.
    spin: std::time::Duration,
    #[cfg(not(target_os = "linux"))]
    parked: (Mutex<()>, std::sync::Condvar),
}

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
const CONTENDED: u32 = 2;

/// How a waiter took the loader's lock: on its first spin, or after `parks` parks.
#[derive(Clone, Copy)]
pub struct Taken {
    pub parks: u32,
}

impl ParkingMutex {
    fn new(spin: std::time::Duration) -> ParkingMutex {
        ParkingMutex {
            state: AtomicU32::new(UNLOCKED),
            waiting: AtomicU32::new(0),
            spin,
            #[cfg(not(target_os = "linux"))]
            parked: (Mutex::new(()), std::sync::Condvar::new()),
        }
    }

    #[inline]
    fn try_lock(&self) -> bool {
        self.state.compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }

    #[cold]
    fn lock_contended(&self) -> Taken {
        self.waiting.fetch_add(1, Ordering::Relaxed);
        let mut taken = Taken { parks: 0 };
        let mut state = self.spin_waiting();
        if state == UNLOCKED {
            match self.state.compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => {
                    self.waiting.fetch_sub(1, Ordering::Relaxed);
                    return taken;
                }
                Err(s) => state = s,
            }
        }
        loop {
            if state != CONTENDED && self.state.swap(CONTENDED, Ordering::Acquire) == UNLOCKED {
                self.waiting.fetch_sub(1, Ordering::Relaxed);
                return taken;
            }
            let parked = crate::measure::lock_park_begin();
            self.park();
            crate::measure::lock_park_end(parked);
            taken.parks += 1;
            state = self.spin_waiting();
        }
    }

    fn spin_waiting(&self) -> u32 {
        if self.spin.is_zero() {
            self.spin()
        } else {
            self.spin_for(self.spin)
        }
    }

    /// Spins while the mutex is held, contended or not, for at most `bound`.
    fn spin_for(&self, bound: std::time::Duration) -> u32 {
        let start = std::time::Instant::now();
        loop {
            for _ in 0..32 {
                let state = self.state.load(Ordering::Relaxed);
                if state == UNLOCKED {
                    return state;
                }
                std::hint::spin_loop();
            }
            if start.elapsed() >= bound {
                return self.state.load(Ordering::Relaxed);
            }
        }
    }

    fn spin(&self) -> u32 {
        let mut spin = 100;
        loop {
            let state = self.state.load(Ordering::Relaxed);
            if state != LOCKED || spin == 0 {
                return state;
            }
            std::hint::spin_loop();
            spin -= 1;
        }
    }

    /// Releases the mutex; whether a waiter was woken.
    #[inline]
    fn unlock(&self) -> bool {
        if self.state.swap(UNLOCKED, Ordering::Release) == CONTENDED {
            self.wake();
            return true;
        }
        false
    }

    #[cfg(target_os = "linux")]
    fn park(&self) {
        futex(&self.state, FUTEX_WAIT_PRIVATE, CONTENDED);
    }

    #[cfg(target_os = "linux")]
    #[cold]
    fn wake(&self) {
        futex(&self.state, FUTEX_WAKE_PRIVATE, 1);
    }

    #[cfg(not(target_os = "linux"))]
    fn park(&self) {
        let g = self.parked.0.lock().unwrap_or_else(|e| e.into_inner());
        if self.state.load(Ordering::Relaxed) == CONTENDED {
            drop(self.parked.1.wait(g).unwrap_or_else(|e| e.into_inner()));
        }
    }

    /// Taken after the release's store, the condition's mutex orders the notification after any
    /// waiter's check of the state: one that saw it contended is waiting by then.
    #[cfg(not(target_os = "linux"))]
    #[cold]
    fn wake(&self) {
        let _g = self.parked.0.lock().unwrap_or_else(|e| e.into_inner());
        self.parked.1.notify_one();
    }
}

/// How long a waiter for the loader's lock spins, contended or not, before it parks and after each wake:
/// 50 µs, the bound measured as good as 20 to 200 µs at twelve and sixteen workers, past which a hold is a
/// long one (a jar class's conversion, 1.6 to 1.8 ms) and the waiter parks.
/// `TEQ_LOCK_SPIN_US=<n>`, a diagnostic, sets n µs, and 0 std's short spin alone.
fn spin_bound() -> std::time::Duration {
    static BOUND: std::sync::OnceLock<std::time::Duration> = std::sync::OnceLock::new();
    *BOUND.get_or_init(|| std::time::Duration::from_micros(std::env::var("TEQ_LOCK_SPIN_US").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(50)))
}

#[cfg(target_os = "linux")]
const FUTEX_WAIT_PRIVATE: i32 = 128;
#[cfg(target_os = "linux")]
const FUTEX_WAKE_PRIVATE: i32 = 129;

#[cfg(target_os = "linux")]
fn futex(word: &AtomicU32, op: i32, val: u32) {
    extern "C" {
        fn syscall(num: std::os::raw::c_long, ...) -> std::os::raw::c_long;
    }
    #[cfg(target_arch = "x86_64")]
    const SYS_FUTEX: std::os::raw::c_long = 202;
    #[cfg(target_arch = "aarch64")]
    const SYS_FUTEX: std::os::raw::c_long = 98;
    unsafe {
        syscall(SYS_FUTEX, word as *const AtomicU32, op, val, std::ptr::null::<u8>(), std::ptr::null::<u32>(), 0u32);
    }
}

/// A lock one thread holds across a stretch of work and reenters freely (the loader's).
pub struct ReentrantLock {
    lock: ParkingMutex,
    owner: AtomicUsize,
    depth: Cell<u32>,
}

unsafe impl Sync for ReentrantLock {}
unsafe impl Send for ReentrantLock {}

thread_local! {
    /// This thread's number, from 1; what the locks and the cells know a thread by.
    static THREAD_NUMBER: Cell<usize> = const { Cell::new(0) };
    /// Gives the number back when the thread ends (`NumberReturn`).
    static NUMBER_RETURN: NumberReturn = const { NumberReturn(Cell::new(0)) };
    /// How deeply this thread holds the loader's lock.
    static LOCK_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// The numbers in use: a thread takes the lowest free one on its first ask and gives it back
/// when it ends, so that a resident's builds, each with fresh workers, never run out of the
/// cells' wait slots (`WAITING`, one per number).
static NUMBERS_TAKEN: [AtomicBool; MAX_THREADS] = [const { AtomicBool::new(false) }; MAX_THREADS];

struct NumberReturn(Cell<usize>);

impl Drop for NumberReturn {
    fn drop(&mut self) {
        let n = self.0.get();
        if n != 0 {
            WAITING[n].store(0, Ordering::Release);
            NUMBERS_TAKEN[n].store(false, Ordering::Release);
        }
    }
}

/// The threads' numbers in use.
pub fn numbers_taken() -> usize {
    NUMBERS_TAKEN.iter().filter(|n| n.load(Ordering::Acquire)).count()
}

/// The threads' numbers in use by threads other than this one, which the assertion-enabled build
/// checks a retry's attempt gave back.
#[cfg(debug_assertions)]
pub fn numbers_taken_by_others() -> usize {
    numbers_taken() - THREAD_NUMBER.with(|n| (n.get() != 0) as usize)
}

/// The completions this thread has staged under the loader's lock and not yet published.
pub fn staged_completions() -> usize {
    STAGED_DONE.with(|s| s.borrow().len())
}

/// The threads' numbers taken, this thread's own, how deeply it holds the loader's lock and the
/// completions it has staged, for `TEQ_SESSION_INVENTORY`.
pub fn inventory() -> Vec<(&'static str, usize)> {
    vec![
        ("thread numbers taken", numbers_taken()),
        ("thread number", THREAD_NUMBER.with(|n| n.get())),
        ("lock depth", lock_depth() as usize),
        ("staged completions", staged_completions()),
    ]
}

/// This thread's number, given on the first ask.
pub fn thread_number() -> usize {
    THREAD_NUMBER.with(|n| {
        if n.get() == 0 {
            let mine = (1..MAX_THREADS).find(|&i| !NUMBERS_TAKEN[i].swap(true, Ordering::AcqRel)).expect("more threads than the cells' wait slots");
            n.set(mine);
            NUMBER_RETURN.with(|r| r.0.set(mine));
        }
        n.get()
    })
}

/// How deeply this thread holds a `ReentrantLock`.
#[inline]
pub fn lock_depth() -> u32 {
    LOCK_DEPTH.with(|d| d.get())
}

impl ReentrantLock {
    pub fn new() -> ReentrantLock {
        ReentrantLock::spinning(spin_bound())
    }

    /// The lock whose waiters spin for `spin` before they park.
    fn spinning(spin: std::time::Duration) -> ReentrantLock {
        ReentrantLock { lock: ParkingMutex::new(spin), owner: AtomicUsize::new(0), depth: Cell::new(0) }
    }

    /// Takes the lock, or enters it again when this thread holds it.
    pub fn lock(&self) {
        let me = thread_number();
        if self.owner.load(Ordering::Relaxed) == me {
            self.depth.set(self.depth.get() + 1);
        } else {
            if !self.lock.try_lock() {
                self.lock_waiting();
            }
            self.owner.store(me, Ordering::Relaxed);
            self.depth.set(1);
        }
        LOCK_DEPTH.with(|d| d.set(d.get() + 1));
    }

    /// The lock held by another thread: the wait is counted (`measure`).
    #[cold]
    #[inline(never)]
    fn lock_waiting(&self) {
        let start = crate::measure::wait_begin();
        let taken = self.lock.lock_contended();
        crate::measure::lock_taken_waiting(start, taken);
        crate::measure::wait_end(start, crate::measure::Wait::Loader, 0);
    }

    pub fn unlock(&self) {
        debug_assert_eq!(self.owner.load(Ordering::Relaxed), thread_number());
        LOCK_DEPTH.with(|d| d.set(d.get() - 1));
        let depth = self.depth.get() - 1;
        self.depth.set(depth);
        if depth == 0 {
            self.owner.store(0, Ordering::Relaxed);
            let releasing = crate::measure::lock_release_begin(&self.lock.waiting);
            let woke = self.lock.unlock();
            crate::measure::lock_release_end(releasing, woke);
        }
    }

    /// Whether this thread holds the lock.
    #[cfg(test)]
    fn held_by_me(&self) -> bool {
        self.owner.load(Ordering::Relaxed) == thread_number()
    }
}

/// The state of a cell: a definition's completion, claimed by one
/// thread and read by every other. `InProgress` carries the claimant's thread number, which the
/// wait graph reads.
pub struct CellState(AtomicU32);

const NOT_STARTED: u32 = 0;
const DONE: u32 = 1;
/// From here up: `Done` stored under the loader's lock, `Done` to the thread that stored it
/// and `InProgress` to every other until the records the completion changed are published
/// (`flush_staged_done`, from `Worker::lock_released`).
const STAGED: u32 = 2 + MAX_THREADS as u32;

thread_local! {
    static STAGED_DONE: std::cell::RefCell<Vec<*const CellState>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Every cell this thread completed under the loader's lock `Done` for every thread: after
/// the lock's holder published what the completions changed.
pub fn flush_staged_done() {
    let staged = STAGED_DONE.with(|s| std::mem::take(&mut *s.borrow_mut()));
    if staged.is_empty() {
        return;
    }
    for cell in staged {
        // SAFETY: a cell lives in a chunk that is never moved or freed while its store lives,
        // and the store outlives the lock's hold that staged the cell.
        unsafe { (*cell).0.store(DONE, Ordering::Release) };
    }
    notify();
}

impl CellState {
    #[cfg(test)]
    pub const fn new(c: crate::symbols::Completion) -> CellState {
        use crate::symbols::Completion;
        CellState(AtomicU32::new(match c {
            Completion::NotStarted => NOT_STARTED,
            Completion::Done => DONE,
            // No thread: a record made in progress on the thread that reads it next.
            Completion::InProgress => 2,
        }))
    }

    #[inline]
    pub fn get(&self) -> crate::symbols::Completion {
        use crate::symbols::Completion;
        match self.0.load(Ordering::Acquire) {
            NOT_STARTED => Completion::NotStarted,
            DONE => Completion::Done,
            v => CellState::in_progress(v),
        }
    }

    /// A claimed cell's state to this thread: `Done` when this thread staged its completion.
    #[cold]
    #[inline(never)]
    fn in_progress(v: u32) -> crate::symbols::Completion {
        if v >= STAGED && (v - STAGED) as usize == thread_number() {
            crate::symbols::Completion::Done
        } else {
            crate::symbols::Completion::InProgress
        }
    }

    /// The thread that claimed the cell, while it is in progress: the one whose staged `Done`
    /// is pending as well.
    #[inline]
    pub fn owner(&self) -> Option<usize> {
        match self.0.load(Ordering::Acquire) {
            NOT_STARTED | DONE => None,
            v if v >= STAGED => Some((v - STAGED) as usize),
            v => Some((v - 2) as usize),
        }
    }

    /// Whether this thread holds the claim.
    #[inline]
    pub fn mine(&self) -> bool {
        self.owner() == Some(thread_number())
    }

    pub fn set(&self, c: crate::symbols::Completion) {
        use crate::symbols::Completion;
        let v = match c {
            Completion::NotStarted => NOT_STARTED,
            Completion::Done if lock_depth() > 0 => return self.stage(),
            Completion::Done => DONE,
            Completion::InProgress => 2 + thread_number() as u32,
        };
        self.0.store(v, Ordering::Release);
        if v == DONE {
            notify();
        }
    }

    /// `Done` for a record just made: nothing has read the cell, so nothing waits on it.
    #[inline]
    pub fn done_fresh(&self) {
        self.0.store(DONE, Ordering::Release);
    }

    /// `Done` under the loader's lock: this thread's until the flush.
    #[cold]
    #[inline(never)]
    fn stage(&self) {
        STAGED_DONE.with(|s| s.borrow_mut().push(self as *const CellState));
        self.0.store(STAGED + thread_number() as u32, Ordering::Release);
    }

    /// Claims the cell for this thread: whether it was unclaimed.
    pub fn claim(&self) -> bool {
        self.0.compare_exchange(NOT_STARTED, 2 + thread_number() as u32, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }
}

impl Clone for CellState {
    fn clone(&self) -> CellState {
        CellState(AtomicU32::new(self.0.load(Ordering::Acquire)))
    }
}

impl PartialEq<crate::symbols::Completion> for CellState {
    fn eq(&self, other: &crate::symbols::Completion) -> bool {
        self.get() == *other
    }
}

impl std::fmt::Debug for CellState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

/// The waits of the parallel typer: a thread that finds a cell claimed by another waits here
/// until a publish wakes it. Every thread's `waiting` slot names the
/// cell it waits on, which the cycle check walks before a wait.
static WAITS: Mutex<()> = Mutex::new(());
static WOKEN: std::sync::Condvar = std::sync::Condvar::new();
static WAITING: [std::sync::atomic::AtomicU64; MAX_THREADS] = [const { std::sync::atomic::AtomicU64::new(0) }; MAX_THREADS];
pub const MAX_THREADS: usize = 256;

/// How many threads wait on a cell: `notify` has nobody to wake at zero, which is every
/// completion of a build on one worker.
static WAITERS: AtomicUsize = AtomicUsize::new(0);

/// Wakes the threads that wait on a cell. A waiter counts itself in before it looks at the
/// cell, and the cell is set before this looks at the count, so one of the two sees the
/// other; a waiter polls besides (`wait_until`), which bounds what a miss could cost.
#[inline]
pub fn notify() {
    if WAITERS.load(Ordering::SeqCst) == 0 {
        return;
    }
    notify_waiters();
}

#[cold]
fn notify_waiters() {
    let _g = WAITS.lock().unwrap_or_else(|e| e.into_inner());
    WOKEN.notify_all();
}

/// A cell as the wait graph names it: its kind and id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CellKey(pub u64);

impl CellKey {
    pub fn new(kind: u8, id: u32) -> CellKey {
        CellKey(((kind as u64) << 32) | id as u64 | 1 << 40)
    }
    pub fn kind(self) -> u8 {
        ((self.0 >> 32) & 0xff) as u8
    }
    pub fn id(self) -> u32 {
        self.0 as u32
    }
}

/// The cell thread `t` waits on, if any.
pub fn waiting_on(t: usize) -> Option<CellKey> {
    match WAITING[t].load(Ordering::Acquire) {
        0 => None,
        v => Some(CellKey(v)),
    }
}

/// Waits until `done` holds, this thread's slot naming `cell` meanwhile, unless the wait
/// would close a cycle of the wait graph (`owner_of` names the thread holding a cell): `false`
/// then, and nothing recorded. The check and the edge are one operation under `WAITS`, so that
/// two threads about to wait on each other's cell cannot both check a clean graph, and the
/// check runs again on every wake, since a cycle closes when another thread records its edge.
/// A wait under the loader's lock is an error the caller keeps from happening
/// (`Worker::wait_cell`).
pub fn wait_until(cell: CellKey, owner_of: impl Fn(CellKey) -> Option<usize>, done: impl Fn() -> bool) -> bool {
    assert!(lock_depth() == 0, "a wait on a cell under the loader's lock");
    crate::shake::point(crate::shake::Point::Wait);
    let me = thread_number();
    assert!(me < MAX_THREADS, "more threads than the cells' wait slots");
    let mut g = WAITS.lock().unwrap_or_else(|e| e.into_inner());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut waiting = false;
    while !done() {
        if cycles(me, cell, &owner_of) {
            if waiting {
                WAITERS.fetch_sub(1, Ordering::SeqCst);
                WAITING[me].store(0, Ordering::Release);
            }
            return false;
        }
        if !waiting {
            WAITING[me].store(cell.0, Ordering::Release);
            WAITERS.fetch_add(1, Ordering::SeqCst);
            waiting = true;
        }
        let (guard, r) = WOKEN.wait_timeout(g, std::time::Duration::from_millis(5)).unwrap_or_else(|e| e.into_inner());
        g = guard;
        if r.timed_out() && std::time::Instant::now() > deadline {
            panic!("a wait on a cell of the parallel typer did not end in 120 s: cell {:?}", cell);
        }
    }
    drop(g);
    if waiting {
        WAITERS.fetch_sub(1, Ordering::SeqCst);
        WAITING[me].store(0, Ordering::Release);
    }
    true
}

/// Whether thread `me` waiting on `cell` would close a cycle: the cell's holder waits on a
/// cell whose holder waits ... on a cell `me` holds. Read under `WAITS`.
fn cycles(me: usize, cell: CellKey, owner_of: &impl Fn(CellKey) -> Option<usize>) -> bool {
    let mut at = cell;
    for _ in 0..MAX_THREADS {
        let Some(owner) = owner_of(at) else { return false };
        if owner == me {
            return true;
        }
        let Some(next) = waiting_on(owner) else { return false };
        at = next;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::Completion;
    use std::sync::Arc;

    /// A thread's number goes back when it ends: threads made one after another, more of them
    /// than the wait slots, all take numbers below the slots' count.
    #[test]
    fn a_completion_under_the_lock_is_done_to_its_holder_alone_until_the_flush() {
        use crate::symbols::Completion;
        let cell = Arc::new(CellState::new(Completion::NotStarted));
        let lock = ReentrantLock::new();
        lock.lock();
        assert!(cell.claim());
        cell.set(Completion::Done);
        assert_eq!(cell.get(), Completion::Done);
        let me = thread_number();
        let other = {
            let cell = cell.clone();
            std::thread::spawn(move || (cell.get(), cell.owner())).join().unwrap()
        };
        assert_eq!(other, (Completion::InProgress, Some(me)));
        flush_staged_done();
        lock.unlock();
        let other = {
            let cell = cell.clone();
            std::thread::spawn(move || cell.get()).join().unwrap()
        };
        assert_eq!(other, Completion::Done);
    }

    #[test]
    fn thread_numbers_are_given_back() {
        let mut numbers = Vec::new();
        for _ in 0..MAX_THREADS + 50 {
            numbers.push(std::thread::spawn(thread_number).join().unwrap());
        }
        assert!(numbers.iter().all(|&n| n > 0 && n < MAX_THREADS), "a number past the wait slots");
        let distinct: std::collections::HashSet<usize> = numbers.iter().copied().collect();
        assert!(distinct.len() < 20, "{} distinct numbers for threads made one after another", distinct.len());
    }

    /// Two threads holding a cell each and waiting on the other's: the second to check sees the
    /// first's edge and does not wait, whatever the order; the first wakes when the second's
    /// cell is done.
    #[test]
    fn a_wait_cycle_is_seen_by_one_of_the_two() {
        let cells = Arc::new([CellState::new(Completion::NotStarted), CellState::new(Completion::NotStarted)]);
        let start = Arc::new(std::sync::Barrier::new(2));
        let threads: Vec<_> = (0..2usize)
            .map(|k| {
                let (cells, start) = (cells.clone(), start.clone());
                std::thread::spawn(move || {
                    assert!(cells[k].claim());
                    start.wait();
                    let other = 1 - k;
                    let waited = wait_until(CellKey::new(9, other as u32), |key| cells[key.id() as usize].owner(), || cells[other].get() == Completion::Done);
                    cells[k].set(Completion::Done);
                    waited
                })
            })
            .collect();
        let waited: Vec<bool> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(waited.iter().filter(|&&w| !w).count(), 1, "one of the two sees the cycle: {:?}", waited);
        assert!(cells.iter().all(|c| c.get() == Completion::Done));
    }

    /// The storage contract of the type store's overlays: a
    /// mapped vector reads its own entries and its regions' by their position in the one
    /// mapping, a region is written at its offset from its start, and the vector's own entries
    /// stay where the mapping put them.
    #[test]
    fn a_mapped_vector_reads_its_regions_by_position() {
        let v: SlabVec<u64> = SlabVec::with_capacity(4);
        for x in [10, 11, 12] {
            v.push(x);
        }
        let before = v.get(1) as *const u64;
        assert!(v.map_regions(1 << 20, 1 << 16));
        assert_eq!((*v.get(0), *v.get(1), *v.get(2)), (10, 11, 12));
        assert_eq!(*unsafe { &*before }, 11, "the buffer the entries left is kept for its readers");
        let region = v.region(1 << 18, 1 << 10);
        assert_eq!(region.push(70), 0);
        assert_eq!(region.push(71), 1);
        assert_eq!((*v.get(1 << 18), *v.get((1 << 18) + 1)), (70, 71));
        assert_eq!(*region.get(1), 71);
        assert_eq!(v.push(13), 3);
        assert_eq!(v.len(), 4);
        assert!(v.held() < 1 << 12, "a mapped vector counts its entries, not its room: {}", v.held());
    }

    /// A value that counts its drops.
    struct Counted(std::sync::Arc<AtomicUsize>);

    impl Drop for Counted {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A mapped vector with two entries of its own and a region with three, the drops counted.
    fn mapped_with_region() -> (SlabVec<Counted>, Region<Counted>, std::sync::Arc<AtomicUsize>) {
        let drops = std::sync::Arc::new(AtomicUsize::new(0));
        let v: SlabVec<Counted> = SlabVec::with_capacity(4);
        v.push(Counted(drops.clone()));
        assert!(v.map_regions(1 << 12, 1 << 8));
        v.push(Counted(drops.clone()));
        let region = v.region(1 << 10, 16);
        for _ in 0..3 {
            region.push(Counted(drops.clone()));
        }
        (v, region, drops)
    }

    /// A region holds the mapping with its vector: whichever goes first, each entry is dropped
    /// once, by its owner, and the pages go with the last of the two, as they do after the
    /// vector's entries are taken out (`into_vec`) with a region still live.
    #[test]
    fn a_region_and_its_vector_go_in_either_order() {
        let before = mapped_here();
        let (v, region, drops) = mapped_with_region();
        drop(v);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
        assert!(mapped_here() > before, "the region keeps the pages");
        assert_eq!(region.len(), 3);
        drop(region);
        assert_eq!((drops.load(Ordering::Relaxed), mapped_here()), (5, before));

        let (v, region, drops) = mapped_with_region();
        drop(region);
        assert_eq!(drops.load(Ordering::Relaxed), 3);
        drop(v);
        assert_eq!((drops.load(Ordering::Relaxed), mapped_here()), (5, before));

        let (v, region, drops) = mapped_with_region();
        let own = v.into_vec();
        assert_eq!((own.len(), drops.load(Ordering::Relaxed)), (2, 0));
        assert!(mapped_here() > before);
        drop(own);
        drop(region);
        assert_eq!((drops.load(Ordering::Relaxed), mapped_here()), (5, before));
    }

    /// A region's room is its writer's limit: past it, a panic, never an entry in the next
    /// region (a worker's types past `WORKER_SPAN`).
    #[test]
    #[should_panic(expected = "a worker made more than 3 entries")]
    fn a_region_refuses_an_entry_past_its_room() {
        let v: SlabVec<u32> = SlabVec::with_capacity(4);
        assert!(v.map_regions(64, 16));
        let region = v.region(32, 4);
        for x in 0..4 {
            region.push(x);
        }
    }

    /// The vector's own room in its mapping is its limit too (the base's `LOCAL_BASE` ids).
    #[test]
    #[should_panic(expected = "a mapped store ran past its 8 entries")]
    fn a_mapped_vector_refuses_a_push_past_its_own_room() {
        let v: SlabVec<u32> = SlabVec::with_capacity(4);
        assert!(v.map_regions(64, 8));
        for x in 0..9 {
            v.push(x);
        }
    }

    /// A table indexes an overlay by the offset in its region, so that the first entry of a
    /// high-numbered worker allocates a table's minimum: indexed by its id, the table's kept
    /// hashes would be sized by it (`keep_hash_apart`), about 512 MiB for the first worker's
    /// first id, `LOCAL_BASE`.
    #[test]
    fn an_overlay_index_by_offset_stays_small() {
        let v: SlabVec<u64> = SlabVec::with_capacity(4);
        assert!(v.map_regions(1 << 24, 1 << 10));
        let region = v.region(15 << 20, 1 << 20);
        let index = Table::with_capacity(16);
        let at = region.push(99) as u32;
        index.insert(hash_of(&99u64), at);
        assert_eq!(index.find(hash_of(&99u64), |i| *region.get(i as usize) == 99), Some(0));
        assert!(index.held() < 1 << 12, "{} bytes", index.held());
    }

    /// The base keeps growing after the fork while workers probe it: an entry indexed before
    /// the growth is found through every resize (the table a reader loaded is retired, not
    /// changed), and an entry indexed meanwhile is found or missed, never mistaken.
    #[test]
    fn a_reader_finds_every_earlier_entry_while_the_table_grows() {
        use std::sync::Arc;
        let entries: Arc<SlabVec<u64>> = Arc::new(SlabVec::with_capacity(16));
        let table = Arc::new(Table::with_capacity(16));
        const BEFORE: u64 = 2000;
        for k in 0..BEFORE {
            let pos = entries.push(k * 7919);
            table.insert(hash_of(&(k * 7919)), pos as u32);
        }
        let reader = {
            let (entries, table) = (entries.clone(), table.clone());
            std::thread::spawn(move || {
                for round in 0..200u64 {
                    for k in 0..BEFORE {
                        let x = k * 7919;
                        assert_eq!(table.find(hash_of(&x), |i| *entries.get(i as usize) == x), Some(k as u32), "round {}", round);
                    }
                    let later = (BEFORE + round) * 7919;
                    if let Some(i) = table.find(hash_of(&later), |i| *entries.get(i as usize) == later) {
                        assert_eq!(*entries.get(i as usize), later);
                    }
                }
            })
        };
        for k in BEFORE..BEFORE + 20000 {
            let pos = entries.push(k * 7919);
            table.insert(hash_of(&(k * 7919)), pos as u32);
        }
        reader.join().unwrap();
    }

    #[test]
    fn slab_keeps_references_across_growth() {
        let v: SlabVec<u64> = SlabVec::with_capacity(16);
        v.push(7);
        let first = v.get(0);
        for i in 1..1000 {
            v.push(i);
        }
        assert_eq!(*first, 7);
        assert_eq!(*v.get(999), 999);
        assert_eq!(v.len(), 1000);
        assert_eq!(v.into_vec().len(), 1000);
    }

    #[test]
    fn table_finds_what_it_indexed() {
        let entries: SlabVec<u64> = SlabVec::with_capacity(16);
        let table = Table::with_capacity(16);
        for k in 0..5000u64 {
            let h = hash_of(&(k * 7919));
            assert!(table.find(h, |i| *entries.get(i as usize) == k * 7919).is_none());
            let pos = entries.push(k * 7919);
            table.insert(h, pos as u32);
        }
        for k in 0..5000u64 {
            let h = hash_of(&(k * 7919));
            assert_eq!(table.find(h, |i| *entries.get(i as usize) == k * 7919), Some(k as u32));
        }
    }

    /// Four threads passing the lock between them, each release taking place while others wait for
    /// it, its waiters spinning for `spin` and parking after, each hold `hold` long: none may miss its
    /// wake and stay parked, and no two may hold it at once. The threads start together behind a
    /// barrier and must all finish; a watchdog fails the test otherwise.
    fn hand_over(spin: std::time::Duration, hold: std::time::Duration, rounds: u64) {
        use std::sync::atomic::AtomicU64;
        use std::sync::{Arc, Barrier};
        const THREADS: usize = 4;
        let lock = Arc::new(ReentrantLock::spinning(spin));
        let counted = Arc::new(AtomicU64::new(0));
        let inside = Arc::new(AtomicBool::new(false));
        let start = Arc::new(Barrier::new(THREADS));
        let (done, finished) = std::sync::mpsc::channel::<()>();
        for _ in 0..THREADS {
            let (lock, counted, inside, start, done) = (lock.clone(), counted.clone(), inside.clone(), start.clone(), done.clone());
            std::thread::spawn(move || {
                start.wait();
                for i in 0..rounds {
                    lock.lock();
                    assert!(!inside.swap(true, Ordering::Relaxed), "two threads hold the lock");
                    if i % 3 == 0 {
                        lock.lock();
                        lock.unlock();
                    }
                    let until = std::time::Instant::now() + hold;
                    while std::time::Instant::now() < until {
                        std::hint::spin_loop();
                    }
                    counted.fetch_add(1, Ordering::Relaxed);
                    inside.store(false, Ordering::Relaxed);
                    lock.unlock();
                }
                let _ = done.send(());
            });
        }
        for _ in 0..THREADS {
            finished.recv_timeout(std::time::Duration::from_secs(30)).expect("the lock was never handed over: a thread is stuck");
        }
        assert_eq!(counted.load(Ordering::Relaxed), THREADS as u64 * rounds);
    }

    /// Std's short spin alone: nearly every waiter parks.
    #[test]
    fn reentrant_lock_is_handed_over_to_parked_waiters() {
        hand_over(std::time::Duration::ZERO, std::time::Duration::ZERO, 100_000);
    }

    /// The default spin, every hold shorter than it: the waiters take the lock spinning.
    #[test]
    fn reentrant_lock_is_handed_over_to_spinning_waiters() {
        hand_over(std::time::Duration::from_micros(50), std::time::Duration::ZERO, 100_000);
    }

    /// Holds longer than the spin: waiters park after spinning while others arrive and spin.
    #[test]
    fn reentrant_lock_is_handed_over_past_the_spin() {
        hand_over(std::time::Duration::from_micros(5), std::time::Duration::from_micros(30), 2_000);
    }

    #[test]
    fn reentrant_lock_counts_depth() {
        let l = ReentrantLock::new();
        l.lock();
        l.lock();
        assert_eq!(lock_depth(), 2);
        assert!(l.held_by_me());
        l.unlock();
        assert!(l.held_by_me());
        l.unlock();
        assert!(!l.held_by_me());
        assert_eq!(lock_depth(), 0);
    }
}
