//! A thread-local size-class allocator with a centre the threads exchange memory through.
//! Blocks up to 32 KB are carved from 4 MB chunks and recycled through per-thread free lists;
//! larger requests go to the system allocator (a resident process maps the ones of 1 MB and
//! more itself, below). The copy inlined at every allocation site is
//! the free-list pop or the bump, and at every free the push: a free costs a compiler as many
//! instructions as an allocation (a check frees what it allocates, 2.6 million blocks on the
//! core-only benchmark), so nothing is counted or compared there. Everything else is cold and
//! out of line.
//!
//! A process that compiles once and ends needs no more than that, and gets no more: a thread
//! whose region is used up takes a chunk of the system and bumps from it whatever the class,
//! and what a thread holds when it ends is left where it is.
//!
//! A resident process (`resident`, which `teq compiler watch` calls before its first build) hands
//! memory between its threads in one direction: a worker of a build allocates the output the
//! compiler thread frees, the typing thread allocates the program the compiler thread drops.
//! So there what a thread has no use for goes to the centre, and a thread that runs dry takes
//! from there before anything is carved anew:
//!
//! - A list that is empty is refilled with a chain of the centre (`BATCH_BYTES` of blocks at
//!   most); without one the centre carves a run of that class (`RUN_BYTES`) from its regions,
//!   and without a region it takes a chunk of the system. No thread has a region of its own:
//!   it would bump every class from it, the classes the centre has free included.
//! - A thread that lives across builds calls `settle` where it goes idle (the compiler thread
//!   before it waits for a command and after it dropped a program, the typing thread after
//!   every phase): each list keeps a batch and the rest goes to the centre in chains. Between
//!   two such points a thread holds what it freed since the first, which is the bound: no
//!   free is ever counted.
//! - A thread that ends hands over its lists (`Handback`, a thread-local with a destructor).
//!   Every thread of the compiler is started through `spawn` or `spawn_in`, or has `enter` as
//!   the first statement of what it runs: the hand-over is then registered whether or not
//!   the thread ever allocates, and before any thread-local of the program, so that it runs
//!   after them and what their destructors free goes with it. The rule is the author's to
//!   keep: a thread-local with a destructor that a thread makes before `enter` is destroyed
//!   after the hand-over. A unit test holds the list of the places that start a thread and
//!   fails when a file starts one more or one less, which calls for the list and this rule to
//!   be read; it proves nothing about the places it lists.
//!
//! Chunks are never returned to the system: the allocator holds the most its threads needed
//! at one time, per class.
//!
//! A resident process also maps its requests of 1 MB and more itself (`Mapping`): the arrays
//! of a program, which a full build drops and makes anew, and the arrays a build works in.
//! The system's allocator keeps such blocks mapped and written to once they are freed, for a
//! request of their size to come, and how much it keeps is not the session's to say: the same
//! hundred edits of a resident check ended at 200 MB in some runs and at 375 MB in others, and
//! a thousand wandered between 420 and 750 MB over an account of 163 MB. A mapping has room
//! to grow in place (four times its request in address space, of which the pages written to
//! count), is kept at the centre when it is freed for the next request of its size, and is
//! unmapped when it sat there through a whole build (`age`, at the compiler thread's idle
//! point), so that the arrays a build works in are the last build's and a dropped program's
//! go back to the system.
//!
//! What runs after `Handback` on an ending thread still allocates and frees: `STATE` has no
//! destructor and is never invalid. An allocation then comes from the system, one block at a
//! time, and joins the lists of whoever frees it; a free lands on the list of the ending
//! thread and is lost with it. On a thread of the compiler that is the runtime's own cleanup
//! alone, which frees the thread's handle when the thread holds the last reference to it
//! (some tens of bytes, and none where the thread is joined through its handle, as the
//! workers of a build are). A thread that was not started as above and registered a
//! thread-local of its own first loses what that destructor frees; the compiler has none.
//!
//! Nothing here allocates through the allocator or takes a lock but the centre's, which is
//! held over pointer moves alone.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const SMALL_STEP: usize = 16;
const SMALL_LIMIT: usize = 1024;
const SMALL_CLASSES: usize = SMALL_LIMIT / SMALL_STEP;
const LARGE_CLASSES: usize = 5;
const MAX_SIZE: usize = SMALL_LIMIT << LARGE_CLASSES;
const CHUNK_SIZE: usize = 4 * 1024 * 1024;
const CLASSES: usize = SMALL_CLASSES + LARGE_CLASSES;
/// The blocks a chain holds and a list keeps when its thread settles, in bytes; one block
/// where a block is as large.
const BATCH_BYTES: usize = 32 * 1024;
/// The blocks carved anew for a class of a resident process, in bytes; two blocks of the
/// largest class.
const RUN_BYTES: usize = 64 * 1024;
/// The least rest that stays a region; a smaller one is carved into blocks.
const REGION_MIN: usize = 4096;
const WORD: usize = std::mem::size_of::<usize>();
/// The least request a resident process maps itself.
const MAPPED_MIN: usize = 1 << 20;
/// A mapping's first words: its length, and the next mapping while it is kept at the centre.
const MAPPED_HEADER: usize = SMALL_STEP;
/// The address space of a mapping against its request: two doublings grow in place.
const MAPPED_ROOM: usize = 4;
/// A mapping's length is a power of two, which is its tier.
const TIERS: usize = usize::BITS as usize;

struct State {
    bump: *mut u8,
    end: *mut u8,
    free: [*mut u8; CLASSES],
}

const EMPTY: State = State { bump: null_mut(), end: null_mut(), free: [null_mut(); CLASSES] };

thread_local! {
    static STATE: UnsafeCell<State> = const { UnsafeCell::new(EMPTY) };
    static HANDBACK: Handback = const { Handback };
}

/// The calling thread's state, through `try_with`: the standard library marks it `#[inline]`, so
/// each codegen unit that allocates holds a copy, where `with` is one function in one unit, and
/// the allocation paths inlined into the other units called the thread-local's accessor out of
/// line in some builds and not in others (docs/SPEED.md, "Placement and the instruction count").
#[inline(always)]
fn state() -> *mut State {
    match STATE.try_with(UnsafeCell::get) {
        Ok(state) => state,
        Err(_) => std::process::abort(),
    }
}

/// Gives the thread's memory to the centre when the thread ends.
struct Handback;

impl Drop for Handback {
    fn drop(&mut self) {
        if !RESIDENT.load(Ordering::Relaxed) {
            return;
        }
        // SAFETY: `STATE` has no destructor, so it is valid while the thread's destructors
        // run, and nothing else borrows it: the allocator's entries do not nest.
        unsafe {
            let s = &mut *state();
            release_region(s);
            release_lists(s, false);
        }
    }
}

/// What the threads exchange. A chain is a list of free blocks of one class linked through
/// their first word; the chains of a class are linked through the second word of their first
/// blocks, and a chain of more than one block has its length in the second word of its second
/// block. A region is memory no block was carved from, its first word the next region and
/// its second its end.
struct Centre {
    chains: [*mut u8; CLASSES],
    blocks: [usize; CLASSES],
    regions: *mut u8,
    region_bytes: usize,
    /// The mappings freed since the compiler thread was last idle, by tier, and the ones
    /// freed before that.
    fresh: [*mut u8; TIERS],
    stale: [*mut u8; TIERS],
    kept_bytes: usize,
    /// The live mappings, address and length, for `resident_pages`: the first `live_n`.
    live_maps: [(usize, usize); LIVE_MAPS],
    live_n: usize,
}

impl Centre {
    fn note_live(&mut self, mapping: usize, len: usize) {
        if self.live_n < LIVE_MAPS {
            self.live_maps[self.live_n] = (mapping, len);
            self.live_n += 1;
        }
    }

    fn forget_live(&mut self, mapping: usize) {
        if let Some(at) = self.live_maps[..self.live_n].iter().position(|m| m.0 == mapping) {
            self.live_n -= 1;
            self.live_maps[at] = self.live_maps[self.live_n];
        }
    }
}

struct Shared {
    locked: AtomicBool,
    centre: UnsafeCell<Centre>,
}

// SAFETY: the centre is read and written under `locked` alone.
unsafe impl Sync for Shared {}

static SHARED: Shared = Shared {
    locked: AtomicBool::new(false),
    centre: UnsafeCell::new(Centre {
        chains: [null_mut(); CLASSES],
        blocks: [0; CLASSES],
        regions: null_mut(),
        region_bytes: 0,
        fresh: [null_mut(); TIERS],
        stale: [null_mut(); TIERS],
        kept_bytes: 0,
        live_maps: [(0, 0); LIVE_MAPS],
        live_n: 0,
    }),
};

fn note_chunk(chunk: *mut u8) {
    let n = CHUNKS.fetch_add(1, Ordering::Relaxed);
    if n < CHUNK_LIST_LEN {
        CHUNK_LIST[n].store(chunk as usize, Ordering::Relaxed);
    }
}

static RESIDENT: AtomicBool = AtomicBool::new(false);
/// The chunks' addresses in the order taken, for `resident`.
const CHUNK_LIST_LEN: usize = 4096;
static CHUNK_LIST: [AtomicUsize; CHUNK_LIST_LEN] = [const { AtomicUsize::new(0) }; CHUNK_LIST_LEN];
const LIVE_MAPS: usize = 2048;
#[cfg(target_os = "linux")]
const KEPT_LIST: usize = 4096;
static MAPPING: AtomicBool = AtomicBool::new(false);
static MAPPED_LIVE: AtomicUsize = AtomicUsize::new(0);
static MAPPINGS_MADE: AtomicUsize = AtomicUsize::new(0);
static CHUNKS: AtomicUsize = AtomicUsize::new(0);
static STRAY_BYTES: AtomicUsize = AtomicUsize::new(0);
static CHAINS_TAKEN: AtomicUsize = AtomicUsize::new(0);
static RUNS_CARVED: AtomicUsize = AtomicUsize::new(0);
static LARGE_LIVE: AtomicUsize = AtomicUsize::new(0);
static LARGE_PEAK: AtomicUsize = AtomicUsize::new(0);
/// A class whose chains no thread takes from the centre, where a test looks for the blocks a
/// thread gave back (`tests::destructor_frees_come_back`): a thread whose list of it runs dry
/// carves a run meanwhile. `usize::MAX` for none; read by the tests' builds alone (`cfg!(test)`,
/// not an attribute, which would hide the code after it from the count of thread starts).
static KEPT_CLASS: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Runs `f` on the centre. `f` moves pointers and nothing else: it cannot allocate, block or
/// unwind, so a spin lock does and no entry of the allocator is re-entered under it.
#[inline]
fn centre<R>(f: impl FnOnce(&mut Centre) -> R) -> R {
    let mut spins = 0u32;
    while SHARED.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        spins += 1;
        if spins % 64 == 0 {
            std::thread::yield_now();
        } else {
            std::hint::spin_loop();
        }
    }
    // SAFETY: the lock is held.
    let result = f(unsafe { &mut *SHARED.centre.get() });
    SHARED.locked.store(false, Ordering::Release);
    result
}

pub struct TeqAlloc;

#[inline]
fn is_small(size: usize, align: usize) -> bool {
    size <= MAX_SIZE && align <= SMALL_STEP
}

#[inline]
fn class_of(size: usize) -> (usize, usize) {
    if size <= SMALL_LIMIT {
        let class = (size.max(1) + SMALL_STEP - 1) / SMALL_STEP;
        (class - 1, class * SMALL_STEP)
    } else {
        let rounded = size.next_power_of_two();
        let class = SMALL_CLASSES + (rounded.trailing_zeros() - SMALL_LIMIT.trailing_zeros()) as usize - 1;
        (class, rounded)
    }
}

fn size_of_class(class: usize) -> usize {
    if class < SMALL_CLASSES {
        (class + 1) * SMALL_STEP
    } else {
        (2 * SMALL_LIMIT) << (class - SMALL_CLASSES)
    }
}

#[inline(always)]
unsafe fn next(block: *mut u8) -> *mut *mut u8 {
    block as *mut *mut u8
}

/// The second word of a block: the next chain in a chain's first block, the chain's length in
/// its second; a region's end.
#[inline(always)]
unsafe fn second(block: *mut u8) -> *mut usize {
    block.add(WORD) as *mut usize
}

#[inline(always)]
unsafe fn take(s: &mut State, class: usize, size: usize) -> *mut u8 {
    let head = s.free[class];
    if !head.is_null() {
        s.free[class] = *next(head);
        return head;
    }
    let p = s.bump;
    if (s.end as usize) - (p as usize) >= size {
        s.bump = p.add(size);
        return p;
    }
    refill(s, class, size)
}

#[inline(always)]
unsafe fn give(s: &mut State, ptr: *mut u8, class: usize) {
    *next(ptr) = s.free[class];
    s.free[class] = ptr;
}

/// What the centre's first region gives.
enum Carved {
    /// Memory of a length, and the length of what follows it where that is too small to stay
    /// a region.
    Run(*mut u8, usize, usize),
    /// A region that does not hold one block, taken off the centre.
    Short(*mut u8, usize),
    Nothing,
}

/// A block for a thread whose list of the class is empty and whose region is used up.
#[cold]
#[inline(never)]
unsafe fn refill(s: &mut State, class: usize, size: usize) -> *mut u8 {
    if !RESIDENT.load(Ordering::Relaxed) {
        let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK_SIZE, SMALL_STEP));
        if chunk.is_null() {
            return null_mut();
        }
        note_chunk(chunk);
        s.bump = chunk.add(size);
        s.end = chunk.add(CHUNK_SIZE);
        return chunk;
    }
    // Touching `HANDBACK` registers its destructor. Past its destruction the thread is
    // ending and nobody would hand over what it took, so it takes one block of the system.
    if HANDBACK.try_with(|_| ()).is_err() {
        STRAY_BYTES.fetch_add(size, Ordering::Relaxed);
        return System.alloc(Layout::from_size_align_unchecked(size, SMALL_STEP));
    }
    let kept = cfg!(test) && KEPT_CLASS.load(Ordering::Acquire) == class;
    let chain = centre(|c| {
        let chain = if kept { null_mut() } else { c.chains[class] };
        if !chain.is_null() {
            c.chains[class] = *second(chain) as *mut u8;
            c.blocks[class] -= chain_len(chain);
        }
        chain
    });
    if !chain.is_null() {
        CHAINS_TAKEN.fetch_add(1, Ordering::Relaxed);
        s.free[class] = *next(chain);
        return chain;
    }
    let want = (RUN_BYTES / size).max(1) * size;
    loop {
        let carved = centre(|c| {
            let region = c.regions;
            if region.is_null() {
                return Carved::Nothing;
            }
            let end = *second(region);
            let len = end - region as usize;
            if len < size {
                c.regions = *next(region);
                c.region_bytes -= len;
                return Carved::Short(region, len);
            }
            let run = want.min(len / size * size);
            if len - run >= REGION_MIN {
                let rest = region.add(run);
                *next(rest) = *next(region);
                *second(rest) = end;
                c.regions = rest;
                c.region_bytes -= run;
                Carved::Run(region, run, 0)
            } else {
                c.regions = *next(region);
                c.region_bytes -= len;
                Carved::Run(region, run, len - run)
            }
        });
        match carved {
            Carved::Run(start, run, rest) => {
                // A region from before the process was resident.
                release_region(s);
                carve(s, start.add(run), rest);
                RUNS_CARVED.fetch_add(1, Ordering::Relaxed);
                let mut block = start.add(run - size);
                let mut after = s.free[class];
                while block != start {
                    *next(block) = after;
                    after = block;
                    block = block.sub(size);
                }
                s.free[class] = after;
                return start;
            }
            Carved::Short(start, len) => carve(s, start, len),
            Carved::Nothing => {
                let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK_SIZE, SMALL_STEP));
                if chunk.is_null() {
                    return null_mut();
                }
                note_chunk(chunk);
                give_region(chunk, CHUNK_SIZE);
            }
        }
    }
}

/// The blocks of a chain.
unsafe fn chain_len(chain: *mut u8) -> usize {
    let after = *next(chain);
    if after.is_null() {
        1
    } else {
        *second(after)
    }
}

unsafe fn give_region(start: *mut u8, len: usize) {
    *second(start) = start as usize + len;
    centre(|c| {
        *next(start) = c.regions;
        c.regions = start;
        c.region_bytes += len;
    });
}

/// Puts memory too small for a region on the thread's lists, as the largest blocks that fit.
/// Every size carved is a multiple of the step, and so is what is left of any.
unsafe fn carve(s: &mut State, mut start: *mut u8, mut left: usize) {
    while left > 0 {
        let size = if left > SMALL_LIMIT { (1 << left.ilog2()).min(MAX_SIZE) } else { left };
        give(s, start, class_of(size).0);
        start = start.add(size);
        left -= size;
    }
}

/// Gives up the rest of the thread's region: to the centre, or, when it is small, as blocks
/// on the thread's lists.
unsafe fn release_region(s: &mut State) {
    let (start, left) = (s.bump, s.end as usize - s.bump as usize);
    s.bump = null_mut();
    s.end = null_mut();
    if left >= REGION_MIN {
        give_region(start, left);
    } else {
        carve(s, start, left);
    }
}

/// Gives the thread's lists to the centre in chains, all of them or what is beyond a batch.
unsafe fn release_lists(s: &mut State, keep: bool) {
    // Per class the first and the last chain made and their blocks.
    let mut made: [(*mut u8, *mut u8, usize); CLASSES] = [(null_mut(), null_mut(), 0); CLASSES];
    let mut any = false;
    for class in 0..CLASSES {
        let batch = (BATCH_BYTES / size_of_class(class)).max(1);
        let mut head = s.free[class];
        if head.is_null() {
            continue;
        }
        if keep {
            let mut last = head;
            let mut n = 1;
            while n < batch && !(*next(last)).is_null() {
                last = *next(last);
                n += 1;
            }
            head = *next(last);
            *next(last) = null_mut();
        } else {
            s.free[class] = null_mut();
        }
        while !head.is_null() {
            let chain = head;
            let mut last = head;
            let mut n = 1;
            while n < batch && !(*next(last)).is_null() {
                last = *next(last);
                n += 1;
            }
            head = *next(last);
            *next(last) = null_mut();
            if n > 1 {
                *second(*next(chain)) = n;
            }
            let (first, bottom, blocks) = made[class];
            *second(chain) = first as usize;
            made[class] = (chain, if first.is_null() { chain } else { bottom }, blocks + n);
            any = true;
        }
    }
    if !any {
        return;
    }
    centre(|c| {
        for class in 0..CLASSES {
            let (first, bottom, blocks) = made[class];
            if !first.is_null() {
                *second(bottom) = c.chains[class] as usize;
                c.chains[class] = first;
                c.blocks[class] += blocks;
            }
        }
    });
}

/// Whether the process stays (`resident`).
#[inline]
pub fn is_resident() -> bool {
    RESIDENT.load(Ordering::Relaxed)
}

/// Tells the allocator that the process stays: its threads exchange memory through the
/// centre (the module's header).
pub fn resident() {
    RESIDENT.store(true, Ordering::Relaxed);
    // A free tells a mapping from a block of the system by its size alone, so the process
    // maps from its start or not at all.
    if cfg!(any(target_os = "macos", target_os = "linux")) && LARGE_LIVE.load(Ordering::Relaxed) == 0 {
        MAPPING.store(true, Ordering::Relaxed);
    }
}

/// For the compiler thread of a resident process, where it goes idle: the mappings that
/// were kept through the build before go back to the system.
pub fn age() {
    let stale = centre(|c| {
        let stale = std::mem::replace(&mut c.stale, std::mem::replace(&mut c.fresh, [null_mut(); TIERS]));
        for (tier, &first) in stale.iter().enumerate() {
            let mut mapping = first;
            while !mapping.is_null() {
                c.kept_bytes -= 1 << tier;
                // SAFETY: a kept mapping's second word is the next one.
                mapping = unsafe { *second(mapping) as *mut u8 };
            }
        }
        stale
    });
    for (tier, &first) in stale.iter().enumerate() {
        let mut mapping = first;
        while !mapping.is_null() {
            // SAFETY: the mapping is off the centre's lists and this thread's alone.
            unsafe {
                let after = *second(mapping) as *mut u8;
                os::unmap(mapping, 1 << tier);
                mapping = after;
            }
        }
    }
}

/// Whether a request is a mapping of this process's.
#[inline]
fn is_mapped(size: usize, align: usize) -> bool {
    size >= MAPPED_MIN && align <= SMALL_STEP && MAPPING.load(Ordering::Relaxed)
}

/// The mapping of a request: its length in its first word, the block after the header.
unsafe fn map(size: usize) -> *mut u8 {
    let Some(len) = size.checked_mul(MAPPED_ROOM).and_then(|n| n.checked_add(MAPPED_HEADER)).and_then(usize::checked_next_power_of_two) else {
        return null_mut();
    };
    let tier = len.trailing_zeros() as usize;
    let kept = centre(|c| {
        let list = if c.fresh[tier].is_null() { &mut c.stale[tier] } else { &mut c.fresh[tier] };
        let mapping = *list;
        if !mapping.is_null() {
            *list = *second(mapping) as *mut u8;
            c.kept_bytes -= len;
        }
        mapping
    });
    let mapping = if kept.is_null() {
        MAPPINGS_MADE.fetch_add(1, Ordering::Relaxed);
        os::map(len)
    } else {
        kept
    };
    if mapping.is_null() {
        return null_mut();
    }
    *(mapping as *mut usize) = len;
    centre(|c| c.note_live(mapping as usize, len));
    mapping.add(MAPPED_HEADER)
}

/// The page a mapping's pages are given back by: the kernel's on both systems, the larger
/// one where a system has two.
const PAGE: usize = if cfg!(target_os = "macos") { 16384 } else { 4096 };

/// Gives the pages of a mapping past `size` back to the system, their contents the system's
/// to drop (Darwin keeps them readable until it needs the pages; `alloc_zeroed` clears
/// explicitly, and no path takes a mapping for zero).
/// A mapping is `MAPPED_ROOM` times its request and the next request of its tier may use a
/// quarter of it, so what one use touched would stay resident through the mapping's every
/// later life otherwise, 70 MB of a frontend session's 600 by the twentieth full build.
unsafe fn release_past(mapping: *mut u8, len: usize, size: usize) {
    let used = (MAPPED_HEADER + size).div_ceil(PAGE) * PAGE;
    if used < len {
        os::release(mapping.add(used), len - used);
    }
}

/// Keeps a freed mapping at the centre, the pages past the block given back.
unsafe fn unmap(block: *mut u8, size: usize) {
    let mapping = block.sub(MAPPED_HEADER);
    let len = *(mapping as *mut usize);
    let tier = len.trailing_zeros() as usize;
    release_past(mapping, len, size);
    centre(|c| {
        *second(mapping) = c.fresh[tier] as usize;
        c.fresh[tier] = mapping;
        c.kept_bytes += len;
        c.forget_live(mapping as usize);
    });
}

/// A mapping for a new size: itself while the size fits its length and is not a small part
/// of it, a mapping of the new size's otherwise.
unsafe fn remap(block: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
    let len = *(block.sub(MAPPED_HEADER) as *mut usize);
    if new_size + MAPPED_HEADER <= len && new_size >= len / (4 * MAPPED_ROOM) {
        if new_size < old_size {
            release_past(block.sub(MAPPED_HEADER), len, new_size);
        }
        return block;
    }
    let new = map(new_size);
    if !new.is_null() {
        std::ptr::copy_nonoverlapping(block, new, old_size.min(new_size));
        unmap(block, old_size);
    }
    new
}

/// Anonymous memory from the system and back to it.
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod os {
    use std::ffi::c_void;

    extern "C" {
        fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
        fn munmap(addr: *mut c_void, len: usize) -> i32;
        fn madvise(addr: *mut c_void, len: usize, advice: i32) -> i32;
    }

    /// The advice that takes the pages off the process's footprint: Linux drops them on
    /// `MADV_DONTNEED`; Darwin only deactivates them on that and on `MADV_FREE`, and takes
    /// them off the ledger on `MADV_FREE_REUSABLE`.
    #[cfg(target_os = "linux")]
    const RELEASE_ADVICE: i32 = 4;
    #[cfg(target_os = "macos")]
    const RELEASE_ADVICE: i32 = 7;

    const PROT_READ_WRITE: i32 = 1 | 2;
    const MAP_PRIVATE: i32 = 2;
    #[cfg(target_os = "macos")]
    const MAP_ANONYMOUS: i32 = 0x1000;
    #[cfg(target_os = "linux")]
    const MAP_ANONYMOUS: i32 = 0x20;
    /// No swap is set aside for the room a mapping may never use.
    #[cfg(target_os = "macos")]
    const MAP_NORESERVE: i32 = 0x40;
    #[cfg(target_os = "linux")]
    const MAP_NORESERVE: i32 = 0x4000;

    pub unsafe fn map(len: usize) -> *mut u8 {
        let at = mmap(std::ptr::null_mut(), len, PROT_READ_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
        if at as isize == -1 {
            std::ptr::null_mut()
        } else {
            at as *mut u8
        }
    }

    pub unsafe fn unmap(at: *mut u8, len: usize) {
        munmap(at as *mut c_void, len);
    }

    /// The pages of a range given back, the range still mapped.
    pub unsafe fn release(at: *mut u8, len: usize) {
        madvise(at as *mut c_void, len, RELEASE_ADVICE);
    }

    /// The process's physical footprint on Darwin, the kernel's ledger as the memory suite
    /// reads it (`proc_pid_rusage`, `ri_phys_footprint`); zero elsewhere.
    #[cfg(target_os = "macos")]
    pub fn footprint() -> usize {
        extern "C" {
            fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
        }
        // `rusage_info_v0`: the uuid, then user time, system time, package idle wakeups,
        // interrupt wakeups, pageins, wired size, resident size, physical footprint, ...
        let mut info = [0u64; 12];
        let rc = unsafe { proc_pid_rusage(std::process::id() as i32, 0, info.as_mut_ptr()) };
        if rc == 0 {
            info[9] as usize
        } else {
            0
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn footprint() -> usize {
        0
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod os {
    pub unsafe fn map(_: usize) -> *mut u8 {
        std::ptr::null_mut()
    }

    pub unsafe fn unmap(_: *mut u8, _: usize) {}

    pub unsafe fn release(_: *mut u8, _: usize) {}

    pub fn footprint() -> usize {
        0
    }
}

/// `len` bytes of zeros mapped from the system, for a directory whose pages a build touches
/// sparsely (`arena.rs`): the pages are the system's until written, where a zeroed block of the
/// system's allocator can be a freed one it clears in full (0.24 ms of the type phase for the
/// cell directories of `ovl_1`, whose parse had freed such blocks).
pub fn zeroed_pages(len: usize) -> *mut u8 {
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        unsafe { os::map(len) }
    } else {
        unsafe { System.alloc_zeroed(Layout::from_size_align_unchecked(len, SMALL_STEP)) }
    }
}

/// Gives back what `zeroed_pages` gave.
pub unsafe fn free_zeroed_pages(at: *mut u8, len: usize) {
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        os::unmap(at, len)
    } else {
        System.dealloc(at, Layout::from_size_align_unchecked(len, SMALL_STEP))
    }
}

/// The whole pages under the first `len` bytes of the `room` bytes from `at` given back to the
/// system, the range still mapped and reading as zeros: a page partly used is given back where
/// the rest of it lies in the room, which no one else writes.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub unsafe fn release_pages(at: *mut u8, len: usize, room: usize) {
    if len == 0 {
        return;
    }
    extern "C" {
        fn getpagesize() -> i32;
    }
    let page = getpagesize().max(4096) as usize;
    let start = at as usize;
    let begin = (start + page - 1) & !(page - 1);
    let end = ((start + len + page - 1) & !(page - 1)).min((start + room) & !(page - 1));
    if end > begin {
        os::release(begin as *mut u8, end - begin);
    }
}

/// Elsewhere the pages are the system allocator's, `zeroed_pages`'s, and stay.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub unsafe fn release_pages(_: *mut u8, _: usize, _: usize) {}

/// Windows' counterpart of a mapping without a reservation of swap (`MAP_NORESERVE`), which a
/// commit would be: `len` bytes of address space reserved (`VirtualAlloc`'s `MEM_RESERVE`),
/// none of the system's commit limit taken; their pages are taken as their writer commits them
/// (`commit_pages`). Null when refused.
#[cfg(windows)]
pub fn reserve_pages(len: usize) -> *mut u8 {
    unsafe { windows_memory::VirtualAlloc(std::ptr::null_mut(), len, windows_memory::MEM_RESERVE, windows_memory::PAGE_READWRITE) as *mut u8 }
}

/// The pages under the `len` bytes from `at` of a reservation committed, reading as zeros until
/// written; those already committed stay as they are. False when the system's commit limit
/// refuses them.
#[cfg(windows)]
pub unsafe fn commit_pages(at: *mut u8, len: usize) -> bool {
    len == 0 || !windows_memory::VirtualAlloc(at as *mut std::ffi::c_void, len, windows_memory::MEM_COMMIT, windows_memory::PAGE_READWRITE).is_null()
}

/// Gives back a reservation `reserve_pages` made, committed pages and all.
#[cfg(windows)]
pub unsafe fn unreserve_pages(at: *mut u8) {
    windows_memory::VirtualFree(at as *mut std::ffi::c_void, 0, windows_memory::MEM_RELEASE);
}

#[cfg(windows)]
mod windows_memory {
    use std::ffi::c_void;
    pub const MEM_COMMIT: u32 = 0x1000;
    pub const MEM_RESERVE: u32 = 0x2000;
    pub const MEM_RELEASE: u32 = 0x8000;
    pub const PAGE_READWRITE: u32 = 0x04;
    #[link(name = "kernel32")]
    extern "system" {
        pub fn VirtualAlloc(at: *mut c_void, len: usize, kind: u32, protection: u32) -> *mut c_void;
        pub fn VirtualFree(at: *mut c_void, len: usize, kind: u32) -> i32;
    }
}

/// The process's physical footprint on Darwin (the kernel's ledger), zero elsewhere.
pub fn footprint() -> usize {
    os::footprint()
}

/// A thread's first call in a resident process: registers what it hands over when it ends.
pub fn enter() {
    if RESIDENT.load(Ordering::Relaxed) {
        let _ = HANDBACK.try_with(|_| ());
    }
}

/// `std::thread::spawn` for a thread of the compiler.
pub fn spawn<F, T>(f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        enter();
        f()
    })
}

/// `Scope::spawn` for a thread of the compiler.
pub fn spawn_in<'scope, F, T>(scope: &'scope std::thread::Scope<'scope, '_>, f: F) -> std::thread::ScopedJoinHandle<'scope, T>
where
    F: FnOnce() -> T + Send + 'scope,
    T: Send + 'scope,
{
    scope.spawn(move || {
        enter();
        f()
    })
}

/// For a thread of a resident process that lives across builds, where it goes idle: what its
/// lists hold beyond a batch goes to the centre.
pub fn settle() {
    if !RESIDENT.load(Ordering::Relaxed) {
        return;
    }
    // SAFETY: nothing else borrows the state, the allocator's entries do not nest.
    unsafe { release_lists(&mut *state(), true) }
}

#[cold]
#[inline(never)]
unsafe fn alloc_system(layout: Layout) -> *mut u8 {
    large(layout.size(), 0);
    if is_mapped(layout.size(), layout.align()) {
        MAPPED_LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        return map(layout.size());
    }
    System.alloc(layout)
}

#[cold]
#[inline(never)]
unsafe fn dealloc_system(ptr: *mut u8, layout: Layout) {
    large(0, layout.size());
    if is_mapped(layout.size(), layout.align()) {
        MAPPED_LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        return unmap(ptr, layout.size());
    }
    System.dealloc(ptr, layout)
}

#[cold]
#[inline(never)]
unsafe fn realloc_system(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    large(new_size, layout.size());
    match (is_mapped(layout.size(), layout.align()), is_mapped(new_size, layout.align())) {
        (false, false) => System.realloc(ptr, layout, new_size),
        (true, true) => {
            MAPPED_LIVE.fetch_add(new_size, Ordering::Relaxed);
            MAPPED_LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            remap(ptr, layout.size(), new_size)
        }
        (was, _) => {
            let new = if was {
                MAPPED_LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
                System.alloc(Layout::from_size_align_unchecked(new_size, layout.align()))
            } else {
                MAPPED_LIVE.fetch_add(new_size, Ordering::Relaxed);
                map(new_size)
            };
            if !new.is_null() {
                std::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                if was {
                    unmap(ptr, layout.size());
                } else {
                    System.dealloc(ptr, layout);
                }
            }
            new
        }
    }
}

fn large(grown: usize, shrunk: usize) {
    let now = LARGE_LIVE.fetch_add(grown, Ordering::Relaxed) + grown;
    LARGE_PEAK.fetch_max(now, Ordering::Relaxed);
    LARGE_LIVE.fetch_sub(shrunk, Ordering::Relaxed);
}

/// What the allocator holds, in bytes but for the counts.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Held {
    /// Taken from the system for blocks: the chunks, and the single blocks of ending threads.
    pub reserved: usize,
    pub chunks: usize,
    /// Free at the centre: the blocks of its chains and its regions.
    pub centre_free: usize,
    /// The requests over 32 KB: live now, and the most since `forget_peak`.
    pub large: usize,
    pub large_peak: usize,
    /// Of them the ones this process maps itself, and the address space of the mappings
    /// kept at the centre for the next request.
    pub mapped: usize,
    pub mapped_kept: usize,
    pub mappings_made: usize,
    /// How often a thread took a chain of the centre, and a run carved anew.
    pub chains_taken: usize,
    pub runs_carved: usize,
}

/// The pages resident in what the allocator holds, by owner: the chunks, the mappings kept
/// and the mappings live, beside the process's whole resident size. Linux only, by
/// `/proc/self/pagemap`; zeros elsewhere.
#[derive(Clone, Copy, Default, Debug)]
pub struct ResidentPages {
    pub rss: usize,
    pub chunks: usize,
    pub kept: usize,
    pub mapped: usize,
}

#[cfg(not(target_os = "linux"))]
pub fn resident_pages() -> ResidentPages {
    ResidentPages::default()
}

/// The bytes of a range's pages that are resident, by `/proc/self/pagemap`.
#[cfg(target_os = "linux")]
fn present_in(pagemap: &mut std::fs::File, buf: &mut Vec<u8>, at: usize, len: usize) -> usize {
    use std::io::{Read, Seek, SeekFrom};
    buf.clear();
    buf.resize(len / PAGE * 8, 0);
    if pagemap.seek(SeekFrom::Start((at / PAGE * 8) as u64)).is_err() || pagemap.read_exact(buf).is_err() {
        return 0;
    }
    buf.chunks_exact(8).filter(|word| word[7] & 0x80 != 0).count() * PAGE
}

/// The resident bytes of one range: zero where the system does not say.
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
pub fn resident_in(at: usize, len: usize) -> usize {
    #[cfg(target_os = "linux")]
    {
        let Ok(mut pagemap) = std::fs::File::open("/proc/self/pagemap") else { return 0 };
        return present_in(&mut pagemap, &mut Vec::new(), at, len);
    }
    #[allow(unreachable_code)]
    {
        let _ = (at, len);
        0
    }
}

#[cfg(target_os = "linux")]
pub fn resident_pages() -> ResidentPages {
    let rss = std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1).and_then(|p| p.parse::<usize>().ok()))
        .unwrap_or(0)
        * PAGE;
    let Ok(mut pagemap) = std::fs::File::open("/proc/self/pagemap") else {
        return ResidentPages { rss, ..ResidentPages::default() };
    };
    let mut buf: Vec<u8> = Vec::new();
    let mut present = |at: usize, len: usize| -> usize { present_in(&mut pagemap, &mut buf, at, len) };
    let chunks = (0..CHUNKS.load(Ordering::Relaxed).min(CHUNK_LIST_LEN))
        .map(|i| CHUNK_LIST[i].load(Ordering::Relaxed))
        .filter(|&at| at != 0)
        .map(|at| present(at, CHUNK_SIZE))
        .sum();
    let mut kept_list = [(0usize, 0usize); KEPT_LIST];
    let mut live_list = [(0usize, 0usize); LIVE_MAPS];
    let kept_n = centre(|c| {
        let mut k = 0;
        for tier in 0..TIERS {
            for list in [c.fresh[tier], c.stale[tier]] {
                let mut mapping = list;
                while !mapping.is_null() {
                    if k < KEPT_LIST {
                        kept_list[k] = (mapping as usize, 1 << tier);
                        k += 1;
                    }
                    // SAFETY: a kept mapping's second word is the next one.
                    mapping = unsafe { *second(mapping) as *mut u8 };
                }
            }
        }
        live_list = c.live_maps;
        (k, c.live_n)
    });
    let kept = kept_list[..kept_n.0].iter().map(|&(at, len)| present(at, len)).sum();
    let mapped = live_list[..kept_n.1].iter().map(|&(at, len)| present(at, len)).sum();
    ResidentPages { rss, chunks, kept, mapped }
}

pub fn held() -> Held {
    let (centre_free, mapped_kept) = centre(|c| (c.region_bytes + (0..CLASSES).map(|class| c.blocks[class] * size_of_class(class)).sum::<usize>(), c.kept_bytes));
    let chunks = CHUNKS.load(Ordering::Relaxed);
    let large = LARGE_LIVE.load(Ordering::Relaxed);
    Held {
        reserved: chunks * CHUNK_SIZE + STRAY_BYTES.load(Ordering::Relaxed),
        chunks,
        centre_free,
        large,
        large_peak: LARGE_PEAK.load(Ordering::Relaxed).max(large),
        mapped: MAPPED_LIVE.load(Ordering::Relaxed),
        mapped_kept,
        mappings_made: MAPPINGS_MADE.load(Ordering::Relaxed),
        chains_taken: CHAINS_TAKEN.load(Ordering::Relaxed),
        runs_carved: RUNS_CARVED.load(Ordering::Relaxed),
    }
}

/// Starts the most of the requests over 32 KB anew, from what is live.
pub fn forget_peak() {
    LARGE_PEAK.store(LARGE_LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// The free bytes the calling thread holds: its lists and the rest of its region.
pub fn thread_free() -> usize {
    // SAFETY: as in `settle`.
    unsafe {
        let s = &*state();
        let mut bytes = s.end as usize - s.bump as usize;
        for class in 0..CLASSES {
            let mut block = s.free[class];
            while !block.is_null() {
                bytes += size_of_class(class);
                block = *next(block);
            }
        }
        bytes
    }
}

unsafe impl GlobalAlloc for TeqAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !is_small(layout.size(), layout.align()) {
            return alloc_system(layout);
        }
        let (class, size) = class_of(layout.size());
        take(&mut *state(), class, size)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !is_small(layout.size(), layout.align()) {
            return dealloc_system(ptr, layout);
        }
        let (class, _) = class_of(layout.size());
        give(&mut *state(), ptr, class)
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let small_before = is_small(layout.size(), layout.align());
        let small_after = is_small(new_size, layout.align());
        if !small_before && !small_after {
            return realloc_system(ptr, layout, new_size);
        }
        if small_before && small_after && class_of(layout.size()).0 == class_of(new_size).0 {
            return ptr;
        }
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

/// One test at a time among those that read or set the allocator's state of the process
/// (the resident flag, the centre's counts), in a resident process: the allocator's own tests
/// and the interpreter registry's, which registers in a resident alone.
#[cfg(test)]
pub(crate) fn serial_test() -> std::sync::MutexGuard<'static, ()> {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    resident();
    guard
}

#[cfg(test)]
mod tests {
    //! The tests share the process's allocator with the harness and the tests of other
    //! modules, whose threads take chunks and hand back blocks of any class meanwhile. So
    //! they run one at a time, each in a size class of its own, and assert what holds
    //! whatever the others do: bounds far under what the traffic would take without the
    //! centre.
    use super::*;
    use std::cell::Cell;
    use std::collections::HashSet;
    use std::sync::mpsc::channel;
    use std::sync::MutexGuard;

    /// One test at a time, in a resident process (`serial_test`).
    fn serial() -> MutexGuard<'static, ()> {
        super::serial_test()
    }

    unsafe fn block(size: usize, fill: u8) -> usize {
        let p = TeqAlloc.alloc(Layout::from_size_align(size, 8).unwrap());
        assert!(!p.is_null());
        std::ptr::write_bytes(p, fill, size);
        p as usize
    }

    unsafe fn filled(p: usize, size: usize, fill: u8) -> bool {
        std::slice::from_raw_parts(p as *const u8, size).iter().all(|&b| b == fill)
    }

    unsafe fn free(p: usize, size: usize) {
        TeqAlloc.dealloc(p as *mut u8, Layout::from_size_align(size, 8).unwrap())
    }

    fn centre_blocks(size: usize) -> usize {
        centre(|c| c.blocks[class_of(size).0])
    }

    #[test]
    fn classes_and_sizes_agree() {
        for class in 0..CLASSES {
            let size = size_of_class(class);
            assert_eq!(class_of(size), (class, size));
            assert_eq!(class_of(size - 1), (class, size));
        }
    }

    #[test]
    fn a_rest_is_carved_into_blocks_that_cover_it() {
        for len in [16, 4080, 1040, 2048 + 16, 40_000, 3 * MAX_SIZE + 48] {
            let buffer = vec![0u8; len + SMALL_STEP].leak();
            let start = buffer.as_mut_ptr().wrapping_add(buffer.as_ptr().align_offset(SMALL_STEP));
            let mut s = EMPTY;
            unsafe { carve(&mut s, start, len) };
            let mut spans = Vec::new();
            for class in 0..CLASSES {
                let mut b = s.free[class];
                while !b.is_null() {
                    spans.push((b as usize, size_of_class(class)));
                    b = unsafe { *next(b) };
                }
            }
            spans.sort_unstable();
            let mut at = start as usize;
            for (from, size) in spans {
                assert_eq!(from, at, "a rest of {len}");
                at += size;
            }
            assert_eq!(at, start as usize + len);
        }
    }

    #[test]
    fn a_producer_and_a_consumer_that_both_stay_alive() {
        let _serial = serial();
        const SIZE: usize = 1008;
        const BLOCKS: usize = 2000;
        const ROUNDS: usize = 100;
        let before = held().chunks;
        let (to_consumer, inbox) = channel::<Vec<usize>>();
        let (done, acknowledged) = channel::<()>();
        let consumer = std::thread::spawn(move || {
            for (round, blocks) in inbox.into_iter().enumerate() {
                for p in blocks {
                    unsafe {
                        assert!(filled(p, SIZE, round as u8), "a block handed out twice");
                        free(p, SIZE);
                    }
                }
                settle();
                done.send(()).unwrap();
            }
        });
        for round in 0..ROUNDS {
            let blocks: Vec<usize> = (0..BLOCKS).map(|_| unsafe { block(SIZE, round as u8) }).collect();
            to_consumer.send(blocks).unwrap();
            acknowledged.recv().unwrap();
        }
        drop(to_consumer);
        consumer.join().unwrap();
        // 200 MB went from one thread to the other, fifty chunks; a round is 2 MB.
        let taken = held().chunks - before;
        assert!(taken <= 12, "{taken} chunks taken from the system");
    }

    #[test]
    fn a_thread_ends_holding_blocks_that_another_frees_later() {
        let _serial = serial();
        const SIZE: usize = 992;
        const BLOCKS: usize = 1000;
        let first: Vec<usize> = std::thread::spawn(|| (0..BLOCKS).map(|_| unsafe { block(SIZE, 7) }).collect()).join().unwrap();
        for &p in &first {
            unsafe {
                assert!(filled(p, SIZE, 7));
                free(p, SIZE);
            }
        }
        settle();
        let second: Vec<usize> = std::thread::spawn(|| (0..BLOCKS).map(|_| unsafe { block(SIZE, 8) }).collect()).join().unwrap();
        let known: HashSet<usize> = first.iter().copied().collect();
        let again = second.iter().filter(|p| known.contains(p)).count();
        // All but the batch this thread kept.
        assert!(again >= BLOCKS * 9 / 10, "{again} of {BLOCKS} blocks came back");
        assert_eq!(second.iter().copied().collect::<HashSet<usize>>().len(), BLOCKS);
        for p in second {
            unsafe { free(p, SIZE) };
        }
    }

    #[test]
    fn a_thread_that_ends_hands_back_its_lists_and_its_region() {
        let _serial = serial();
        const SIZE: usize = 976;
        const BLOCKS: usize = 1000;
        let work = || {
            let blocks: Vec<usize> = (0..BLOCKS).map(|_| unsafe { block(SIZE, 9) }).collect();
            for &p in &blocks {
                unsafe { free(p, SIZE) };
            }
            blocks
        };
        let before = held().chunks;
        let first = std::thread::spawn(work).join().unwrap();
        assert!(centre_blocks(SIZE) >= BLOCKS);
        let known: HashSet<usize> = first.into_iter().collect();
        for _ in 0..200 {
            let blocks = std::thread::spawn(work).join().unwrap();
            // A block of the class that another thread carved from a rest may be among them.
            let again = blocks.iter().filter(|p| known.contains(p)).count();
            assert!(again >= BLOCKS * 9 / 10, "{again} of {BLOCKS} blocks came back");
        }
        // 200 MB in all, fifty chunks.
        let taken = held().chunks - before;
        assert!(taken <= 12, "{taken} chunks taken from the system by threads that came and went");
    }

    #[test]
    fn a_process_that_ends_gives_its_threads_regions() {
        const SIZE: usize = 928;
        let _serial = serial();
        RESIDENT.store(false, Ordering::Relaxed);
        let (blocks, region) = std::thread::spawn(|| {
            let blocks: Vec<usize> = (0..100).map(|_| unsafe { block(SIZE, 5) }).collect();
            (blocks, STATE.with(|s| unsafe { (*s.get()).end as usize - (*s.get()).bump as usize }))
        })
        .join()
        .unwrap();
        resident();
        // Bumped one after the other from a chunk that is the thread's.
        assert!(region > 0);
        assert!(blocks.windows(2).all(|w| w[1] == w[0] + SIZE));
        for p in blocks {
            unsafe { free(p, SIZE) };
        }
    }

    #[test]
    fn a_run_is_carved_for_its_class_alone() {
        const SIZE: usize = 912;
        const OTHER: usize = 896;
        let _serial = serial();
        std::thread::spawn(|| {
            // The centre has blocks of the other class and none of this one.
            let others: Vec<usize> = (0..100).map(|_| unsafe { block(OTHER, 6) }).collect();
            for &p in &others {
                unsafe { free(p, OTHER) };
            }
            STATE.with(|s| unsafe { release_lists(&mut *s.get(), false) });
            let first = unsafe { block(SIZE, 6) };
            assert!(STATE.with(|s| unsafe { (*s.get()).bump.is_null() }), "a resident thread took a region");
            // The other class comes from its runs again: the blocks it had of them, or the
            // ones that follow a block it had within a run's length.
            let again: Vec<usize> = (0..100).map(|_| unsafe { block(OTHER, 6) }).collect();
            let back = again.iter().filter(|&&p| others.iter().any(|&o| p >= o && p < o + RUN_BYTES)).count();
            assert!(back >= 90, "{back} of 100 blocks came back");
            unsafe { free(first, SIZE) };
            for p in again {
                unsafe { free(p, OTHER) };
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn settling_keeps_a_batch() {
        let _serial = serial();
        const SIZE: usize = 960;
        const BLOCKS: usize = 1000;
        std::thread::spawn(|| {
            let blocks: Vec<usize> = (0..BLOCKS).map(|_| unsafe { block(SIZE, 1) }).collect();
            for p in blocks {
                unsafe { free(p, SIZE) };
            }
            // The blocks freed and the rest of the last run carved.
            let listed = STATE.with(|s| unsafe {
                let mut n = 0;
                let mut b = (*s.get()).free[class_of(SIZE).0];
                while !b.is_null() {
                    n += 1;
                    b = *next(b);
                }
                n
            });
            assert!(listed >= BLOCKS);
            let held_before = thread_free();
            let at_centre = centre_blocks(SIZE);
            settle();
            let kept = BATCH_BYTES / SIZE;
            assert_eq!(centre_blocks(SIZE) - at_centre, listed - kept);
            assert!(held_before - thread_free() >= (listed - kept) * SIZE);
            // The batch is on the list still: no block of the centre is taken for it.
            let again: Vec<usize> = (0..kept).map(|_| unsafe { block(SIZE, 2) }).collect();
            assert_eq!(centre_blocks(SIZE) - at_centre, listed - kept);
            for p in again {
                unsafe { free(p, SIZE) };
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn chains_of_the_smallest_blocks_come_back_whole() {
        let _serial = serial();
        // A block of 16 bytes holds the two words a chain needs and nothing else.
        const BLOCKS: usize = 5000;
        let batch = BATCH_BYTES / SMALL_STEP;
        let buffer = vec![0u8; (BLOCKS + 1) * SMALL_STEP].leak();
        let start = buffer.as_mut_ptr().wrapping_add(buffer.as_ptr().align_offset(SMALL_STEP));
        let mut giver = EMPTY;
        for i in 0..BLOCKS {
            unsafe { give(&mut giver, start.wrapping_add(i * SMALL_STEP), 0) };
        }
        unsafe { release_lists(&mut giver, false) };
        assert!(giver.free[0].is_null());
        let mut taker = EMPTY;
        let first = unsafe { refill(&mut taker, 0, SMALL_STEP) };
        let mut seen = HashSet::new();
        seen.insert(first as usize);
        let mut b = taker.free[0];
        while !b.is_null() {
            assert!(seen.insert(b as usize), "a block twice in a chain");
            b = unsafe { *next(b) };
        }
        assert!(seen.len() <= batch);
        unsafe { release_lists(&mut taker, false) };
    }

    /// The mapping a shrink comes back to is the one the centre kept for its size, which any other
    /// thread's request of that size takes first: the test runs alone in a process of its own, where
    /// no other test's allocations reach the centre.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_mapping_grows_in_place_and_comes_back() {
        const ALONE: &str = "TEQ_ALLOC_TEST_ALONE";
        if std::env::var_os(ALONE).is_none() {
            let run = std::process::Command::new(std::env::current_exe().expect("the test binary"))
                .args(["--exact", "alloc::tests::a_mapping_grows_in_place_and_comes_back", "--test-threads=1"])
                .env(ALONE, "1")
                .output()
                .expect("the test's own process");
            assert!(run.status.success(), "the test in its own process: {}\n{}{}", run.status, String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
            return;
        }
        let _serial = serial();
        const SIZE: usize = 3 << 20;
        unsafe {
            let block = map(SIZE);
            assert!(!block.is_null() && block as usize % SMALL_STEP == 0);
            std::ptr::write_bytes(block, 7, SIZE);
            // Within its room, and beyond.
            assert_eq!(remap(block, SIZE, 2 * SIZE), block);
            let moved = remap(block, 2 * SIZE, 16 * SIZE);
            assert!(moved != block && filled(moved as usize, SIZE, 7));
            // A small part of its length moves to a mapping of its own size: the first one,
            // which was kept for the next request of its size.
            let shrunk = remap(moved, 16 * SIZE, SIZE);
            assert_eq!(shrunk, block);
            assert!(filled(shrunk as usize, SIZE, 7));
            let kept = held().mapped_kept;
            unmap(shrunk, SIZE);
            assert!(held().mapped_kept > kept);
            // What is kept through two idle points is gone.
            age();
            let kept = held().mapped_kept;
            age();
            assert!(held().mapped_kept < kept);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_freed_mappings_pages_past_its_block_are_given_back() {
        const SIZE: usize = 3 << 20;
        unsafe {
            let block = map(SIZE);
            let mapping = block.sub(MAPPED_HEADER) as usize;
            let len = *(mapping as *const usize);
            std::ptr::write_bytes(block, 3, SIZE);
            let touched = resident_in(mapping, len);
            // Shrunk in place: the pages past the new size go.
            assert_eq!(remap(block, SIZE, SIZE / 2), block);
            let shrunk = resident_in(mapping, len);
            // Freed with a small block: what is left is that block's pages and the header's.
            unmap(block, 64 << 10);
            let freed = resident_in(mapping, len);
            if cfg!(target_os = "linux") {
                assert!(touched >= SIZE, "{touched} of {SIZE} resident after the write");
                assert!(shrunk <= SIZE / 2 + 2 * PAGE, "{shrunk} resident after the shrink");
                assert!(freed <= (64 << 10) + 2 * PAGE, "{freed} resident after the free");
            }
            age();
            age();
        }
    }

    #[test]
    fn a_freed_mappings_pages_leave_the_footprint_on_darwin() {
        if !cfg!(target_os = "macos") {
            return;
        }
        const SIZE: usize = 16 << 20;
        unsafe {
            let block = map(SIZE);
            std::ptr::write_bytes(block, 5, SIZE);
            let before = os::footprint();
            unmap(block, 64 << 10);
            let after = os::footprint();
            assert!(before > 0 && before - after >= SIZE / 2, "the footprint went from {before} to {after} for {SIZE} bytes given back");
            age();
            age();
        }
    }

    #[test]
    fn a_thread_that_only_frees_hands_them_over() {
        let _serial = serial();
        const SIZE: usize = 880;
        const BLOCKS: usize = 2000;
        let blocks: Vec<usize> = (0..BLOCKS).map(|_| unsafe { block(SIZE, 1) }).collect();
        let before = centre_blocks(SIZE);
        spawn(move || {
            let lists_before = thread_free();
            for &p in &blocks {
                unsafe { free(p, SIZE) };
            }
            assert!(thread_free() >= lists_before + BLOCKS * SIZE);
        })
        .join()
        .unwrap();
        assert_eq!(centre_blocks(SIZE), before + BLOCKS);
    }

    /// The places of the compiler that start a thread or a process: the file, how often
    /// `spawn` or a name that begins with `spawn_` stands in its code, and the entry its
    /// threads go through.
    const STARTS: &[(&str, usize, &str)] = &[
        ("src/alloc.rs", 4, "spawn and spawn_in themselves, which enter first"),
        ("src/classpath.rs", 1, "alloc::spawn_in"),
        ("src/crew.rs", 2, "a builder for the merge threads, alloc::enter the first statement of the closure; the retired data's free through spawn"),
        ("src/emit/layout.rs", 2, "alloc::spawn_in for the scan of the anonymous classes' creations and for their shapes"),
        ("src/emit/mod.rs", 1, "alloc::spawn_in"),
        ("src/emit/outline.rs", 1, "alloc::spawn_in"),
        ("src/frontend.rs", 1, "alloc::spawn_in"),
        ("src/jvm/classfile.rs", 1, "alloc::spawn_in"),
        ("src/jvm/kept.rs", 1, "alloc::spawn_in for the slices of the kept class files' facts"),
        ("src/jvm/mod.rs", 2, "alloc::spawn_in for the class generation and for the class files' writing"),
        ("src/lsp/export.rs", 2, "alloc::spawn for the thread that waits on sbt; the other starts the sbt process"),
        ("src/lsp/mod.rs", 7, "alloc::spawn six times, the generators' worker among them; the seventh starts a child process"),
        ("src/main.rs", 1, "a builder for the stack's size; alloc::enter is the first statement of compile"),
        ("src/products.rs", 1, "alloc::spawn_in for the publication's steps"),
        ("src/task/client.rs", 1, "starts the daemon process"),
        ("src/task/daemon.rs", 4, "alloc::spawn for the watch, per connection and per test run's client watcher, alloc::spawn_in per resident's build"),
        ("src/task/fetch.rs", 2, "starts curl; alloc::spawn_in per transfer worker of a cold fetch"),
        ("src/task/job.rs", 2, "starts dev's dev command and run's JVM, once per platform's branch; taskkill runs to its end through status"),
        ("src/task/resident.rs", 2, "alloc::spawn for the thread that drains the resident's stderr; the other starts the resident"),
        ("src/task/runner.rs", 2, "alloc::spawn for the thread that drains the test runner's stderr; the other starts the runner's JVM"),
        ("src/typer/thread.rs", 1, "a builder for the stack's size; alloc::enter is the first statement of the closure"),
        ("src/typer/mod.rs", 2, "a builder for the stack's size, alloc::enter the first statement of each worker's closure; the merge's free of the other workers' variables' tables through spawn"),
        ("src/typer/loader/bodies.rs", 1, "a builder for the stack's size, alloc::enter the first statement of the closure"),
        ("src/watch.rs", 1, "alloc::spawn for the relay of a stdin that cannot be peeked, on Windows"),
        ("src/write.rs", 2, "alloc::spawn_in"),
    ];

    /// How often `spawn` or a name that begins with `spawn_` stands in the code of a source,
    /// its comments and its unit tests left out.
    fn starts(source: &str) -> usize {
        let code = source.split("#[cfg(test)]").next().unwrap();
        let mut count = 0;
        for line in code.lines() {
            let line = line.split("//").next().unwrap();
            let bytes = line.as_bytes();
            let mut at = 0;
            while let Some(found) = line[at..].find("spawn") {
                let from = at + found;
                at = from + "spawn".len();
                let inside = from > 0 && (bytes[from - 1].is_ascii_alphanumeric() || bytes[from - 1] == b'_');
                let longer = bytes.get(at).is_some_and(|b| b.is_ascii_alphanumeric());
                if !inside && !longer {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn the_count_of_thread_starts_sees_a_start_however_it_is_written() {
        assert_eq!(starts("use std::thread::spawn as fork;\nfn f() { fork(|| ()); }"), 1);
        assert_eq!(starts("fn f() { std::thread::scope(|s| { s.spawn(|| ()); }); }"), 1);
        assert_eq!(starts("fn f() { std::thread::spawn\n    (|| ()); }"), 1);
        assert_eq!(starts("fn f() { Builder::new().spawn(a); crate::alloc::enter(); Builder::new().spawn(b); }"), 2);
        assert_eq!(starts("fn f() { crate::alloc::spawn_in(scope, || ()); } // spawn\n/// spawn\n"), 1);
        assert_eq!(starts("fn respawn() { let spawned = 1; }\n#[cfg(test)]\nmod tests { fn f() { std::thread::spawn(|| ()); } }"), 0);
    }

    /// A tripwire for a place that starts a thread and is not in the list, not a proof that
    /// the listed places enter as the list says.
    #[test]
    fn the_places_that_start_a_thread_are_the_listed_ones() {
        fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    sources(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        sources(&root.join("src"), &mut files);
        assert!(files.len() > 50);
        for file in files {
            let name = file.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let listed = STARTS.iter().find(|(path, _, _)| *path == name).map_or(0, |&(_, count, _)| count);
            let found = starts(&std::fs::read_to_string(&file).unwrap());
            assert_eq!(found, listed, "{name} starts {found} threads or processes and the list in src/alloc.rs has {listed}: list the place with the entry it goes through (the module's header has the rule)");
        }
    }

    /// Owns blocks that its destructor frees, as a thread-local cache of a thread does.
    struct Owner(Cell<Vec<usize>>);
    const OWNED_SIZE: usize = 864;

    impl Drop for Owner {
        fn drop(&mut self) {
            for p in self.0.take() {
                unsafe { free(p, OWNED_SIZE) };
            }
        }
    }

    thread_local! {
        static OWNER: Owner = const { Owner(Cell::new(Vec::new())) };
    }

    /// The blocks of a class at the centre, by address. The list is sized before the centre's
    /// lock is taken, since nothing allocates under it.
    fn centre_list(size: usize) -> HashSet<usize> {
        let class = class_of(size).0;
        let mut found = Vec::with_capacity(2 * centre_blocks(size) + 4096);
        let whole = centre(|c| unsafe {
            let mut chain = c.chains[class];
            while !chain.is_null() {
                let mut b = chain;
                while !b.is_null() {
                    if found.len() == found.capacity() {
                        return false;
                    }
                    found.push(b as usize);
                    b = *next(b);
                }
                chain = *second(chain) as *mut u8;
            }
            true
        });
        assert!(whole, "the centre's blocks of the class outgrew the list sized for them");
        found.into_iter().collect()
    }

    /// A thread leaves blocks to a thread-local whose destructor frees them: they come back to
    /// the centre when the thread ends, and so does the rest of its list of the class. Each block
    /// is looked for by its address: the centre's count moves with the blocks the thread took
    /// from it before carving any (the class's blocks an earlier run handed back, or a block
    /// another test's thread left there) and with the other tests' threads. From the end of the
    /// thread's allocations to the look, no thread takes a chain of the class from the centre
    /// (`KEPT_CLASS`), so that a block found missing was not given back, rather than given back
    /// and taken again by a thread whose list ran dry (`consumer`, the check's own such thread).
    fn destructor_frees_come_back(consumer: bool) {
        const BLOCKS: usize = 2000;
        let allocated = std::sync::Arc::new(std::sync::Barrier::new(2));
        let kept = std::sync::Arc::new(std::sync::Barrier::new(2));
        let (a, k) = (allocated.clone(), kept.clone());
        let worker = spawn(move || {
            let blocks: Vec<usize> = (0..BLOCKS).map(|_| unsafe { block(OWNED_SIZE, 2) }).collect();
            OWNER.with(|owner| owner.0.set(blocks.clone()));
            a.wait();
            k.wait();
            // The blocks of the class that the thread holds besides: the rest of its last run,
            // read as its last act.
            let mut held = Vec::with_capacity(BLOCKS);
            STATE.with(|s| unsafe {
                let mut b = (*s.get()).free[class_of(OWNED_SIZE).0];
                while !b.is_null() && held.len() < held.capacity() {
                    held.push(b as usize);
                    b = *next(b);
                }
            });
            (blocks, held)
        });
        allocated.wait();
        KEPT_CLASS.store(class_of(OWNED_SIZE).0, Ordering::Release);
        kept.wait();
        let (owned, held) = worker.join().unwrap();
        // A thread whose list of the class is empty, as another test's may be: it carves.
        let taken = consumer.then(|| spawn(|| unsafe { block(OWNED_SIZE, 3) }).join().unwrap());
        let at_centre = centre_list(OWNED_SIZE);
        KEPT_CLASS.store(usize::MAX, Ordering::Release);
        if let Some(p) = taken {
            unsafe { free(p, OWNED_SIZE) };
        }
        let missing = owned.iter().chain(&held).filter(|b| !at_centre.contains(b)).count();
        assert_eq!(missing, 0, "of the {} blocks the destructor freed and the {} the thread held, {} are not at the centre", owned.len(), held.len(), missing);
    }

    #[test]
    fn what_a_thread_locals_destructor_frees_comes_back() {
        let _serial = serial();
        // The second time the centre holds what the first gave back, which the thread takes
        // before it carves anything; the third, a thread of the class's empty list allocates
        // before the look.
        destructor_frees_come_back(false);
        destructor_frees_come_back(false);
        destructor_frees_come_back(true);
    }

    struct Late {
        block: Cell<usize>,
        pristine: Cell<bool>,
    }

    static LATE_RAN: AtomicUsize = AtomicUsize::new(0);
    static LATE_AFTER_HANDBACK: AtomicUsize = AtomicUsize::new(0);
    const LATE_SIZE: usize = 944;

    impl Drop for Late {
        fn drop(&mut self) {
            let ended = HANDBACK.try_with(|_| ()).is_err();
            unsafe {
                // An allocation made during the thread's teardown, and blocks freed there.
                let p = block(LATE_SIZE, 4);
                assert!(filled(self.block.get(), LATE_SIZE, 3));
                free(self.block.get(), LATE_SIZE);
                assert!(filled(p, LATE_SIZE, 4));
                free(p, LATE_SIZE);
                let text = format!("{}{}", "teardown ", self.block.get());
                assert!(text.starts_with("teardown"));
            }
            if self.pristine.get() {
                // Registered before the thread's first allocation, so destroyed after the
                // allocator's own thread-local.
                assert!(ended);
            }
            if ended {
                LATE_AFTER_HANDBACK.fetch_add(1, Ordering::Relaxed);
            }
            LATE_RAN.fetch_add(1, Ordering::Relaxed);
        }
    }

    thread_local! {
        static LATE: Late = const { Late { block: Cell::new(0), pristine: Cell::new(false) } };
    }

    fn pristine() -> bool {
        STATE.with(|s| unsafe {
            let s = &*s.get();
            s.bump.is_null() && s.free.iter().all(|b| b.is_null())
        })
    }

    #[test]
    fn a_thread_local_destroyed_before_and_after_the_allocators_own() {
        let _serial = serial();
        let ran = LATE_RAN.load(Ordering::Relaxed);
        // Registered after the thread's first allocation: destroyed before the allocator's.
        std::thread::spawn(|| {
            let p = unsafe { block(LATE_SIZE, 3) };
            LATE.with(|late| late.block.set(p));
        })
        .join()
        .unwrap();
        // Registered before it: destroyed after, when the thread's memory went to the centre.
        let strays = STRAY_BYTES.load(Ordering::Relaxed);
        let after = LATE_AFTER_HANDBACK.load(Ordering::Relaxed);
        std::thread::spawn(|| {
            let first = pristine();
            LATE.with(|late| late.pristine.set(first));
            let p = unsafe { block(LATE_SIZE, 3) };
            LATE.with(|late| late.block.set(p));
        })
        .join()
        .unwrap();
        assert_eq!(LATE_RAN.load(Ordering::Relaxed), ran + 2);
        if LATE_AFTER_HANDBACK.load(Ordering::Relaxed) > after {
            assert!(STRAY_BYTES.load(Ordering::Relaxed) >= strays + LATE_SIZE);
        }
    }
}
