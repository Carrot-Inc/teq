//! A thread-local size-class allocator with a centre the threads exchange memory through.
//! Blocks up to 32 KB are carved from 4 MB chunks and recycled through per-thread free lists;
//! larger requests go to the system allocator, or in a resident process to mappings of its own
//! (below), each behind a word that says which of the two holds it. The copy inlined at every
//! allocation site is the free-list pop or the bump, and at every free the push: a free costs a compiler as many
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
//! A resident process maps its requests over 32 KB itself (`map`): the arrays of a program, which
//! a full build drops and makes anew, the arrays a build works in, and the tables and buffers its
//! workers grow. The system's allocator keeps such blocks written to once they are freed, for a
//! request of their size to come, in an arena of each thread that freed them, and how much it
//! keeps is not the session's to say: the same hundred edits of a resident check ended at 200 MB
//! in some runs and at 375 MB in others, a thousand wandered between 420 and 750 MB over an
//! account of 163 MB, and a session of eight workers held more than one of one worker by what its
//! workers' arenas kept (docs/SPEED.md, "A session's memory"). A mapping's length is its request's and
//! its header's in whole pages, as the system's allocator maps a chunk; a realloc within it is served in
//! place, a block that grows past it is given the new length by Linux's `mremap` (the difference of
//! address space alone, the pages moved, not copied; elsewhere it moves to a mapping with room to grow,
//! copied), and a shrink unmaps what is past the block (`remap`): a live block holds the address space
//! the system's allocator would hold for it, and under a bound on it (`ulimit -v`) a request or a
//! thread's start is refused where the system's allocator's would be: the band's
//! mappings are carved from 64 MB of address space mapped ahead, as glibc's arenas carve theirs
//! (`from_reserve`), so that the first request a bound refuses is one that needs address space of its
//! own; where the system refuses a request, the mappings kept and the reserve's rest give way
//! (`give_way`), and once a bound refused one, the system's allocator is asked first until the process
//! goes idle (`PRESSED`). A mapping is kept at the centre when it is freed, for the next request of its
//! class (`length_for`, four to a doubling) or of the three below it, or, the first of its class's list,
//! of the class above where its length holds the request, the address space past the next block unmapped
//! (`shed`): a build takes back what it freed, pages and all, but for the kept mapping a block grown by
//! the system passes by, which gives back its pages (`cool_passed`). Where the compiler thread goes idle
//! (`idle`), after what its build freed late, every mapping kept is unmapped: a session at idle holds the
//! address space and the pages of its live blocks alone, besides the rest of the band's reserve (up to
//! 64 MB of address space, none of it written) and what the system refused to unmap at its bound on the
//! count of mappings (a kept mapping, cold, its header page resident, and a range no list names, one page
//! of it, each tried again at the next idle point), and the next build maps what it asks for anew, which it
//! would have written again either way. `teq fix` keeps its mappings' address
//! space across a typing instead (`age`), their pages given back, for the next typing to take.
//!
//! A block over 32 KB at the step's alignment has a word before it (`OWNER`) from the process's
//! start: its mapping's length where the process mapped it, `SYSTEM` where the system's allocator
//! holds it. A free and a realloc read it, never the size or the mode, so the process maps from
//! whenever it becomes resident, whatever it allocated before (its arguments, of which a class
//! path is one line of 50 KB and more), and each block of the system's goes back to it, or to a
//! mapping when it is grown. A request aligned past the step is the system's alone, with no word,
//! which its layout says at every free and realloc. Windows maps nothing: its resident processes
//! keep the system's allocator for every request over 32 KB, as one-shot builds do everywhere.
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
/// A mapping's first words: the one before it on the centre's list of the live mappings, the next one
/// on that list or, while it is kept, on its class's, its length, the word before its block's owner's
/// (`OWNER`), and how far from its start its blocks may have written since its pages were last given
/// back (`extent`).
const MAPPED_HEADER: usize = 2 * SMALL_STEP;
/// The bytes before a block over 32 KB at the step's alignment, whose first word says who holds it:
/// a mapping's header, whose first word is its length, or a block of the system's allocator's of
/// as many bytes more, whose first word is `SYSTEM`.
const OWNER: usize = SMALL_STEP;
/// The owner's word of a block of the system's allocator: no mapping's length.
const SYSTEM: usize = 0;
/// The address space of a mapping made for a block that grows where the system cannot resize a mapping
/// (`os::RESIZES`), against its request: two doublings grow in place.
const MAPPED_ROOM: usize = 4;
/// The address space the mappings of the requests under 1 MB are carved from, mapped ahead (`from_reserve`):
/// glibc's arena heap's on 64-bit (`HEAP_MAX_SIZE`), the address space it reserves ahead of the
/// requests it serves there.
const RESERVE: usize = 64 << 20;
/// The requests carved from the reserve: under it, the band between the small blocks and the arrays.
const RESERVED_UNDER: usize = 1 << 20;
/// A mapping's length is one of four to a doubling (`length_for`): its class, the list it is kept on.
const QUARTERS: usize = 4;
const LENGTHS: usize = usize::BITS as usize * QUARTERS;

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
    /// The kept mappings by class: the warm ones, their freed blocks' pages kept, which a request takes
    /// first, and the cold ones, their pages given back: a mapping a growth passed by (`cool_passed`) or whose
    /// block moved, and in `teq fix` the ones kept through the typing before (`age`).
    fresh: [*mut u8; LENGTHS],
    stale: [*mut u8; LENGTHS],
    kept_bytes: usize,
    kept_n: usize,
    /// What is left of the reserve the band's mappings are carved from (`from_reserve`), and where.
    reserve: *mut u8,
    reserve_left: usize,
    /// The ranges of the allocator's own that no list names, whose unmapping the system refused, held for a
    /// later idle point to unmap (`unmap_range`), linked through a node in each range's last page, and how many.
    orphans: *mut u8,
    orphaned_n: usize,
    /// The live mappings, for `resident_pages`, linked through their headers (`back`, `second`).
    live: *mut u8,
    live_n: usize,
}

impl Centre {
    /// The kept mapping a request of `want` bytes takes, the first of the class below's list where its length
    /// holds the request, or the first of `class`'s (`length_for`'s) or of the three above it, off its list: a
    /// warm one (`fresh`, its block's pages kept) where `warm`, a cold one (`stale`) otherwise; null where none is
    /// kept.
    unsafe fn take_kept(&mut self, class: usize, want: usize, warm: bool) -> *mut u8 {
        if class > 0 {
            let list = if warm { &mut self.fresh[class - 1] } else { &mut self.stale[class - 1] };
            let mapping = *list;
            if !mapping.is_null() && *length(mapping) >= want {
                *list = *second(mapping) as *mut u8;
                self.kept_bytes -= *length(mapping);
                self.kept_n -= 1;
                return mapping;
            }
        }
        for k in class..(class + QUARTERS).min(LENGTHS) {
            let list = if warm { &mut self.fresh[k] } else { &mut self.stale[k] };
            let mapping = *list;
            if !mapping.is_null() {
                *list = *second(mapping) as *mut u8;
                self.kept_bytes -= *length(mapping);
                self.kept_n -= 1;
                return mapping;
            }
        }
        null_mut()
    }

    /// A range `unmap_range` holds, by its node (the range, its length, the next node), onto the list of them.
    unsafe fn hold(&mut self, node: *mut usize) {
        *node.add(2) = self.orphans as usize;
        self.orphans = node as *mut u8;
        self.orphaned_n += 1;
    }

    /// A kept mapping onto its class's list, the warm ones' (`fresh`) or the cold ones'.
    unsafe fn keep(&mut self, mapping: *mut u8, warm: bool) {
        let class = class(*length(mapping));
        let list = if warm { &mut self.fresh[class] } else { &mut self.stale[class] };
        *second(mapping) = *list as usize;
        *list = mapping;
        self.kept_bytes += *length(mapping);
        self.kept_n += 1;
    }

    /// SAFETY: a mapping off every list.
    unsafe fn note_live(&mut self, mapping: *mut u8) {
        *back(mapping) = 0;
        *second(mapping) = self.live as usize;
        if !self.live.is_null() {
            *back(self.live) = mapping as usize;
        }
        self.live = mapping;
        self.live_n += 1;
    }

    /// SAFETY: a mapping on the list of the live ones.
    unsafe fn forget_live(&mut self, mapping: *mut u8) {
        let (before, after) = (*back(mapping) as *mut u8, *second(mapping) as *mut u8);
        if before.is_null() {
            self.live = after;
        } else {
            *second(before) = after as usize;
        }
        if !after.is_null() {
            *back(after) = before as usize;
        }
        self.live_n -= 1;
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
        fresh: [null_mut(); LENGTHS],
        stale: [null_mut(); LENGTHS],
        kept_bytes: 0,
        kept_n: 0,
        reserve: null_mut(),
        reserve_left: 0,
        orphans: null_mut(),
        orphaned_n: 0,
        live: null_mut(),
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
static MAPPING: AtomicBool = AtomicBool::new(false);
static MAPPED_LIVE: AtomicUsize = AtomicUsize::new(0);
static MAPPINGS_MADE: AtomicUsize = AtomicUsize::new(0);
static CHUNKS: AtomicUsize = AtomicUsize::new(0);
static STRAY_BYTES: AtomicUsize = AtomicUsize::new(0);
static CHAINS_TAKEN: AtomicUsize = AtomicUsize::new(0);
static RUNS_CARVED: AtomicUsize = AtomicUsize::new(0);
static LARGE_LIVE: AtomicUsize = AtomicUsize::new(0);
static LARGE_PEAK: AtomicUsize = AtomicUsize::new(0);
static IDLE_US: AtomicUsize = AtomicUsize::new(0);
static IDLE_UNMAPPED: AtomicUsize = AtomicUsize::new(0);
static RESIZES: AtomicUsize = AtomicUsize::new(0);
static KEPT_UNMAPPED: AtomicUsize = AtomicUsize::new(0);
static ADVICE_REFUSED: AtomicUsize = AtomicUsize::new(0);
static UNMAPS_REFUSED: AtomicUsize = AtomicUsize::new(0);
static ADVICE_KEPT: AtomicUsize = AtomicUsize::new(0);
/// Whether the system refused a request under a bound on the address space (`ulimit -v`) since the
/// process last went idle (`asked`): from then on a new block over 32 KB is asked of the system's
/// allocator before it is mapped (`alloc_system`, `realloc_system`). A request refused with no bound (a
/// size no address space holds) leaves the order as it is.
static PRESSED: AtomicBool = AtomicBool::new(false);
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

/// The first word of a mapping: the one before it on the list of the live mappings.
#[inline(always)]
unsafe fn back(mapping: *mut u8) -> *mut usize {
    mapping as *mut usize
}

/// A mapping's length, the word before its block's owner's.
unsafe fn length(mapping: *mut u8) -> *mut usize {
    mapping.add(MAPPED_HEADER - OWNER) as *mut usize
}

/// The bytes from a mapping's start its blocks may have written since its pages were last given back:
/// what `release_past` gives back, a system call saved where nothing lies past the block.
unsafe fn extent(mapping: *mut u8) -> *mut usize {
    mapping.add(MAPPED_HEADER - OWNER + WORD) as *mut usize
}

/// The second word of a block: the next chain in a chain's first block, the chain's length in
/// its second; a region's end; the next mapping on a mapping's list.
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
        let chunk = asked(|| System.alloc(Layout::from_size_align_unchecked(CHUNK_SIZE, SMALL_STEP)));
        if chunk.is_null() {
            return stray(size);
        }
        note_chunk(chunk);
        s.bump = chunk.add(size);
        s.end = chunk.add(CHUNK_SIZE);
        return chunk;
    }
    // Touching `HANDBACK` registers its destructor. Past its destruction the thread is
    // ending and nobody would hand over what it took, so it takes one block of the system.
    if HANDBACK.try_with(|_| ()).is_err() {
        return stray(size);
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
                let chunk = asked(|| System.alloc(Layout::from_size_align_unchecked(CHUNK_SIZE, SMALL_STEP)));
                if chunk.is_null() {
                    return stray(size);
                }
                note_chunk(chunk);
                give_region(chunk, CHUNK_SIZE);
            }
        }
    }
}

/// One block of the system's for a small request, where a chunk is refused (a bound on the address space
/// that has room for the block and not for 4 MB) or the thread is past its hand-back: it joins the lists
/// of whoever frees it.
unsafe fn stray(size: usize) -> *mut u8 {
    let block = asked(|| System.alloc(Layout::from_size_align_unchecked(size, SMALL_STEP)));
    if !block.is_null() {
        STRAY_BYTES.fetch_add(size, Ordering::Relaxed);
    }
    block
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
    // A block's word says who holds it (`OWNER`), so the blocks of the system's allocator live
    // now go back to it whenever they are freed.
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        MAPPING.store(true, Ordering::Relaxed);
    }
}

/// For `teq fix` between two typings: the cold kept mappings go back to the system (those kept through the
/// typing before, a mapping whose block moved out of it, one a growth passed by; where the system refuses to
/// unmap one, at its bound on the count of mappings, it stays cold, its header page resident, for the next
/// age or idle point, and so do the ranges `unmap_range` held), and the warm ones the last
/// typing freed stay for the next, their address space alone: their pages are given back, off the lists
/// meanwhile so that no request takes one while its pages go, and they are cold from then on. A block grown by the
/// system (`resize`) takes no kept mapping, so the mappings freed after their own growth are taken again
/// by no typing, and with their pages they held `teq fix`'s peak a third over the allocator's that grew by
/// moving into room (the memory line's `peak-fix-frontend@1`). A mapping whose pages the system will not take
/// back (a page of it locked) is unmapped instead, and counted (`advice_refused`), or kept cold where that
/// unmapping is refused as well (`unmap_or_keep`).
pub fn age() {
    PRESSED.store(false, Ordering::Relaxed);
    let lists = centre(|c| {
        let lists = [std::mem::replace(&mut c.stale, [null_mut(); LENGTHS]), std::mem::replace(&mut c.fresh, [null_mut(); LENGTHS])];
        for list in &lists {
            for &first in list {
                let mut mapping = first;
                while !mapping.is_null() {
                    // SAFETY: a kept mapping's header holds its length, its second word the next one.
                    unsafe {
                        c.kept_bytes -= *length(mapping);
                        mapping = *second(mapping) as *mut u8;
                    }
                    c.kept_n -= 1;
                }
            }
        }
        lists
    });
    let [stale, kept] = lists;
    for &first in &stale {
        let mut mapping = first;
        while !mapping.is_null() {
            // SAFETY: the mapping is off the centre's lists and this thread's alone.
            unsafe {
                let after = *second(mapping) as *mut u8;
                unmap_or_keep(mapping);
                mapping = after;
            }
        }
    }
    // The pages given back, the mapping kept cold; a mapping whose advice the system refuses (a page of it
    // locked) is unmapped instead, counted, rather than kept with its pages through the next typing.
    for &first in &kept {
        let mut mapping = first;
        while !mapping.is_null() {
            // SAFETY: as above; the header page stays, with the list's link.
            unsafe {
                let after = *second(mapping) as *mut u8;
                if release_past(mapping, 0) {
                    unmap_refused(mapping);
                } else {
                    centre(|c| c.keep(mapping, false));
                }
                mapping = after;
            }
        }
    }
    unmap_orphans();
}

/// A mapping of `len` bytes carved from the reserve, which a new one replaces where it is short, its rest
/// unmapped; a mapping of its own where the system refuses a reserve. The reserve's address space is the
/// process's ahead of the requests it serves, as the system allocator's arenas are, so that under a bound
/// on the address space (`ulimit -v`) the first request the system refuses is not one of these but the
/// next that needs address space of its own (a thread's stack); each mapping carved is unmapped alone.
unsafe fn from_reserve(len: usize) -> *mut u8 {
    let take = |c: &mut Centre| {
        if c.reserve_left < len {
            return null_mut();
        }
        let at = c.reserve;
        c.reserve = at.add(len);
        c.reserve_left -= len;
        at
    };
    let carved = centre(take);
    if !carved.is_null() {
        return carved;
    }
    let fresh = os::map(RESERVE);
    if fresh.is_null() {
        return os::map(len);
    }
    let (rest, rest_len) = centre(|c| {
        let old = (c.reserve, c.reserve_left);
        c.reserve = fresh.add(len);
        c.reserve_left = RESERVE - len;
        old
    });
    if rest_len > 0 {
        unmap_range(rest, rest_len);
    }
    fresh
}

/// Every mapping kept for a later request unmapped (`idle`, and where the system refuses a request
/// address space, `give_way`): how many.
#[cold]
#[inline(never)]
fn unmap_kept() -> usize {
    let taken = centre(|c| {
        let lists = [std::mem::replace(&mut c.fresh, [null_mut(); LENGTHS]), std::mem::replace(&mut c.stale, [null_mut(); LENGTHS])];
        for list in &lists {
            for &first in list {
                let mut mapping = first;
                while !mapping.is_null() {
                    // SAFETY: a kept mapping's header holds its length, its second word the next one.
                    unsafe {
                        c.kept_bytes -= *length(mapping);
                        mapping = *second(mapping) as *mut u8;
                    }
                    c.kept_n -= 1;
                }
            }
        }
        lists
    });
    let mut n = 0;
    for list in &taken {
        for &first in list {
            let mut mapping = first;
            while !mapping.is_null() {
                // SAFETY: the mapping is off the centre's lists and this thread's alone.
                unsafe {
                    let after = *second(mapping) as *mut u8;
                    if unmap_or_keep(mapping) {
                        n += 1;
                    }
                    mapping = after;
                }
            }
        }
    }
    n
}

/// A request to the system (`ask`, null where it is refused), asked again once the mappings kept and the
/// reserve's rest gave way where the system refused it (`give_way`): every request of the allocator's and
/// of its pages' callers goes through it, so that what the process keeps for later requests never stands
/// in the way of one the system would serve without it.
pub(crate) fn asked<T>(mut ask: impl FnMut() -> *mut T) -> *mut T {
    let got = ask();
    if !got.is_null() {
        return got;
    }
    if !PRESSED.load(Ordering::Relaxed) && os::bounded() {
        PRESSED.store(true, Ordering::Relaxed);
    }
    if give_way() {
        ask()
    } else {
        got
    }
}

/// Where the system refuses a request address space (a bound on it, `ulimit -v`, or on its count of
/// mappings): the mappings kept, the reserve's rest and the ranges held unmapped, for the request to be asked
/// again, as free space of the system's allocator would serve it; whether any was. What the system refuses
/// to unmap stays the allocator's (`unmap_or_keep`, `unmap_orphans`): the reserve's rest is the reserve again,
/// or, where another thread mapped a new reserve meanwhile, held as a range no list names (`unmap_range`).
#[cold]
#[inline(never)]
fn give_way() -> bool {
    let (rest, rest_len) = centre(|c| (std::mem::replace(&mut c.reserve, null_mut()), std::mem::take(&mut c.reserve_left)));
    let mut rest_unmapped = false;
    if rest_len > 0 {
        // SAFETY: the reserve's rest is no mapping's, off the centre now and this thread's alone.
        unsafe {
            if os::unmap(rest, rest_len) {
                rest_unmapped = true;
            } else {
                UNMAPS_REFUSED.fetch_add(1, Ordering::Relaxed);
                let restored = centre(|c| {
                    if c.reserve_left > 0 {
                        return false;
                    }
                    (c.reserve, c.reserve_left) = (rest, rest_len);
                    true
                });
                if !restored {
                    unmap_range(rest, rest_len);
                }
            }
        }
    }
    let kept = unmap_kept();
    let orphans = unmap_orphans();
    let any = kept > 0 || rest_unmapped || orphans > 0;
    if any {
        KEPT_UNMAPPED.fetch_add(1, Ordering::Relaxed);
    }
    any
}

/// For a resident process's thread where the process goes idle, after what its last build freed late
/// (the interpreter's disposal, the drops handed to threads of their own, `crew::give_back`'s and the
/// overlays' that `crew::keep_thread` keeps): every
/// mapping kept for a later request is unmapped. A build reuses the mappings its own frees keep, their
/// pages and all; across builds a kept mapping would hold its address space for the next build, whose
/// requests map anew at the cost of a system call each, the pages written either way. So a session at
/// idle holds the address space and the pages of its live blocks alone, besides the rest of the band's
/// reserve, address space none of whose pages were written, which the next build's band requests take. Where
/// the system refuses an unmapping (at its bound on the count of mappings), the kept mapping stays kept, cold,
/// its header page resident (`unmap_or_keep`), and a range no list names stays held, one page of it resident
/// (`unmap_orphans`), for the next idle point to try again.
pub fn idle() {
    let start = std::time::Instant::now();
    PRESSED.store(false, Ordering::Relaxed);
    IDLE_UNMAPPED.fetch_add(unmap_kept(), Ordering::Relaxed);
    unmap_orphans();
    IDLE_US.store(start.elapsed().as_micros() as usize, Ordering::Relaxed);
}

/// Whether the process maps its requests over 32 KB at the step's alignment itself: a resident
/// process does from `resident` on; what holds a block made before is its word's to say
/// (`is_mapping`).
#[inline]
fn maps() -> bool {
    MAPPING.load(Ordering::Relaxed)
}

/// Whether a block over 32 KB at the step's alignment is a mapping of the process's, by its word.
unsafe fn is_mapping(block: *mut u8) -> bool {
    *(block.sub(OWNER) as *const usize) != SYSTEM
}

/// A block of the system's allocator behind the word that says so; null where the system refuses it.
unsafe fn system_block(size: usize) -> *mut u8 {
    let Some(layout) = size.checked_add(OWNER).and_then(|len| Layout::from_size_align(len, SMALL_STEP).ok()) else {
        return null_mut();
    };
    let start = asked(|| System.alloc(layout));
    if start.is_null() {
        return start;
    }
    *(start as *mut usize) = SYSTEM;
    start.add(OWNER)
}

/// The layout the system's allocator holds a block of `system_block`'s under.
unsafe fn system_layout(size: usize) -> Layout {
    Layout::from_size_align_unchecked(size + OWNER, SMALL_STEP)
}

/// A mapping for a block that grows: of its own length on Linux, which `mremap` grows; elsewhere with
/// room for two more doublings, or where the system refuses that room (a bound on the address space), of
/// its own length, the room being for speed alone.
unsafe fn map_growing(size: usize) -> *mut u8 {
    let roomy = map(size, true);
    if roomy.is_null() && !os::RESIZES {
        map(size, false)
    } else {
        roomy
    }
}

/// The least length of a mapping's at `want` bytes or more, and its class: four to a doubling (2^e and
/// 1.25, 1.5 and 1.75 times it), each a whole number of the system's pages, which a length of the
/// four rounded up to pages is again.
fn length_for(want: usize) -> Option<(usize, usize)> {
    let e = usize::BITS - 1 - want.max(1).leading_zeros();
    let base = 1usize << e;
    let step = (base / QUARTERS).max(1);
    let len = base.checked_add((want - base).div_ceil(step) * step)?.checked_next_multiple_of(os::page())?;
    Some((class(len), len))
}

/// The class of one of `length_for`'s lengths.
fn class(len: usize) -> usize {
    let e = (usize::BITS - 1 - len.leading_zeros()) as usize;
    e * QUARTERS + ((len - (1 << e)) >> e.saturating_sub(2))
}

/// The mapping of a request, kept or made, its header written (`length`, `extent`) and on the
/// list of the live ones: the block after the header, in a kept mapping of the request's class (`length_for`)
/// or of the three above it within a doubling, or in the first of the class below's list where its length
/// holds the request, where one is kept, the warm ones first, or in one made anew.
/// Either way its length is the request's and the header's in whole pages, or `MAPPED_ROOM` times the
/// request's where the block grows on a system that cannot resize a mapping (`room`): the address space of a
/// block that never grows is its own, and a thread's start or another request under a bound on the address
/// space is not refused for room no block uses.
unsafe fn map(size: usize, room: bool) -> *mut u8 {
    let want = if room && !os::RESIZES { size.checked_mul(MAPPED_ROOM) } else { Some(size) };
    let Some(want) = want.and_then(|n| n.checked_add(MAPPED_HEADER)) else {
        return null_mut();
    };
    // The least class whose mappings all hold the request: a kept mapping of it or of the three above it
    // serves it; a mapping made anew is the request's own length in whole pages, as the system's
    // allocator maps a chunk, no address space past it but the last page's rest.
    let Some((class, _)) = length_for(want) else {
        return null_mut();
    };
    let Some(least) = want.checked_next_multiple_of(os::page()) else {
        return null_mut();
    };
    // The first kept mapping of the class below's list where its length holds the request, or of the class or of
    // the three above it, within a doubling of the length (`take_kept`): address space mapped already, what is past
    // the block unmapped below.
    let kept = centre(|c| {
        let warm = c.take_kept(class, want, true);
        if warm.is_null() {
            c.take_kept(class, want, false)
        } else {
            warm
        }
    });
    // The address space past the request's pages unmapped (past its room's, where it grows on a system that cannot
    // resize a mapping), as the system's allocator unmaps a chunk freed and maps the next request's length, and
    // the pages an earlier, larger block wrote past this one given back: a live block holds its own length's
    // address space and its own pages alone. A mapping whose pages the system will not take back is not taken.
    let kept = if kept.is_null() {
        kept
    } else {
        shed(kept, *length(kept), want - MAPPED_HEADER);
        // The pages this request takes, given back at the mapping's last free or shrink, taken again; those
        // past it stay given back, outside the extent no later release looks past (on Darwin a page taken
        // again counts in the footprint until it is given back again, and one the system will not take back
        // is not written).
        let page = os::page();
        let used = (MAPPED_HEADER + size).div_ceil(page) * page;
        if release_past(kept, size) || (used > page && !os::reuse(kept.add(page), used - page)) {
            unmap_refused(kept);
            null_mut()
        } else {
            kept
        }
    };
    let len = if kept.is_null() { least } else { *length(kept) };
    let mapping = if kept.is_null() {
        let made = asked(|| if size < RESERVED_UNDER { from_reserve(len) } else { os::map(len) });
        if !made.is_null() {
            MAPPINGS_MADE.fetch_add(1, Ordering::Relaxed);
        }
        made
    } else {
        kept
    };
    if mapping.is_null() {
        return null_mut();
    }
    *length(mapping) = len;
    if kept.is_null() {
        *extent(mapping) = MAPPED_HEADER + size;
    } else {
        *extent(mapping) = (*extent(mapping)).max(MAPPED_HEADER + size);
    }
    centre(|c| c.note_live(mapping));
    mapping.add(MAPPED_HEADER)
}

/// Gives the pages of a mapping that its blocks wrote past `size` back to the system (`extent`),
/// their contents the system's to drop (Darwin keeps them readable until it needs the pages;
/// `alloc_zeroed` clears explicitly, and no path takes a mapping for zero): what a freed block wrote
/// past its size while it was larger, which would stay resident through the mapping's later lives
/// otherwise. Whether the system refused the advice (a page of the range locked), the pages kept.
unsafe fn release_past(mapping: *mut u8, size: usize) -> bool {
    let page = os::page();
    let used = (MAPPED_HEADER + size).div_ceil(page) * page;
    let written = (*extent(mapping)).div_ceil(page) * page;
    if written <= used {
        return false;
    }
    let released = os::release(mapping.add(used), written - used);
    if released {
        *extent(mapping) = MAPPED_HEADER + size;
    }
    !released
}

/// A mapping off every list whose pages the system will not take back (a page of it locked): unmapped rather
/// than kept, or taken again, with them, and counted (`advice_refused`), or, where the unmapping is refused as
/// well, kept cold for a later idle point (`unmap_or_keep`). Every site that gives a kept mapping's pages back
/// ends here on a refusal: a free (`unmap`), a growth passing one by (`cool_passed`), a request taking one
/// (`map`) and `teq fix`'s age point (`age`).
unsafe fn unmap_refused(mapping: *mut u8) {
    if unmap_or_keep(mapping) {
        ADVICE_REFUSED.fetch_add(1, Ordering::Relaxed);
    }
}

/// A kept mapping off every list unmapped, or, where the system refuses (at its bound on the count of
/// mappings, an unmapping inside a larger mapping would split it), kept the allocator's still: its pages
/// past the header given back and the mapping kept cold, for a later idle point to unmap again or a request
/// to take. Whether it was unmapped. Every unmapping of a kept mapping goes through it: the idle point and
/// give-way (`unmap_kept`), `teq fix`'s age point and a refused advice (`unmap_refused`).
unsafe fn unmap_or_keep(mapping: *mut u8) -> bool {
    if os::unmap(mapping, *length(mapping)) {
        return true;
    }
    UNMAPS_REFUSED.fetch_add(1, Ordering::Relaxed);
    if release_past(mapping, 0) {
        ADVICE_KEPT.fetch_add(1, Ordering::Relaxed);
    }
    centre(|c| c.keep(mapping, false));
    false
}

/// A range of the allocator's own that no list names (the reserve's rest a new reserve replaced, the page
/// count's list, a stack, a directory of zeroed pages) unmapped, or, where the system refuses, held for a
/// later idle point to unmap (`unmap_orphans`): its pages but the last given back, the last holding its place
/// on the centre's list (the range, its length, the next one), the one page of it that stays resident; the
/// last page, since a stack's first is its guard. Whether it was unmapped.
unsafe fn unmap_range(at: *mut u8, len: usize) -> bool {
    if os::unmap(at, len) {
        return true;
    }
    UNMAPS_REFUSED.fetch_add(1, Ordering::Relaxed);
    let page = os::page();
    let len = len.next_multiple_of(page);
    if len > page && !os::release(at, len - page) {
        ADVICE_KEPT.fetch_add(1, Ordering::Relaxed);
    }
    let node = at.add(len - page) as *mut usize;
    *node = at as usize;
    *node.add(1) = len;
    centre(|c| c.hold(node));
    false
}

/// The ranges `unmap_range` held, unmapped where the system now takes them, the rest held again: how many went.
fn unmap_orphans() -> usize {
    let mut node = centre(|c| {
        c.orphaned_n = 0;
        std::mem::replace(&mut c.orphans, null_mut()) as *mut usize
    });
    let mut n = 0;
    while !node.is_null() {
        // SAFETY: a held range's node, off the centre's list: the range, its length, the next node.
        unsafe {
            let (at, len, next) = (*node as *mut u8, *node.add(1), *node.add(2) as *mut usize);
            if os::unmap(at, len) {
                n += 1;
            } else {
                UNMAPS_REFUSED.fetch_add(1, Ordering::Relaxed);
                centre(|c| c.hold(node));
            }
            node = next;
        }
    }
    n
}

/// Keeps a freed mapping at the centre, the pages past the block given back: warm, its block's pages kept for
/// the next request of its class, or cold where the block's contents moved (`size` 0).
unsafe fn unmap(block: *mut u8, size: usize) {
    let mapping = block.sub(MAPPED_HEADER);
    let refused = release_past(mapping, size);
    centre(|c| {
        c.forget_live(mapping);
        if !refused {
            c.keep(mapping, size > 0);
        }
    });
    if refused {
        unmap_refused(mapping);
    }
}

/// The kept mapping a request of `size` would take where a growth to `size` passes it by (`resize`): its pages
/// given back, its address space kept for a later request, cold. A growth by the system takes no kept mapping
/// (taking one would ask for no address space where the bounded typing's thread starts need the address space
/// to grow, `typing/starts/met`), so without this a mapping freed after its block's growth stays kept with its
/// pages until the idle point, the next block grown past it, and the next: a text the language server builds
/// by growth, eighty versions of it in a second, held eighty texts.
unsafe fn cool_passed(size: usize) {
    let Some(want) = size.checked_add(MAPPED_HEADER) else {
        return;
    };
    let Some((class, _)) = length_for(want) else {
        return;
    };
    let passed = centre(|c| c.take_kept(class, want, true));
    if passed.is_null() {
        return;
    }
    if release_past(passed, 0) {
        unmap_refused(passed);
    } else {
        centre(|c| c.keep(passed, false));
    }
}

/// A mapping for a new size, by one rule: a size within the mapping's length is served in place, with no
/// new address space, growth included (the pages a shrink gave back taken again) and a shrink unmapping
/// the address space past the block's pages; a size beyond it is given its length in whole pages by the
/// system on Linux (`mremap`, which needs the difference of address space alone, moving the pages where
/// the mapping cannot grow in place), and elsewhere moves to a mapping with room for two more doublings,
/// copied. Where the system refuses, null, the block as it was.
unsafe fn remap(block: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
    let mapping = block.sub(MAPPED_HEADER);
    let len = *length(mapping);
    if new_size + MAPPED_HEADER <= len {
        if new_size <= old_size {
            // Off the list of the live ones while its length changes, which the page count reads under the
            // centre's lock (`resident_pages`), as `resize` does; the same size changes nothing.
            if new_size < old_size && (MAPPED_HEADER + new_size).next_multiple_of(os::page()) < len {
                centre(|c| c.forget_live(mapping));
                shed(mapping, len, new_size);
                centre(|c| c.note_live(mapping));
            }
            return block;
        }
        // Pages an earlier shrink gave back, taken again; where the system will not take them back (Darwin's
        // `MADV_FREE_REUSE`), the block moves instead, below.
        let page = os::page();
        let from = (MAPPED_HEADER + old_size) / page * page;
        if os::reuse(mapping.add(from), (MAPPED_HEADER + new_size).div_ceil(page) * page - from) {
            *extent(mapping) = (*extent(mapping)).max(MAPPED_HEADER + new_size);
            return block;
        }
        ADVICE_KEPT.fetch_add(1, Ordering::Relaxed);
    }
    if os::RESIZES {
        cool_passed(new_size);
        return resize(mapping, len, new_size);
    }
    let new = map_growing(new_size);
    if !new.is_null() {
        std::ptr::copy_nonoverlapping(block, new, old_size);
        // The block's contents have moved: its pages go back with the mapping kept, which would
        // otherwise hold the array's old written pages resident until it is taken or aged (the
        // class bodies' array past 16,384 records, 5.5 MB of a resident check).
        unmap(block, 0);
    }
    new
}

/// The address space of a mapping past its block's new size, in whole pages, unmapped: a shrink in place
/// keeps no more address space than a mapping made for its size has. Where the system refuses (at its bound
/// on the count of mappings), the mapping keeps its length and the pages past the block are given back.
unsafe fn shed(mapping: *mut u8, len: usize, new_size: usize) {
    let short = (MAPPED_HEADER + new_size).next_multiple_of(os::page());
    if short < len {
        if os::unmap(mapping.add(short), len - short) {
            *length(mapping) = short;
            *extent(mapping) = (*extent(mapping)).min(short);
        } else {
            UNMAPS_REFUSED.fetch_add(1, Ordering::Relaxed);
            if release_past(mapping, new_size) {
                ADVICE_KEPT.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// A live mapping given a longer length by the system (`os::resize`), the new size's in whole pages:
/// off the list of the live ones meanwhile, whose neighbours name its address. Null where the system
/// refuses, the mapping as it was.
unsafe fn resize(mapping: *mut u8, len: usize, new_size: usize) -> *mut u8 {
    let Some(want) = new_size.checked_add(MAPPED_HEADER) else {
        return null_mut();
    };
    let Some(new_len) = want.checked_next_multiple_of(os::page()) else {
        return null_mut();
    };
    centre(|c| c.forget_live(mapping));
    let moved = asked(|| os::resize(mapping, len, new_len));
    let at = if moved.is_null() { mapping } else { moved };
    if !moved.is_null() {
        *length(at) = new_len;
        *extent(at) = (*extent(at)).max(want);
        RESIZES.fetch_add(1, Ordering::Relaxed);
    }
    centre(|c| c.note_live(at));
    if moved.is_null() {
        null_mut()
    } else {
        at.add(MAPPED_HEADER)
    }
}

/// Anonymous memory from the system and back to it.
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod os {
    use std::ffi::c_void;

    extern "C" {
        fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
        fn munmap(addr: *mut c_void, len: usize) -> i32;
        fn madvise(addr: *mut c_void, len: usize, advice: i32) -> i32;
        #[cfg(feature = "threads")]
        fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    }

    /// The first `len` bytes from `at` made inaccessible (a stack's guard), whether they are.
    #[cfg(feature = "threads")]
    pub unsafe fn protect_none(at: *mut u8, len: usize) -> bool {
        mprotect(at as *mut c_void, len, 0) == 0
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

    /// Whether the system unmapped the range: it refuses at its bound on the count of mappings
    /// (`vm.max_map_count`) where the unmapping would split a mapping in two.
    pub unsafe fn unmap(at: *mut u8, len: usize) -> bool {
        munmap(at as *mut c_void, len) == 0
    }

    /// The bytes a block of the system's allocator holds, at its start (glibc's and musl's
    /// `malloc_usable_size`, Darwin's `malloc_size`): what its realloc serves in place.
    pub unsafe fn usable(at: *mut u8) -> usize {
        extern "C" {
            #[cfg(target_os = "linux")]
            fn malloc_usable_size(at: *mut c_void) -> usize;
            #[cfg(target_os = "macos")]
            fn malloc_size(at: *const c_void) -> usize;
        }
        #[cfg(target_os = "linux")]
        return malloc_usable_size(at as *mut c_void);
        #[cfg(target_os = "macos")]
        return malloc_size(at as *const c_void);
    }

    /// Whether the process has a bound on its address space (`ulimit -v`).
    pub fn bounded() -> bool {
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
        }
        #[cfg(target_os = "linux")]
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux
        #[cfg(target_os = "macos")]
        const RLIMIT_AS: i32 = 5; // libc's resource number on Darwin
        #[cfg(target_os = "linux")]
        const RLIM_INFINITY: u64 = u64::MAX; // libc's RLIM_INFINITY on Linux
        #[cfg(target_os = "macos")]
        const RLIM_INFINITY: u64 = i64::MAX as u64; // libc's RLIM_INFINITY on Darwin
        let mut limit = Rlimit { cur: 0, max: 0 };
        // SAFETY: the system fills a struct of its layout.
        unsafe { getrlimit(RLIMIT_AS, &mut limit) == 0 && limit.cur < RLIM_INFINITY }
    }

    /// Whether the system gives a mapping a new length, its pages moved where it moves (`resize`).
    pub const RESIZES: bool = cfg!(target_os = "linux");

    /// A mapping given a longer length, both whole pages: Linux's `mremap`, which moves it and its pages
    /// where it cannot grow in place (`MREMAP_MAYMOVE`), needing the difference of address space alone;
    /// null where the system refuses, the mapping as it was. Darwin has none (`RESIZES`).
    #[cfg(target_os = "linux")]
    pub unsafe fn resize(at: *mut u8, len: usize, new_len: usize) -> *mut u8 {
        extern "C" {
            fn mremap(old: *mut c_void, old_len: usize, new_len: usize, flags: i32, ...) -> *mut c_void;
        }
        const MREMAP_MAYMOVE: i32 = 1;
        let moved = mremap(at as *mut c_void, len, new_len, MREMAP_MAYMOVE);
        if moved as isize == -1 {
            std::ptr::null_mut()
        } else {
            moved as *mut u8
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub unsafe fn resize(_: *mut u8, _: usize, _: usize) -> *mut u8 {
        std::ptr::null_mut()
    }

    /// The pages of a range given back, the range still mapped; whether the system took the advice.
    pub unsafe fn release(at: *mut u8, len: usize) -> bool {
        madvise(at as *mut c_void, len, RELEASE_ADVICE) == 0
    }

    /// A range given back by `release` taken again: on Darwin its pages count in the footprint once
    /// more (`MADV_FREE_REUSE`, which libmalloc gives before it reuses a range it made reusable); nothing
    /// on Linux, whose pages are the system's again when first written.
    /// Whether the system took the advice (always on Linux).
    pub unsafe fn reuse(at: *mut u8, len: usize) -> bool {
        #[cfg(target_os = "macos")]
        return madvise(at as *mut c_void, len, 8) == 0;
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (at, len);
            true
        }
    }

    /// The system's page, asked once (`getpagesize`): 4 KB on x86-64 Linux and Intel macOS, 16 KB on
    /// Apple's arm64, 4, 16 or 64 KB on arm64 Linux by its kernel.
    pub fn page() -> usize {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static PAGE: AtomicUsize = AtomicUsize::new(0);
        extern "C" {
            fn getpagesize() -> i32;
        }
        let known = PAGE.load(Ordering::Relaxed);
        if known != 0 {
            return known;
        }
        // SAFETY: no argument, no state.
        let page = (unsafe { getpagesize() } as usize).max(4096);
        PAGE.store(page, Ordering::Relaxed);
        page
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

    pub unsafe fn unmap(_: *mut u8, _: usize) -> bool {
        true
    }

    pub fn bounded() -> bool {
        false
    }

    pub const RESIZES: bool = false;

    pub unsafe fn resize(_: *mut u8, _: usize, _: usize) -> *mut u8 {
        std::ptr::null_mut()
    }

    pub unsafe fn usable(_: *mut u8) -> usize {
        0
    }

    pub unsafe fn release(_: *mut u8, _: usize) -> bool {
        true
    }

    pub unsafe fn reuse(_: *mut u8, _: usize) -> bool {
        true
    }

    pub fn page() -> usize {
        4096
    }

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
        asked(|| unsafe { os::map(len) })
    } else {
        asked(|| unsafe { System.alloc_zeroed(Layout::from_size_align_unchecked(len, SMALL_STEP)) })
    }
}

/// A coroutine's stack (`interp/threads.rs`): `len` bytes of address space mapped without a reservation
/// of swap, each page the system's as the stack first touches it, the lowest `guard` bytes inaccessible;
/// null where the system refuses the mapping (a limit of the address space) or its guard (the split of the
/// mapping, at the bound on the count of mappings), the two asked again together once what the allocator
/// keeps gave way (`asked`). Given back by `free_zeroed_pages`.
#[cfg(all(feature = "threads", any(target_os = "macos", target_os = "linux")))]
pub fn stack_pages(len: usize, guard: usize) -> *mut u8 {
    asked(|| unsafe {
        let at = os::map(len);
        if at.is_null() || os::protect_none(at, guard) {
            return at;
        }
        unmap_range(at, len);
        std::ptr::null_mut()
    })
}

/// Gives back what `zeroed_pages` gave.
pub unsafe fn free_zeroed_pages(at: *mut u8, len: usize) {
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        unmap_range(at, len);
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
    let page = os::page();
    let start = at as usize;
    let begin = (start + page - 1) & !(page - 1);
    let end = ((start + len + page - 1) & !(page - 1)).min((start + room) & !(page - 1));
    // Where the system refuses (a page locked), the pages stay the vector's, counted.
    if end > begin && !os::release(begin as *mut u8, end - begin) {
        ADVICE_KEPT.fetch_add(1, Ordering::Relaxed);
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

/// `spawn` for a thread that only frees what it is given: where the system gives no thread (its stack's address space),
/// the closure is dropped on the calling thread with what it holds, as the free it stood for, and there is no handle.
pub fn spawn_or_drop<F>(f: F) -> Option<std::thread::JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .spawn(move || {
            enter();
            f()
        })
        .ok()
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
    let size = layout.size();
    let block = if layout.align() > SMALL_STEP {
        asked(|| System.alloc(layout))
    } else if maps() {
        // Where the system refuses a mapping, a block of its allocator's, which may serve it from space
        // it holds free; where the system has refused a request since the process last went idle (a bound
        // on the address space), that block first, the mapping where it is refused, as the system's
        // allocator alone would serve the request from that space before it asked for more.
        let pressed = PRESSED.load(Ordering::Relaxed);
        let first = if pressed { system_block(size) } else { null_mut() };
        if !first.is_null() {
            first
        } else {
            let block = map(size, false);
            if block.is_null() {
                if pressed {
                    block
                } else {
                    system_block(size)
                }
            } else {
                MAPPED_LIVE.fetch_add(size, Ordering::Relaxed);
                block
            }
        }
    } else {
        system_block(size)
    };
    if !block.is_null() {
        large(size, 0);
    }
    block
}

#[cold]
#[inline(never)]
unsafe fn dealloc_system(ptr: *mut u8, layout: Layout) {
    let size = layout.size();
    large(0, size);
    if layout.align() > SMALL_STEP {
        System.dealloc(ptr, layout)
    } else if is_mapping(ptr) {
        MAPPED_LIVE.fetch_sub(size, Ordering::Relaxed);
        unmap(ptr, size)
    } else {
        System.dealloc(ptr.sub(OWNER), system_layout(size))
    }
}

/// A block over 32 KB for another size over 32 KB: the old one's owner by its word; a mapping stays one,
/// a system block grows into a mapping in a process that maps and stays the system's otherwise. Null where
/// the system refuses every way, the old block and the account as they were.
#[cold]
#[inline(never)]
unsafe fn realloc_system(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    let size = layout.size();
    if layout.align() > SMALL_STEP {
        let new = asked(|| System.realloc(ptr, layout, new_size));
        if !new.is_null() {
            large(new_size, size);
        }
        return new;
    }
    let was = is_mapping(ptr);
    // A system block in place, by the system's realloc, its word moved with it.
    let in_place = || match new_size.checked_add(OWNER).filter(|&len| Layout::from_size_align(len, SMALL_STEP).is_ok()) {
        Some(len) => {
            let start = asked(|| System.realloc(ptr.sub(OWNER), system_layout(size), len));
            if start.is_null() {
                start
            } else {
                start.add(OWNER)
            }
        }
        None => null_mut(),
    };
    // A mapping stays one: the process maps from `resident` on and never stops. A system block stays the
    // system's within its usable size (the system's realloc serves it in place) and moves to a mapping
    // when it grows past it in a process that maps, or, where the system refuses that mapping, grows as
    // the system's realloc grows it (in place, or by its own mapping's move, which needs no second copy
    // of the block's address space); where the system has refused a request since the process last went
    // idle, the system's realloc first and the mapping where it is refused (`alloc_system`).
    let (new, mapped) = if was {
        let moved = remap(ptr, size, new_size);
        if moved.is_null() {
            // Refused every way a mapping grows: a block of the system's allocator, which may serve it from
            // space it holds free, the contents copied, as the system's realloc moves a block.
            let block = system_block(new_size);
            if !block.is_null() {
                std::ptr::copy_nonoverlapping(ptr, block, size.min(new_size));
                unmap(ptr, 0);
            }
            (block, false)
        } else {
            (moved, true)
        }
    } else if maps() && new_size + OWNER > os::usable(ptr.sub(OWNER)).max(size + OWNER) {
        let pressed = PRESSED.load(Ordering::Relaxed);
        let first = if pressed { in_place() } else { null_mut() };
        let moved = if first.is_null() { map_growing(new_size) } else { null_mut() };
        if !first.is_null() {
            (first, false)
        } else if moved.is_null() {
            (if pressed { moved } else { in_place() }, false)
        } else {
            std::ptr::copy_nonoverlapping(ptr, moved, size);
            System.dealloc(ptr.sub(OWNER), system_layout(size));
            (moved, true)
        }
    } else {
        (in_place(), false)
    };
    if !new.is_null() {
        large(new_size, size);
        if mapped {
            MAPPED_LIVE.fetch_add(new_size, Ordering::Relaxed);
        }
        if was {
            MAPPED_LIVE.fetch_sub(size, Ordering::Relaxed);
        }
    }
    new
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
    /// Of them the ones this process maps itself; the address space of the mappings kept at the
    /// centre for the next request and their count; the mappings taken from the system.
    pub mapped: usize,
    pub mapped_kept: usize,
    pub kept_mappings: usize,
    pub mappings_made: usize,
    /// The mappings the idle points unmapped (`idle`), how long the last took, in microseconds, and the
    /// mappings the system gave a longer length (`resize`).
    pub idle_unmapped: usize,
    pub idle_us: usize,
    pub resizes: usize,
    /// How often what the allocator keeps gave way to a request the system refused (`give_way`): the mappings
    /// kept, the reserve's rest or the ranges held, any of them unmapped.
    pub kept_unmapped: usize,
    /// The mappings unmapped because the system refused to take their pages back (a page of one locked): at a
    /// free, where a growth passes one by, where a request takes one, and at `teq fix`'s age point
    /// (`unmap_refused`).
    pub advice_refused: usize,
    /// How often the system refused an unmapping (at its bound on the count of mappings, one inside a larger
    /// mapping would split it), the range kept the allocator's: a kept mapping (`unmap_or_keep`) and a range no
    /// list names (`unmap_range`) for the next idle point, a shrunk block's tail (`shed`) until the block is
    /// freed, the reserve's rest (`give_way`) as the reserve; and how many ranges no list names are held now
    /// (`orphaned`).
    pub unmaps_refused: usize,
    pub orphaned: usize,
    /// How often the system refused an advice and the pages stayed: a vector's unused tail (`release_pages`),
    /// the pages of a kept mapping whose unmapping was refused (`unmap_or_keep`), of a shrunk block's tail
    /// (`shed`) and of a range held (`unmap_range`), and on macOS the pages a block grows into in place
    /// (`remap`, which moves the block instead).
    pub advice_kept: usize,
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

/// The bytes of a range's pages that are resident, by `/proc/self/pagemap`, read a small block's
/// worth of entries at a time: a request over 32 KB would be a mapping the look itself frees, kept
/// for the next build and counted by the next look.
#[cfg(target_os = "linux")]
fn present_in(pagemap: &mut std::fs::File, buf: &mut Vec<u8>, at: usize, len: usize) -> usize {
    use std::io::{Read, Seek, SeekFrom};
    let page = os::page();
    let (mut first, mut pages, mut present) = (at / page, len / page, 0);
    while pages > 0 {
        let n = pages.min(MAX_SIZE / 8);
        buf.clear();
        buf.resize(n * 8, 0);
        if pagemap.seek(SeekFrom::Start((first * 8) as u64)).is_err() || pagemap.read_exact(buf).is_err() {
            return present;
        }
        present += buf.chunks_exact(8).filter(|word| word[7] & 0x80 != 0).count() * page;
        first += n;
        pages -= n;
    }
    present
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
        * os::page();
    let Ok(mut pagemap) = std::fs::File::open("/proc/self/pagemap") else {
        return ResidentPages { rss, ..ResidentPages::default() };
    };
    // A small block's worth from the start: grown by `resize` from less, it could double past 32 KB.
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_SIZE);
    let mut present = |at: usize, len: usize| -> usize { present_in(&mut pagemap, &mut buf, at, len) };
    let chunks = (0..CHUNKS.load(Ordering::Relaxed).min(CHUNK_LIST_LEN))
        .map(|i| CHUNK_LIST[i].load(Ordering::Relaxed))
        .filter(|&at| at != 0)
        .map(|at| present(at, CHUNK_SIZE))
        .sum();
    // The mappings, address and length, read under the centre's lock into a list sized before it is
    // taken, since nothing allocates under it, and again where the mappings outgrew it. The list is a
    // mapping of the system's made and unmapped here, never one of the allocator's: one of those (a
    // list past 32 KB, some 1,400 mappings) would join the lists it counts, and stay kept once freed.
    let span = std::mem::size_of::<(usize, usize)>();
    let mut room = 0;
    let mut list: *mut (usize, usize) = null_mut();
    let (n_kept, n_live) = loop {
        let wanted = centre(|c| c.kept_n + c.live_n) * span;
        if wanted > room {
            // SAFETY: the list's own mapping, this function's alone.
            unsafe {
                if !list.is_null() {
                    unmap_range(list as *mut u8, room);
                }
                room = wanted.div_ceil(os::page()) * os::page();
                list = os::map(room) as *mut (usize, usize);
            }
            if list.is_null() {
                return ResidentPages { rss, chunks, ..ResidentPages::default() };
            }
        }
        let read = centre(|c| {
            if (c.kept_n + c.live_n) * span > room {
                return None;
            }
            let mut n = 0;
            // SAFETY: a mapping's second word is the next one on its list, its length in its header;
            // the list has room for every mapping counted.
            unsafe {
                for class in 0..LENGTHS {
                    for head in [c.fresh[class], c.stale[class]] {
                        let mut mapping = head;
                        while !mapping.is_null() {
                            *list.add(n) = (mapping as usize, *length(mapping));
                            n += 1;
                            mapping = *second(mapping) as *mut u8;
                        }
                    }
                }
                let kept = n;
                let mut mapping = c.live;
                while !mapping.is_null() {
                    *list.add(n) = (mapping as usize, *length(mapping));
                    n += 1;
                    mapping = *second(mapping) as *mut u8;
                }
                Some((kept, n - kept))
            }
        });
        if let Some(counts) = read {
            break counts;
        }
    };
    let (mut kept, mut mapped) = (0, 0);
    for i in 0..n_kept + n_live {
        // SAFETY: the first `n_kept + n_live` entries were written above.
        let (at, len) = unsafe { *list.add(i) };
        *(if i < n_kept { &mut kept } else { &mut mapped }) += present(at, len);
    }
    if !list.is_null() {
        // SAFETY: as above.
        unsafe { unmap_range(list as *mut u8, room) };
    }
    ResidentPages { rss, chunks, kept, mapped }
}

pub fn held() -> Held {
    let (centre_free, mapped_kept, kept_mappings) = centre(|c| (c.region_bytes + (0..CLASSES).map(|class| c.blocks[class] * size_of_class(class)).sum::<usize>(), c.kept_bytes, c.kept_n));
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
        kept_mappings,
        mappings_made: MAPPINGS_MADE.load(Ordering::Relaxed),
        idle_unmapped: IDLE_UNMAPPED.load(Ordering::Relaxed),
        idle_us: IDLE_US.load(Ordering::Relaxed),
        resizes: RESIZES.load(Ordering::Relaxed),
        kept_unmapped: KEPT_UNMAPPED.load(Ordering::Relaxed),
        advice_refused: ADVICE_REFUSED.load(Ordering::Relaxed),
        unmaps_refused: UNMAPS_REFUSED.load(Ordering::Relaxed),
        orphaned: centre(|c| c.orphaned_n),
        advice_kept: ADVICE_KEPT.load(Ordering::Relaxed),
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
    //! they run one at a time, each in a size class of its own, and the ones that count a
    //! class's blocks or the chunks taken, or set the mode, in a process of their own
    //! (`alone`), where no other module's test reaches the centre; and assert bounds far
    //! under what the traffic would take without the centre.
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

    /// A mapping's lengths: the least of the four to a doubling at the bytes wanted or more, whole
    /// pages, each its own class, in order.
    #[test]
    fn a_mappings_length_is_the_least_of_its_class() {
        let page = os::page();
        let mut last = (0, 0);
        let mut want = MAX_SIZE;
        while want < (1 << 34) {
            let (class, len) = length_for(want).unwrap();
            assert!(len >= want && len % page == 0, "{want}: {len}");
            assert!(len - want < want / QUARTERS + page, "{want}: {len}");
            assert_eq!(length_for(len), Some((class, len)), "{len} is its class's length");
            assert!(class >= last.0 && (class > last.0 || len == last.1), "{want}: class {class} after {last:?}");
            last = (class, len);
            want += want / 7 + 1;
        }
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
        if !alone("a_producer_and_a_consumer_that_both_stay_alive") {
            return;
        }
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
        if !alone("a_thread_ends_holding_blocks_that_another_frees_later") {
            return;
        }
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
        if !alone("a_thread_that_ends_hands_back_its_lists_and_its_region") {
            return;
        }
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
        if !alone("a_run_is_carved_for_its_class_alone") {
            return;
        }
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
        if !alone("settling_keeps_a_batch") {
            return;
        }
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

    /// A mapping grows (within its room, or on Linux by the system's resize) and past it, and shrinks to a
    /// length of its own size's class, its contents kept; freed, it is kept until two idle points of `teq
    /// fix`'s (`age`):
    /// the test runs alone in a process of its own, where no other test's allocations reach the centre.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_mapping_grows_in_place_and_comes_back() {
        if !alone("a_mapping_grows_in_place_and_comes_back") {
            return;
        }
        let _serial = serial();
        const SIZE: usize = 3 << 20;
        unsafe {
            let block = map(SIZE, true);
            assert!(!block.is_null() && block as usize % SMALL_STEP == 0);
            std::ptr::write_bytes(block, 7, SIZE);
            let grown = remap(block, SIZE, 2 * SIZE);
            assert!(!grown.is_null() && filled(grown as usize, SIZE, 7));
            if !os::RESIZES {
                assert_eq!(grown, block, "grown within its room");
            }
            let bigger = remap(grown, 2 * SIZE, 16 * SIZE);
            assert!(!bigger.is_null() && filled(bigger as usize, SIZE, 7));
            let shrunk = remap(bigger, 16 * SIZE, SIZE);
            assert!(!shrunk.is_null() && filled(shrunk as usize, SIZE, 7));
            assert!(*length(shrunk.sub(MAPPED_HEADER)) < MAPPED_ROOM * SIZE);
            let kept = held().mapped_kept;
            unmap(shrunk, SIZE);
            assert!(held().mapped_kept > kept);
            // What the last of `teq fix`'s typings freed is kept through the next, its address space alone;
            // what is kept through two is gone.
            age();
            let kept = held().mapped_kept;
            if cfg!(target_os = "linux") {
                let (resident, n) = (resident_pages().kept, held().kept_mappings);
                assert!(resident <= n * os::page(), "{resident} resident in {n} mappings kept through a typing");
            }
            age();
            assert!(held().mapped_kept < kept);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_freed_mappings_pages_past_its_block_are_given_back() {
        const SIZE: usize = 3 << 20;
        unsafe {
            let block = map(SIZE, true);
            let mapping = block.sub(MAPPED_HEADER);
            let len = *length(mapping);
            let mapping = mapping as usize;
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
                assert!(shrunk <= SIZE / 2 + 2 * os::page(), "{shrunk} resident after the shrink");
                assert!(freed <= (64 << 10) + 2 * os::page(), "{freed} resident after the free");
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
            let block = map(SIZE, false);
            std::ptr::write_bytes(block, 5, SIZE);
            let before = os::footprint();
            unmap(block, 64 << 10);
            let after = os::footprint();
            assert!(before > 0 && before - after >= SIZE / 2, "the footprint went from {before} to {after} for {SIZE} bytes given back");
            age();
            age();
        }
    }

    /// Runs the calling test, `alloc::tests::<test>`, in a process of its own, where no other test's
    /// allocations move the allocator's counts and the mode is the test's to set: true in that process,
    /// where the test goes on, and false in the harness's once that process passed. The harness's side
    /// starts the process one test at a time with the allocator's own (`serial`) and hands the blocks
    /// it freed doing so (the environment copied for the process, its output) to the centre before it
    /// lets them go on, so that none arrives there while another counts its class's blocks.
    fn alone(test: &str) -> bool {
        alone_in(test, &[])
    }

    /// `alone`, the test's process given the environment `env` besides.
    fn alone_in(test: &str, env: &[(&str, &str)]) -> bool {
        const ALONE: &str = "TEQ_ALLOC_TEST_ALONE";
        if std::env::var_os(ALONE).is_some() {
            return true;
        }
        let _serial = serial();
        let failed = {
            let name = format!("alloc::tests::{test}");
            let run = std::process::Command::new(std::env::current_exe().expect("the test binary"))
                .args(["--exact", &name, "--test-threads=1"])
                .env(ALONE, "1")
                .envs(env.iter().copied())
                .output()
                .expect("the test's own process");
            let out = String::from_utf8_lossy(&run.stdout);
            (!(run.status.success() && out.contains("1 passed"))).then(|| format!("{name} in its own process: {}\n{}{}", run.status, out, String::from_utf8_lossy(&run.stderr)))
        };
        if let Some(why) = failed {
            panic!("{why}");
        }
        // SAFETY: as in `settle`.
        unsafe {
            let s = &mut *state();
            release_region(s);
            release_lists(s, false);
        }
        false
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn layout(size: usize, align: usize) -> Layout {
        Layout::from_size_align(size, align).unwrap()
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    unsafe fn fill(p: *mut u8, size: usize, fill: u8) -> *mut u8 {
        assert!(!p.is_null());
        std::ptr::write_bytes(p, fill, size);
        p
    }

    /// Blocks of the system's allocator made before the process is resident, live when it becomes so
    /// (as an argument of more than 32 KB is): the process maps its requests over 32 KB from then on,
    /// each block of the system's moves to a mapping when it is grown, and each is freed by its owner;
    /// one grown past what the system gives stays as it was.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn blocks_from_before_the_mode_keep_their_owner() {
        if !alone("blocks_from_before_the_mode_keep_their_owner") {
            return;
        }
        const SMALL_SYSTEM: usize = 64 << 10;
        const LARGE_SYSTEM: usize = 3 << 20;
        unsafe {
            let start = held();
            assert_eq!(start.mapped, 0, "mapped before the process is resident");
            let a = fill(TeqAlloc.alloc(layout(SMALL_SYSTEM, 8)), SMALL_SYSTEM, 1);
            let b = fill(TeqAlloc.alloc(layout(LARGE_SYSTEM, 8)), LARGE_SYSTEM, 2);
            assert_eq!(held().mapped, 0);
            let _serial = serial();
            // Resident with both live: a new request over 32 KB is a mapping.
            let c = fill(TeqAlloc.alloc(layout(LARGE_SYSTEM, 8)), LARGE_SYSTEM, 3);
            let made = held().mappings_made;
            assert!(made > start.mappings_made && held().mapped == LARGE_SYSTEM, "the resident process mapped nothing: {:?}", held());
            let d = fill(TeqAlloc.alloc(layout(SMALL_SYSTEM, 8)), SMALL_SYSTEM, 4);
            assert_eq!(held().mapped, LARGE_SYSTEM + SMALL_SYSTEM);
            TeqAlloc.dealloc(d, layout(SMALL_SYSTEM, 8));
            // Grown past what the system gives: null, the block and the account as they were.
            let large = held().large;
            assert!(TeqAlloc.realloc(a, layout(SMALL_SYSTEM, 8), isize::MAX as usize / 2).is_null());
            assert!(filled(a as usize, SMALL_SYSTEM, 1));
            assert_eq!((held().large, held().mapped), (large, LARGE_SYSTEM));
            // The system's large block grown: it moves to a mapping, its contents with it.
            let b = TeqAlloc.realloc(b, layout(LARGE_SYSTEM, 8), 2 * LARGE_SYSTEM);
            assert!(!b.is_null() && filled(b as usize, LARGE_SYSTEM, 2));
            assert_eq!(held().mapped, 3 * LARGE_SYSTEM);
            // The system's small block grown, and shrunk back: a mapping from the growth on.
            let a = TeqAlloc.realloc(a, layout(SMALL_SYSTEM, 8), LARGE_SYSTEM);
            assert!(!a.is_null() && filled(a as usize, SMALL_SYSTEM, 1));
            assert_eq!(held().mapped, 4 * LARGE_SYSTEM);
            let a = TeqAlloc.realloc(a, layout(LARGE_SYSTEM, 8), SMALL_SYSTEM);
            assert!(!a.is_null() && filled(a as usize, SMALL_SYSTEM, 1));
            assert_eq!(held().mapped, 3 * LARGE_SYSTEM + SMALL_SYSTEM);
            assert!(filled(c as usize, LARGE_SYSTEM, 3));
            TeqAlloc.dealloc(a, layout(SMALL_SYSTEM, 8));
            TeqAlloc.dealloc(b, layout(2 * LARGE_SYSTEM, 8));
            TeqAlloc.dealloc(c, layout(LARGE_SYSTEM, 8));
            assert_eq!(held().mapped, 0);
            assert_eq!(held().large, start.large);
        }
    }

    /// The requests over 32 KB through the global allocator in a resident process, each a mapping: across
    /// the small blocks' bound both ways and grown and shrunk past a mapping's room, aligned past 16
    /// bytes (the system's), freed by another thread, taken again for zeros, and grown past what the
    /// system gives, which leaves the block and the account as they were.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_global_allocator_keeps_each_blocks_owner() {
        if !alone("the_global_allocator_keeps_each_blocks_owner") {
            return;
        }
        let _serial = serial();
        const MAPPED: usize = 2 << 20;
        const MIDDLE: usize = 64 << 10;
        const SMALL: usize = 16 << 10;
        // More than any system gives: the mapping's room overflows, the system's block is refused.
        const REFUSED: usize = isize::MAX as usize / 2;
        unsafe {
            let start = held();
            // A small block grown past 32 KB, then past a mapping's room, and back down each bound.
            let p = fill(TeqAlloc.alloc(layout(SMALL, 8)), SMALL, 4);
            let p = TeqAlloc.realloc(p, layout(SMALL, 8), MIDDLE);
            assert!(!p.is_null() && filled(p as usize, SMALL, 4));
            assert_eq!((held().large, held().mapped), (start.large + MIDDLE, start.mapped + MIDDLE));
            let p = fill(TeqAlloc.realloc(p, layout(MIDDLE, 8), MAPPED), MAPPED, 5);
            assert_eq!(held().mapped, start.mapped + MAPPED);
            let p = TeqAlloc.realloc(p, layout(MAPPED, 8), MIDDLE);
            assert!(!p.is_null() && filled(p as usize, MIDDLE, 5));
            assert_eq!(held().mapped, start.mapped + MIDDLE);
            let p = TeqAlloc.realloc(p, layout(MIDDLE, 8), SMALL);
            assert!(!p.is_null() && filled(p as usize, SMALL, 5));
            TeqAlloc.dealloc(p, layout(SMALL, 8));
            assert_eq!((held().large, held().mapped), (start.large, start.mapped));

            // Aligned past 16 bytes: the system's, whatever the size, grown and shrunk aligned.
            for align in [64, 4096] {
                for size in [128, MIDDLE, MAPPED] {
                    let p = fill(TeqAlloc.alloc(layout(size, align)), size, 6);
                    assert_eq!(p as usize % align, 0);
                    let q = TeqAlloc.realloc(p, layout(size, align), 2 * MAPPED);
                    assert!(!q.is_null() && q as usize % align == 0 && filled(q as usize, size, 6));
                    let r = TeqAlloc.realloc(q, layout(2 * MAPPED, align), size);
                    assert!(!r.is_null() && r as usize % align == 0 && filled(r as usize, size, 6));
                    assert_eq!(held().mapped, start.mapped);
                    // Grown past what the system gives: null, the block as it was.
                    assert!(TeqAlloc.realloc(r, layout(size, align), REFUSED).is_null());
                    assert!(filled(r as usize, size, 6));
                    TeqAlloc.dealloc(r, layout(size, align));
                }
            }
            assert_eq!(held().large, start.large);

            // Freed on another thread, each kept for the next request of its size, which takes it for
            // zeros.
            let m = fill(TeqAlloc.alloc(layout(MAPPED, 8)), MAPPED, 7) as usize;
            let s = fill(TeqAlloc.alloc(layout(MIDDLE, 8)), MIDDLE, 7) as usize;
            spawn(move || {
                TeqAlloc.dealloc(m as *mut u8, layout(MAPPED, 8));
                TeqAlloc.dealloc(s as *mut u8, layout(MIDDLE, 8));
            })
            .join()
            .unwrap();
            assert_eq!((held().large, held().mapped), (start.large, start.mapped));
            let made = held().mappings_made;
            let z = TeqAlloc.alloc_zeroed(layout(MAPPED, 8));
            assert!(!z.is_null() && filled(z as usize, MAPPED, 0));
            assert_eq!(held().mappings_made, made, "the kept mapping was not taken again");

            // Grown past what the system gives: null, the block and the account as they were.
            let before = held();
            assert!(TeqAlloc.realloc(z, layout(MAPPED, 8), REFUSED).is_null());
            assert!(filled(z as usize, MAPPED, 0));
            let s = fill(TeqAlloc.alloc(layout(MIDDLE, 8)), MIDDLE, 8);
            assert!(TeqAlloc.realloc(s, layout(MIDDLE, 8), REFUSED).is_null());
            assert!(filled(s as usize, MIDDLE, 8));
            assert_eq!((held().large, held().mapped), (before.large + MIDDLE, before.mapped + MIDDLE));
            TeqAlloc.dealloc(s, layout(MIDDLE, 8));
            TeqAlloc.dealloc(z, layout(MAPPED, 8));
            assert_eq!((held().large, held().mapped), (start.large, start.mapped));
        }
    }

    /// The mappings kept at an idle point are unmapped, their address space and their pages: a request
    /// of their size after it maps anew, and grows.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn kept_mappings_are_unmapped_at_idle() {
        if !alone("kept_mappings_are_unmapped_at_idle") {
            return;
        }
        let _serial = serial();
        const SIZE: usize = 8 << 20;
        unsafe {
            let a = fill(TeqAlloc.alloc(layout(SIZE, 8)), SIZE, 9);
            TeqAlloc.dealloc(a, layout(SIZE, 8));
            let kept = held();
            assert!(kept.kept_mappings >= 1);
            if cfg!(target_os = "linux") {
                assert!(resident_pages().kept >= SIZE, "{} resident in the mappings kept", resident_pages().kept);
            }
            let footprint = os::footprint();
            idle();
            let after = held();
            assert_eq!((after.kept_mappings, after.mapped_kept), (0, 0));
            assert_eq!(after.idle_unmapped, kept.idle_unmapped + kept.kept_mappings);
            if cfg!(target_os = "linux") {
                assert_eq!(resident_pages().kept, 0);
            }
            if cfg!(target_os = "macos") {
                let now = os::footprint();
                assert!(footprint.saturating_sub(now) >= SIZE / 2, "the footprint went from {footprint} to {now} for {SIZE} bytes kept");
            }
            let made = held().mappings_made;
            let b = fill(TeqAlloc.alloc(layout(SIZE, 8)), SIZE, 3);
            assert_eq!(held().mappings_made, made + 1);
            let grown = TeqAlloc.realloc(b, layout(SIZE, 8), 2 * SIZE);
            assert!(!grown.is_null() && filled(grown as usize, SIZE, 3));
            TeqAlloc.dealloc(grown, layout(2 * SIZE, 8));
            idle();
            assert_eq!(held().kept_mappings, 0);
        }
    }

    /// A mapping freed and taken again within a build for a smaller block of its doubling holds that block
    /// alone: its pages and its length's address space, what the first block wrote past it unmapped at the
    /// reuse with the rest, as the system's allocator maps a chunk of the request's length (on Darwin, out
    /// of the footprint; a reuse that took the whole mapping again counted it until the mapping went at the
    /// idle point).
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_mapping_taken_again_for_less_holds_its_new_block_alone() {
        if !alone("a_mapping_taken_again_for_less_holds_its_new_block_alone") {
            return;
        }
        let _serial = serial();
        // Both of one class, 3.5 MB long.
        const FIRST: usize = (7 << 19) - (4 << 10);
        const THIRD: usize = (3 << 20) + (256 << 10);
        unsafe {
            let footprint = os::footprint();
            let a = fill(TeqAlloc.alloc(layout(FIRST, 8)), FIRST, 1);
            TeqAlloc.dealloc(a, layout(FIRST, 8));
            let b = fill(TeqAlloc.alloc(layout(THIRD, 8)), THIRD, 2);
            let len = *length(b.sub(MAPPED_HEADER));
            TeqAlloc.dealloc(b, layout(THIRD, 8));
            assert_eq!(b, a, "the mapping freed was not taken again");
            assert_eq!(len, (MAPPED_HEADER + THIRD).next_multiple_of(os::page()), "the mapping taken again kept the address space past its block");
            if cfg!(target_os = "linux") {
                let (resident, n) = (resident_pages().kept, held().kept_mappings);
                assert!(resident <= THIRD + n * 2 * os::page(), "{resident} resident in {n} mappings kept");
            }
            if cfg!(target_os = "macos") {
                let after = os::footprint();
                assert!(after <= footprint + THIRD + (64 << 10), "the footprint went from {footprint} to {after} over a mapping taken again for {THIRD} bytes");
            }
            idle();
        }
    }

    /// A mapping shrunk in place beside the page count (`resident_pages`), which reads the live mappings'
    /// lengths under the centre's lock: the shrink takes the mapping off the live list while its length
    /// changes. In a process of its own, so that a race a sanitizer reports (ThreadSanitizer's exit) fails it.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_mapping_shrinks_beside_the_page_count() {
        if !alone("a_mapping_shrinks_beside_the_page_count") {
            return;
        }
        let _serial = serial();
        const ROUNDS: usize = 2000;
        let done = std::sync::Arc::new(AtomicBool::new(false));
        let shrinking = {
            let done = done.clone();
            spawn(move || {
                for round in 0..ROUNDS {
                    // SAFETY: a block of the global allocator's, shrunk and freed by its layout.
                    unsafe {
                        let p = fill(TeqAlloc.alloc(layout(2 << 20, 8)), 2 << 20, (round % 251) as u8);
                        let q = TeqAlloc.realloc(p, layout(2 << 20, 8), 64 << 10);
                        assert!(!q.is_null() && filled(q as usize, 64 << 10, (round % 251) as u8));
                        TeqAlloc.dealloc(q, layout(64 << 10, 8));
                    }
                }
                done.store(true, Ordering::Relaxed);
            })
        };
        let mut counts = 0;
        while !done.load(Ordering::Relaxed) {
            let _ = resident_pages();
            counts += 1;
        }
        shrinking.join().unwrap();
        assert!(counts > 0);
        idle();
    }

    /// A kept mapping whose pages the system will not take back at `teq fix`'s age point (a page of it
    /// locked, which `MADV_DONTNEED` refuses) is unmapped there instead, and counted, rather than kept with its
    /// pages through the next typing.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_mapping_whose_advice_is_refused_at_the_age_point_is_unmapped() {
        if !alone("a_mapping_whose_advice_is_refused_at_the_age_point_is_unmapped") {
            return;
        }
        extern "C" {
            fn mlock(at: *const std::ffi::c_void, len: usize) -> i32;
        }
        let _serial = serial();
        const SIZE: usize = 8 << 20;
        unsafe {
            let p = fill(TeqAlloc.alloc(layout(SIZE, 8)), SIZE, 1);
            // The block's first page after the header's, which the age point advises.
            let page = os::page();
            let locked = (p as usize + page) & !(page - 1);
            if mlock(locked as *const std::ffi::c_void, page) != 0 {
                // A system that locks nothing for this process: nothing to see.
                TeqAlloc.dealloc(p, layout(SIZE, 8));
                return;
            }
            TeqAlloc.dealloc(p, layout(SIZE, 8));
            let before = held();
            age();
            let after = held();
            assert_eq!(after.advice_refused, before.advice_refused + 1, "the refusal was not counted");
            assert_eq!(after.kept_mappings, before.kept_mappings - 1, "the refused mapping stayed kept");
            assert!(after.mapped_kept < before.mapped_kept);
            let (resident, n) = (resident_pages().kept, after.kept_mappings);
            assert!(resident <= n * page, "{resident} resident in {n} mappings kept through a typing");
            age();
        }
    }

    /// A block grown by the system past its mapping (`resize`) passes by the kept mapping a request of its new
    /// size would take: that mapping gives back its pages and stays kept, cold, for the next request of its
    /// size, which takes it (the language server's texts, built by growth, one kept per version otherwise).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_growth_gives_back_the_pages_of_the_kept_mapping_it_passes() {
        if !alone("a_growth_gives_back_the_pages_of_the_kept_mapping_it_passes") {
            return;
        }
        let _serial = serial();
        const SIZE: usize = (1 << 20) - (4 << 10);
        const FIRST: usize = 256 << 10;
        unsafe {
            let a = fill(TeqAlloc.alloc(layout(SIZE, 8)), SIZE, 1);
            TeqAlloc.dealloc(a, layout(SIZE, 8));
            assert!(resident_pages().kept >= SIZE / 2, "the freed mapping kept no pages to give back");
            let b = fill(TeqAlloc.alloc(layout(FIRST, 8)), FIRST, 2);
            let resizes = held().resizes;
            let grown = TeqAlloc.realloc(b, layout(FIRST, 8), SIZE);
            assert!(!grown.is_null() && grown != a && filled(grown as usize, FIRST, 2));
            assert_eq!(held().resizes, resizes + 1, "the growth was not the system's");
            let (resident, n) = (resident_pages().kept, held().kept_mappings);
            assert_eq!(n, 1, "the mapping passed by is not kept");
            assert!(resident <= n * os::page(), "{resident} resident in the kept mapping a growth passed by");
            let made = held().mappings_made;
            let c = TeqAlloc.alloc(layout(SIZE, 8));
            assert_eq!((c, held().mappings_made), (a, made), "the cold mapping was not taken again");
            assert!(filled(fill(c, SIZE, 3) as usize, SIZE, 3));
            TeqAlloc.dealloc(c, layout(SIZE, 8));
            TeqAlloc.dealloc(grown, layout(SIZE, 8));
            idle();
        }
    }

    /// A kept mapping whose pages the system will not take back (a page of it locked) where a block grown by the
    /// system passes it by (`cool_passed`) is unmapped and counted, as at the age point, rather than kept cold
    /// with its pages: every site that gives a kept mapping's pages back ends in `unmap_refused` on a refusal.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_mapping_whose_advice_is_refused_where_a_growth_passes_it_is_unmapped() {
        if !alone("a_mapping_whose_advice_is_refused_where_a_growth_passes_it_is_unmapped") {
            return;
        }
        extern "C" {
            fn mlock(at: *const std::ffi::c_void, len: usize) -> i32;
        }
        let _serial = serial();
        const SIZE: usize = (1 << 20) - (4 << 10);
        const FIRST: usize = 256 << 10;
        unsafe {
            let a = fill(TeqAlloc.alloc(layout(SIZE, 8)), SIZE, 1);
            let page = os::page();
            let locked = (a as usize + page) & !(page - 1);
            if mlock(locked as *const std::ffi::c_void, page) != 0 {
                // A system that locks nothing for this process: nothing to see.
                TeqAlloc.dealloc(a, layout(SIZE, 8));
                return;
            }
            TeqAlloc.dealloc(a, layout(SIZE, 8));
            let before = held();
            let b = fill(TeqAlloc.alloc(layout(FIRST, 8)), FIRST, 2);
            let grown = TeqAlloc.realloc(b, layout(FIRST, 8), SIZE);
            assert!(!grown.is_null() && filled(grown as usize, FIRST, 2));
            let after = held();
            assert_eq!(after.resizes, before.resizes + 1, "the growth was not the system's");
            assert_eq!(after.advice_refused, before.advice_refused + 1, "the refusal was not counted");
            assert_eq!(after.kept_mappings, before.kept_mappings - 1, "the refused mapping stayed kept");
            let resident = resident_pages().kept;
            assert!(resident <= after.kept_mappings * page, "{resident} resident in {} mappings kept", after.kept_mappings);
            TeqAlloc.dealloc(grown, layout(SIZE, 8));
            idle();
        }
    }

    /// On a system that cannot resize a mapping (macOS), a growing block that takes a kept mapping keeps the
    /// room it asked for (`MAPPED_ROOM` times its request), as one made anew does, so that its next doubling
    /// grows in place: the kept mapping is shed to the room, not to the request.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_growing_block_keeps_its_room_in_a_kept_mapping() {
        if !alone("a_growing_block_keeps_its_room_in_a_kept_mapping") {
            return;
        }
        let _serial = serial();
        const FIRST: usize = (1 << 20) + (64 << 10);
        const GROWN: usize = 3 << 19;
        unsafe {
            let a = fill(TeqAlloc.alloc(layout(8 << 20, 8)), 8 << 20, 1);
            TeqAlloc.dealloc(a, layout(8 << 20, 8));
            let b = fill(TeqAlloc.alloc(layout(FIRST, 8)), FIRST, 2);
            let made = held().mappings_made;
            let grown = TeqAlloc.realloc(b, layout(FIRST, 8), GROWN);
            assert!(!grown.is_null() && filled(grown as usize, FIRST, 2));
            assert_eq!(grown, a, "the growth did not take the kept mapping");
            assert!(*length(grown.sub(MAPPED_HEADER)) >= MAPPED_ROOM * GROWN, "the kept mapping lost the room asked for");
            let again = TeqAlloc.realloc(grown, layout(GROWN, 8), 2 << 20);
            assert!(again == grown && filled(again as usize, FIRST, 2), "the next doubling moved");
            assert_eq!(held().mappings_made, made);
            TeqAlloc.dealloc(again, layout(2 << 20, 8));
            idle();
        }
    }

    /// What a build gives back to be dropped on a thread of its own (`crew::give_back`, the merge's other workers'
    /// variable tables among it) is dropped before the idle point unmaps the kept mappings (`crew::given_back`, as
    /// `watch::next_command` and `teq fix` call it): a late drop's mappings go with the rest, rather than kept with
    /// their pages until the next idle point.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn what_a_build_gives_back_is_dropped_before_the_idle_point() {
        if !alone("what_a_build_gives_back_is_dropped_before_the_idle_point") {
            return;
        }
        let _serial = serial();
        struct Late(#[allow(dead_code)] Vec<u8>);
        impl Drop for Late {
            fn drop(&mut self) {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        crate::crew::keep_given_back();
        let (made, large) = (held().mappings_made, held().large);
        let late = Late(vec![7u8; 4 << 20]);
        assert_eq!(held().mappings_made, made + 1, "the block given back is no mapping");
        crate::crew::give_back(late);
        crate::crew::given_back();
        assert_eq!(held().large, large, "the block given back was not freed before the idle point");
        idle();
        let after = held();
        assert_eq!((after.kept_mappings, after.mapped_kept), (0, 0), "a late drop's mapping stayed kept past the idle point");
    }

    /// A type store whose overlays' drop is still running when the store goes hands that drop to the idle point
    /// (`TypeStore::give_reclaiming_back`): a watch session's retry after an upstream publication during its build
    /// drops the attempt's typer and makes another store, as a merge or a build that failed drop theirs, so no
    /// later fork of the same store waits for it; the idle point does (`crew::given_back`), whatever store held
    /// the overlays. And a store kept across the idle point hands its own.
    #[test]
    fn a_stores_overlays_drop_is_waited_for_at_the_idle_point_whatever_store_held_them() {
        if !alone("a_stores_overlays_drop_is_waited_for_at_the_idle_point_whatever_store_held_them") {
            return;
        }
        use crate::types::{TypeStore, RECLAIMED, RECLAIM_DELAY_MS};
        let _serial = serial();
        crate::crew::keep_given_back();
        RECLAIM_DELAY_MS.store(300, Ordering::Relaxed);
        let forked = || {
            let s = TypeStore::new();
            s.set_exclusive(false);
            assert!(s.fork_overlays(2), "no overlays to reclaim");
            s.leave_overlay();
            s.join_overlays();
            s.set_exclusive(true);
            s.reclaim_overlays();
            s
        };
        // The retry's: the attempt's store goes with its typer, its overlays' drop still running.
        let ended = RECLAIMED.load(Ordering::Relaxed);
        drop(forked());
        crate::crew::given_back();
        assert_eq!(RECLAIMED.load(Ordering::Relaxed), ended + 1, "the idle point came before the dropped store's overlays were freed");
        // The kept typer's: its store stays and hands the drop on at the idle point.
        let kept = forked();
        kept.give_reclaiming_back();
        crate::crew::given_back();
        assert_eq!(RECLAIMED.load(Ordering::Relaxed), ended + 2, "the idle point came before the kept store's overlays were freed");
        RECLAIM_DELAY_MS.store(0, Ordering::Relaxed);
        drop(kept);
        idle();
    }

    /// At the system's bound on the count of mappings (`vm.max_map_count`), where an unmapping inside a larger
    /// mapping would split it and the system refuses (the push pass of 9afa287f3, its probe): what the allocator
    /// unmaps stays its own. A kept mapping between two live ones of the band's reserve, which the idle point
    /// cannot unmap, stays kept, cold, its pages but the header's given back; a shrink there keeps its length and
    /// gives back the pages past the block; a range no list names is held for a later idle point; and once the
    /// bound is past, the next idle point unmaps them. The bound is reached with mappings of the test's own (one
    /// without access, its pages split until the system refuses another), in a process of its own; nothing is
    /// allocated while the process is at it.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_refused_unmapping_leaves_the_range_the_allocators() {
        if !alone("a_refused_unmapping_leaves_the_range_the_allocators") {
            return;
        }
        use std::ffi::c_void;
        extern "C" {
            fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
            fn munmap(addr: *mut c_void, len: usize) -> i32;
            fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
            fn mincore(addr: *mut u8, len: usize, vec: *mut u8) -> std::os::raw::c_int;
        }
        const PROT_NONE: i32 = 0; // libc's on Linux
        const PROT_READ: i32 = 1; // libc's on Linux
        const PROT_READ_WRITE: i32 = 3; // libc's on Linux
        const MAP_PRIVATE_ANONYMOUS: i32 = 0x22; // libc's MAP_PRIVATE | MAP_ANONYMOUS on Linux
        const MAP_NORESERVE: i32 = 0x4000; // libc's on Linux
        const BLOCK: usize = 64 << 10;
        const SHRUNK: usize = 40 << 10;
        let _serial = serial();
        let bound: usize = std::fs::read_to_string("/proc/sys/vm/max_map_count").unwrap().trim().parse().unwrap();
        unsafe {
            let page = os::page();
            let resident = |at: *mut u8, len: usize, pages: &mut [u8; 64]| -> Option<usize> {
                (mincore(at, len, pages.as_mut_ptr()) == 0).then(|| pages[..len / page].iter().filter(|&&v| v & 1 != 0).count())
            };
            // Three blocks of the band carved one after the other, in the reserve's one mapping.
            let a = fill(TeqAlloc.alloc(layout(BLOCK, 8)), BLOCK, 1);
            let b = fill(TeqAlloc.alloc(layout(BLOCK, 8)), BLOCK, 2);
            let c = fill(TeqAlloc.alloc(layout(BLOCK, 8)), BLOCK, 3);
            let len = *length(b.sub(MAPPED_HEADER));
            assert!(b as usize == a as usize + len && c as usize == b as usize + len, "the band's blocks are not carved one after the other");
            let kept_at = b.sub(MAPPED_HEADER);
            TeqAlloc.dealloc(b, layout(BLOCK, 8));
            // A range of three pages, its middle one for `unmap_range`.
            let scratch = mmap(null_mut(), 3 * page, PROT_READ_WRITE, MAP_PRIVATE_ANONYMOUS, -1, 0) as *mut u8;
            assert!(scratch as isize != -1);
            std::ptr::write_bytes(scratch, 5, 3 * page);
            // The filler, sized by the bound: one mapping without access, a page in two made readable until the
            // system refuses the next split, its last page and one mapping more for the last mappings it allows.
            let span = (bound + bound / 16 + 64) * page;
            let filler = mmap(null_mut(), span, PROT_NONE, MAP_PRIVATE_ANONYMOUS | MAP_NORESERVE, -1, 0) as *mut u8;
            assert!(filler as isize != -1);
            let mut k = 0;
            while (2 * k + 2) * page <= span && mprotect(filler.add((2 * k + 1) * page) as *mut c_void, page, PROT_READ) == 0 {
                k += 1;
            }
            let _edge = mprotect(filler.add(span - page) as *mut c_void, page, PROT_READ);
            let extra = mmap(null_mut(), page, PROT_READ, MAP_PRIVATE_ANONYMOUS, -1, 0);
            // At the bound an unmapping inside a mapping is refused.
            let at_bound = munmap(scratch.add(page) as *mut c_void, page) != 0;
            let mut pages = [0u8; 64];
            let (before, mut after, mut kept_resident, mut c_len, mut c_after, mut c_resident, mut c_moved, mut held_range) = (held(), held(), None, 0, 0, None, false, true);
            if at_bound {
                idle();
                after = held();
                kept_resident = resident(kept_at, len, &mut pages);
                c_len = *length(c.sub(MAPPED_HEADER));
                let shrunk = TeqAlloc.realloc(c, layout(BLOCK, 8), SHRUNK);
                c_moved = shrunk != c;
                c_after = *length(c.sub(MAPPED_HEADER));
                c_resident = resident(c.sub(MAPPED_HEADER), c_len, &mut pages);
                held_range = unmap_range(scratch.add(page), page);
            }
            // The bound past.
            assert_eq!(munmap(filler as *mut c_void, span), 0);
            if extra as isize != -1 {
                munmap(extra, page);
            }
            assert!(at_bound, "the filler did not bring the process to its bound on mappings ({bound}, {k} splits)");
            assert_eq!(after.kept_mappings, before.kept_mappings, "the kept mapping whose unmapping the system refused left the lists");
            assert_eq!(after.idle_unmapped, before.idle_unmapped, "a refused unmapping was counted as one");
            assert!(after.unmaps_refused > before.unmaps_refused, "the refusal was not counted");
            assert!(matches!(kept_resident, Some(n) if n <= 1), "the kept mapping the idle point could not unmap: {kept_resident:?} pages resident (none: unmapped)");
            assert!(!c_moved && c_after == c_len, "the shrink at the bound moved or changed the mapping's length ({c_len} to {c_after})");
            let block_pages = (MAPPED_HEADER + SHRUNK).div_ceil(page);
            assert!(matches!(c_resident, Some(n) if n <= block_pages), "the shrunk block holds {c_resident:?} pages, its own {block_pages}");
            assert!(filled(c as usize, SHRUNK, 3), "the shrink at the bound lost the block's contents");
            assert!(!held_range && held().orphaned == 1, "the range whose unmapping the system refused is not held");
            // The next idle point unmaps what the system refused.
            idle();
            let later = held();
            assert_eq!(later.kept_mappings, before.kept_mappings - 1, "the kept mapping is kept still past the bound");
            assert!(resident(kept_at, len, &mut pages).is_none(), "the kept mapping is mapped still past the bound");
            assert_eq!(later.orphaned, 0, "the held range is held still past the bound");
            assert!(resident(scratch.add(page), page, &mut pages).is_none(), "the held range is mapped still past the bound");
            munmap(scratch as *mut c_void, 3 * page);
            TeqAlloc.dealloc(a, layout(BLOCK, 8));
            TeqAlloc.dealloc(c, layout(SHRUNK, 8));
            idle();
        }
    }

    /// A coroutine's stack asked at the bound on the count of mappings while mappings are kept, two or more of them
    /// mappings of the system's of their own: its mapping is allowed and its guard, a split of it, refused; the
    /// kept mappings give way, the count falls by two or more, and the stack is served, as it is once an idle
    /// point unmapped them (the sixth self-review's probe). The bound is reached as above, in a process of its
    /// own.
    #[cfg(all(target_os = "linux", feature = "threads"))]
    #[test]
    fn a_stacks_guard_refused_at_the_count_bound_gives_way() {
        if !alone("a_stacks_guard_refused_at_the_count_bound_gives_way") {
            return;
        }
        use std::ffi::c_void;
        extern "C" {
            fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
            fn munmap(addr: *mut c_void, len: usize) -> i32;
            fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
        }
        const PROT_NONE: i32 = 0; // libc's on Linux
        const PROT_READ: i32 = 1; // libc's on Linux
        const PROT_READ_WRITE: i32 = 3; // libc's on Linux
        const MAP_PRIVATE_ANONYMOUS: i32 = 0x22; // libc's MAP_PRIVATE | MAP_ANONYMOUS on Linux
        const MAP_NORESERVE: i32 = 0x4000; // libc's on Linux
        const MAP_FIXED_NOREPLACE: i32 = 0x100000; // libc's on Linux
        const OWN: usize = 2 << 20;
        const STACK: usize = 1 << 20;
        const GUARD: usize = 64 << 10;
        let _serial = serial();
        let bound: usize = std::fs::read_to_string("/proc/sys/vm/max_map_count").unwrap().trim().parse().unwrap();
        unsafe {
            let page = os::page();
            idle();
            // Four kept mappings, a page without access mapped just below each as it is mapped, where the system
            // would map the next one, so that the system merges none of them with the next.
            let mut kept = [null_mut(); 4];
            let mut walls = [null_mut(); 4];
            for k in 0..kept.len() {
                kept[k] = fill(TeqAlloc.alloc(layout(OWN, 8)), OWN, k as u8 + 1);
                let below = kept[k].sub(MAPPED_HEADER + page) as *mut c_void;
                walls[k] = mmap(below, page, PROT_NONE, MAP_PRIVATE_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            }
            let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
            let own = kept
                .iter()
                .filter(|&&p| {
                    let at = p as usize - MAPPED_HEADER;
                    let line = format!("{at:x}-{:x} ", at + *length(p.sub(MAPPED_HEADER)));
                    maps.lines().any(|l| l.starts_with(&line))
                })
                .count();
            assert!(own >= 2, "of the four kept mappings {own} are mappings of the system's of their own");
            for p in kept {
                TeqAlloc.dealloc(p, layout(OWN, 8));
            }
            let scratch = mmap(null_mut(), 3 * page, PROT_READ_WRITE, MAP_PRIVATE_ANONYMOUS, -1, 0) as *mut u8;
            assert!(scratch as isize != -1);
            std::ptr::write_bytes(scratch, 5, 3 * page);
            // The filler as above, without the one mapping more: the stack's mapping is the last one allowed.
            let span = (bound + bound / 16 + 64) * page;
            let filler = mmap(null_mut(), span, PROT_NONE, MAP_PRIVATE_ANONYMOUS | MAP_NORESERVE, -1, 0) as *mut u8;
            assert!(filler as isize != -1);
            let mut k = 0;
            while (2 * k + 2) * page <= span && mprotect(filler.add((2 * k + 1) * page) as *mut c_void, page, PROT_READ) == 0 {
                k += 1;
            }
            let _edge = mprotect(filler.add(span - page) as *mut c_void, page, PROT_READ);
            let at_bound = munmap(scratch.add(page) as *mut c_void, page) != 0;
            let before = held();
            let stack = if at_bound { stack_pages(STACK + GUARD, GUARD) } else { null_mut() };
            let after = held();
            if !stack.is_null() {
                free_zeroed_pages(stack, STACK + GUARD);
            }
            // The bound past.
            assert_eq!(munmap(filler as *mut c_void, span), 0);
            munmap(scratch as *mut c_void, 3 * page);
            for w in walls {
                if w as isize != -1 {
                    munmap(w, page);
                }
            }
            assert!(at_bound, "the filler did not bring the process to its bound on mappings ({bound}, {k} splits)");
            assert!(!stack.is_null(), "a stack the system serves once the kept mappings are unmapped was refused while they were kept");
            assert_eq!(after.kept_unmapped, before.kept_unmapped + 1, "the kept mappings did not give way to the stack's guard");
            assert_eq!(after.kept_mappings, 0, "a kept mapping stood in the way of the stack's guard");
            idle();
        }
    }

    /// Give-way at the bound on the count of mappings where the reserve's rest lies inside a larger mapping (a
    /// reserve the system merged with the one before it, above it): the system refuses to unmap the rest, which
    /// stays the reserve, its bookkeeping as it was, and serves the band's next request. The bound is reached as
    /// above and passed by one mapping more, so that the request's own mapping is refused.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_reserves_rest_the_system_refuses_to_unmap_stays_the_reserve() {
        if !alone("a_reserves_rest_the_system_refuses_to_unmap_stays_the_reserve") {
            return;
        }
        use std::ffi::c_void;
        extern "C" {
            fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
            fn munmap(addr: *mut c_void, len: usize) -> i32;
            fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
        }
        const PROT_NONE: i32 = 0; // libc's on Linux
        const PROT_READ: i32 = 1; // libc's on Linux
        const PROT_READ_WRITE: i32 = 3; // libc's on Linux
        const MAP_PRIVATE_ANONYMOUS: i32 = 0x22; // libc's MAP_PRIVATE | MAP_ANONYMOUS on Linux
        const MAP_NORESERVE: i32 = 0x4000; // libc's on Linux
        const BLOCK: usize = (1 << 20) - (64 << 10);
        const ASKED: usize = 4 << 20;
        let _serial = serial();
        let bound: usize = std::fs::read_to_string("/proc/sys/vm/max_map_count").unwrap().trim().parse().unwrap();
        unsafe {
            let page = os::page();
            // Whether a range lies strictly inside one mapping of the system's.
            let inside = |at: usize, len: usize| {
                std::fs::read_to_string("/proc/self/maps").unwrap().lines().any(|l| {
                    let range = l.split(' ').next().unwrap().split_once('-').unwrap();
                    let (from, to) = (usize::from_str_radix(range.0, 16).unwrap(), usize::from_str_radix(range.1, 16).unwrap());
                    from < at && at + len < to
                })
            };
            // Blocks of the band carved until the reserve's rest lies inside a larger mapping: past a new reserve
            // the system mapped just below the one before, whose rest went; three reserves' blocks at most.
            let mut blocks = Vec::with_capacity(3 * RESERVE / BLOCK);
            let (mut rest, mut rest_len) = centre(|c| (c.reserve, c.reserve_left));
            while blocks.len() < blocks.capacity() && !(rest_len > 0 && inside(rest as usize, rest_len)) {
                blocks.push(fill(TeqAlloc.alloc(layout(BLOCK, 8)), page, 1));
                (rest, rest_len) = centre(|c| (c.reserve, c.reserve_left));
            }
            assert!(rest_len > 0 && inside(rest as usize, rest_len), "no reserve's rest lay inside a larger mapping past {} blocks", blocks.len());
            let scratch = mmap(null_mut(), 3 * page, PROT_READ_WRITE, MAP_PRIVATE_ANONYMOUS, -1, 0) as *mut u8;
            assert!(scratch as isize != -1);
            std::ptr::write_bytes(scratch, 5, 3 * page);
            // The filler and the one mapping more, as above.
            let span = (bound + bound / 16 + 64) * page;
            let filler = mmap(null_mut(), span, PROT_NONE, MAP_PRIVATE_ANONYMOUS | MAP_NORESERVE, -1, 0) as *mut u8;
            assert!(filler as isize != -1);
            let mut k = 0;
            while (2 * k + 2) * page <= span && mprotect(filler.add((2 * k + 1) * page) as *mut c_void, page, PROT_READ) == 0 {
                k += 1;
            }
            let _edge = mprotect(filler.add(span - page) as *mut c_void, page, PROT_READ);
            let extra = mmap(null_mut(), page, PROT_READ, MAP_PRIVATE_ANONYMOUS, -1, 0);
            let at_bound = munmap(scratch.add(page) as *mut c_void, page) != 0;
            let before = held();
            let request = if at_bound { TeqAlloc.alloc(layout(ASKED, 8)) } else { null_mut() };
            let after = held();
            let reserve = centre(|c| (c.reserve, c.reserve_left));
            // The bound past.
            assert_eq!(munmap(filler as *mut c_void, span), 0);
            if extra as isize != -1 {
                munmap(extra, page);
            }
            munmap(scratch as *mut c_void, 3 * page);
            if !request.is_null() {
                TeqAlloc.dealloc(request, layout(ASKED, 8));
            }
            assert!(at_bound, "the filler did not bring the process to its bound on mappings ({bound}, {k} splits)");
            assert_eq!(reserve, (rest, rest_len), "the reserve's rest the system refused to unmap is not the reserve");
            assert!(after.unmaps_refused > before.unmaps_refused, "the system's refusal to unmap the rest was not counted");
            let b = TeqAlloc.alloc(layout(BLOCK, 8));
            assert_eq!(b.sub(MAPPED_HEADER), rest, "the band's next request was not carved from the reserve's rest");
            for p in blocks.into_iter().chain([b]) {
                TeqAlloc.dealloc(p, layout(BLOCK, 8));
            }
            idle();
        }
    }

    /// A request the system refuses address space for, under a bound on it (`ulimit -v`), while a
    /// mapping is kept: the mappings kept are unmapped and the request is mapped, as it would be
    /// where none was kept.
    #[cfg(target_os = "linux")]
    #[test]
    fn kept_mappings_give_way_to_a_request_the_address_space_refuses() {
        if !alone("kept_mappings_give_way_to_a_request_the_address_space_refuses") {
            return;
        }
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
        }
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux
        let _serial = serial();
        // The kept mapping (20 MB long) is shorter than the request's (40 MB): it maps anew.
        const KEPT: usize = 16 << 20;
        const ASKED: usize = 32 << 20;
        unsafe {
            let a = fill(TeqAlloc.alloc(layout(KEPT, 8)), KEPT, 1);
            TeqAlloc.dealloc(a, layout(KEPT, 8));
            let kept = held();
            assert!(kept.kept_mappings >= 1);
            let status = std::fs::read_to_string("/proc/self/status").unwrap();
            let size_kb: u64 = status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches(" kB").trim().parse().unwrap();
            let mut old = Rlimit { cur: 0, max: 0 };
            assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0);
            // Room beside what is mapped now for three quarters of the request.
            let bound = Rlimit { cur: size_kb * 1024 + (ASKED / 4 * 3) as u64, max: old.max };
            assert_eq!(setrlimit(RLIMIT_AS, &bound), 0);
            let b = TeqAlloc.alloc(layout(ASKED, 8));
            assert_eq!(setrlimit(RLIMIT_AS, &old), 0);
            assert!(!b.is_null(), "the request was refused with {} bytes of mappings kept", kept.mapped_kept);
            assert_eq!(held().kept_unmapped, kept.kept_unmapped + 1);
            assert_eq!(held().kept_mappings, 0);
            fill(b, ASKED, 2);
            TeqAlloc.dealloc(b, layout(ASKED, 8));
        }
    }

    /// The band's mappings are carved from address space mapped ahead (`from_reserve`): under a bound on
    /// the address space that leaves no room for a new mapping, a request of the band is still served,
    /// and a request over it is mapped once the reserve's rest gives way.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_band_is_carved_from_address_space_mapped_ahead() {
        if !alone("the_band_is_carved_from_address_space_mapped_ahead") {
            return;
        }
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
        }
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux
        let _serial = serial();
        const BAND: usize = 96 << 10;
        const ARRAY: usize = 2 << 20;
        unsafe {
            let a = fill(TeqAlloc.alloc(layout(BAND, 8)), BAND, 1);
            let len = (BAND + MAPPED_HEADER).next_multiple_of(os::page());
            let status = std::fs::read_to_string("/proc/self/status").unwrap();
            let size_kb: u64 = status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches(" kB").trim().parse().unwrap();
            let mut old = Rlimit { cur: 0, max: 0 };
            assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0);
            // No room beside what is mapped now.
            assert_eq!(setrlimit(RLIMIT_AS, &Rlimit { cur: size_kb * 1024, max: old.max }), 0);
            let b = TeqAlloc.alloc(layout(BAND, 8));
            let given = held().kept_unmapped;
            let c = TeqAlloc.alloc(layout(ARRAY, 8));
            assert_eq!(setrlimit(RLIMIT_AS, &old), 0);
            assert!(!b.is_null(), "a request of the band was refused with the reserve's rest mapped");
            assert_eq!(b as usize, a as usize + len, "the band's mappings are not carved one after the other");
            assert!(!c.is_null() && held().kept_unmapped == given + 1, "the reserve's rest did not give way to a request over the band");
            assert!(filled(a as usize, BAND, 1));
            fill(b, BAND, 2);
            fill(c, ARRAY, 3);
            TeqAlloc.dealloc(a, layout(BAND, 8));
            TeqAlloc.dealloc(b, layout(BAND, 8));
            TeqAlloc.dealloc(c, layout(ARRAY, 8));
            idle();
            assert_eq!(held().kept_mappings, 0);
        }
    }

    /// A block grown under a bound on the address space that has room for its new size's class and not for
    /// the room a growing block is given: it grows without the room, its contents kept, a mapping's and a
    /// system block's from before the mode alike (the push pass of 96499a77c, its probe: 102 blocks of
    /// 512 KB pinned, the bound 2 MB over what is mapped, one block grown to 900 KB).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_block_grows_without_room_where_the_address_space_has_none() {
        if !alone("a_block_grows_without_room_where_the_address_space_has_none") {
            return;
        }
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
        }
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux
        const PINNED: usize = 102;
        const BLOCK: usize = 512 << 10;
        const GROWN: usize = 900 << 10;
        unsafe {
            // A system block from before the process is resident.
            let early = fill(TeqAlloc.alloc(layout(BLOCK, 8)), BLOCK, 7);
            let _serial = serial();
            let pinned: Vec<*mut u8> = (0..PINNED).map(|i| fill(TeqAlloc.alloc(layout(BLOCK, 8)), BLOCK, i as u8)).collect();
            let status = std::fs::read_to_string("/proc/self/status").unwrap();
            let size_kb: u64 = status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches(" kB").trim().parse().unwrap();
            let mut old = Rlimit { cur: 0, max: 0 };
            assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0);
            assert_eq!(setrlimit(RLIMIT_AS, &Rlimit { cur: size_kb * 1024 + (2 << 20), max: old.max }), 0);
            let grown = TeqAlloc.realloc(pinned[0], layout(BLOCK, 8), GROWN);
            let moved = if grown.is_null() { grown } else { TeqAlloc.realloc(early, layout(BLOCK, 8), GROWN) };
            assert_eq!(setrlimit(RLIMIT_AS, &old), 0);
            assert!(!grown.is_null(), "a mapping's growth was refused for its room");
            assert!(!moved.is_null(), "a system block's growth was refused for its room");
            assert!(filled(grown as usize, BLOCK, 0) && filled(moved as usize, BLOCK, 7));
            TeqAlloc.dealloc(grown, layout(GROWN, 8));
            TeqAlloc.dealloc(moved, layout(GROWN, 8));
            for &p in &pinned[1..] {
                TeqAlloc.dealloc(p, layout(BLOCK, 8));
            }
            idle();
        }
    }

    /// Requests aligned past 16 bytes, the system allocator's, under a bound on the address space that the
    /// reserve's unused rest fills: the reserve gives way and they are served, a request aligned to 32 and
    /// to 4096 bytes and a block of 64 KB aligned to 32 from before the mode grown (the push pass of
    /// 61f4a7410, its probes, where master serves both). The requests are of 48 MB, past glibc's largest
    /// threshold of a request it maps on its own (`DEFAULT_MMAP_THRESHOLD_MAX`, 32 MB on 64-bit) and past any
    /// free space of its main arena's, so that they need address space of their own.
    #[cfg(target_os = "linux")]
    #[test]
    fn requests_aligned_past_the_step_are_served_once_the_reserve_gives_way() {
        // glibc held to its main arena, whose growth the bound limits: an arena of a thread's reserves 64 MB
        // of address space ahead (`HEAP_MAX_SIZE`), which serves such a request whatever the bound.
        if !alone_in("requests_aligned_past_the_step_are_served_once_the_reserve_gives_way", &[("MALLOC_ARENA_MAX", "1")]) {
            return;
        }
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
        }
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux
        const EARLY: usize = 64 << 10;
        const LARGE: usize = 48 << 20;
        unsafe {
            let early = fill(TeqAlloc.alloc(layout(EARLY, 32)), EARLY, 5);
            let _serial = serial();
            let mut old = Rlimit { cur: 0, max: 0 };
            assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0);
            let vm_size = || -> u64 {
                let status = std::fs::read_to_string("/proc/self/status").unwrap();
                status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches(" kB").trim().parse::<u64>().unwrap() * 1024
            };
            // As after an idle point, a request of the band in a reserve of its own, almost all of it unused;
            // then a bound that leaves 2 MB beside what is mapped.
            let bounded = || {
                idle();
                let band = fill(TeqAlloc.alloc(layout(EARLY, 8)), EARLY, 1);
                let given = held().kept_unmapped;
                assert_eq!(setrlimit(RLIMIT_AS, &Rlimit { cur: vm_size() + (2 << 20), max: old.max }), 0);
                (band, given)
            };
            let mut served = Vec::new();
            for align in [32, 4096] {
                let (band, given) = bounded();
                let p = TeqAlloc.alloc(layout(LARGE, align));
                assert_eq!(setrlimit(RLIMIT_AS, &old), 0);
                assert!(!p.is_null() && p as usize % align == 0, "a request aligned to {align} was refused beside the reserve");
                assert_eq!(held().kept_unmapped, given + 1);
                fill(p, LARGE, 2);
                served.push((p, align));
                TeqAlloc.dealloc(band, layout(EARLY, 8));
            }
            let (band, given) = bounded();
            let grown = TeqAlloc.realloc(early, layout(EARLY, 32), LARGE);
            assert_eq!(setrlimit(RLIMIT_AS, &old), 0);
            assert!(!grown.is_null() && grown as usize % 32 == 0, "a block aligned to 32 from before the mode was refused its growth beside the reserve");
            assert_eq!(held().kept_unmapped, given + 1);
            assert!(filled(grown as usize, EARLY, 5));
            TeqAlloc.dealloc(grown, layout(LARGE, 32));
            TeqAlloc.dealloc(band, layout(EARLY, 8));
            for (p, align) in served {
                TeqAlloc.dealloc(p, layout(LARGE, align));
            }
            idle();
        }
    }

    /// A look at the pages (`resident_pages`, `stats`) over more mappings than a list of 32 KB holds (the
    /// push pass of d6adad162: some 1,365) makes no mapping of the allocator's and leaves none kept: its list
    /// is a mapping of the system's of its own.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_look_at_the_pages_of_many_mappings_leaves_none_kept() {
        if !alone("a_look_at_the_pages_of_many_mappings_leaves_none_kept") {
            return;
        }
        let _serial = serial();
        const MAPPINGS: usize = 2000;
        const BLOCK: usize = 40 << 10;
        unsafe {
            let blocks: Vec<*mut u8> = (0..MAPPINGS).map(|i| fill(TeqAlloc.alloc(layout(BLOCK, 8)), BLOCK, i as u8)).collect();
            idle();
            let before = held();
            assert_eq!(before.kept_mappings, 0);
            let pages = resident_pages();
            let after = held();
            assert_eq!(after.mappings_made, before.mappings_made, "the look made a mapping of the allocator's");
            assert_eq!((after.kept_mappings, after.mapped_kept, pages.kept), (0, 0, 0), "the look left a mapping kept");
            assert!(pages.mapped >= MAPPINGS * BLOCK, "{} resident in {MAPPINGS} live mappings of {BLOCK} bytes", pages.mapped);
            for p in blocks {
                TeqAlloc.dealloc(p, layout(BLOCK, 8));
            }
            idle();
            // Two long ranges, of 12 MiB and then 16 MiB in the walk's order: the page count's buffer stays a small
            // block, where grown by doubling it passed 32 KB on the second and was a mapping of the allocator's.
            let large = fill(TeqAlloc.alloc(layout(16 << 20, 8)), 16 << 20, 1);
            let middle = fill(TeqAlloc.alloc(layout(12 << 20, 8)), 12 << 20, 2);
            let before = held();
            let pages = resident_pages();
            let after = held();
            assert_eq!(after.mappings_made, before.mappings_made, "the look over long ranges made a mapping of the allocator's");
            assert_eq!((after.kept_mappings, pages.kept), (0, 0), "the look over long ranges left a mapping kept");
            TeqAlloc.dealloc(middle, layout(12 << 20, 8));
            TeqAlloc.dealloc(large, layout(16 << 20, 8));
            idle();
        }
    }

    /// Reallocs that master serves in place or with the difference of address space alone, served so
    /// under a bound that leaves no room for a second copy of the block (the push pass of 7673c0bff, its
    /// probe and the sweep after it): a system block of before the mode shrunk stays the system's, and
    /// grown where a mapping is refused grows by the system's realloc; a mapping shrunk far below its
    /// length where a smaller one is refused stays in its own; a mapping grown where a new one is refused
    /// is given the new length by the system (Linux's `mremap`). The system blocks are of 48 MB, past
    /// glibc's largest threshold of a request it maps on its own (`DEFAULT_MMAP_THRESHOLD_MAX`), so that
    /// glibc resizes them in their own mappings; glibc on its main arena alone (`alone_in`).
    #[cfg(target_os = "linux")]
    #[test]
    fn reallocs_that_need_no_second_copy_are_served_under_a_bound() {
        if !alone_in("reallocs_that_need_no_second_copy_are_served_under_a_bound", &[("MALLOC_ARENA_MAX", "1")]) {
            return;
        }
        #[repr(C)]
        struct Rlimit {
            cur: u64,
            max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
        }
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux
        const SYSTEM: usize = 48 << 20;
        const MAPPED: usize = 8 << 20;
        unsafe {
            let early = fill(TeqAlloc.alloc(layout(SYSTEM, 8)), SYSTEM, 3);
            let _serial = serial();
            let mut old = Rlimit { cur: 0, max: 0 };
            assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0);
            let vm_size = || -> u64 {
                let status = std::fs::read_to_string("/proc/self/status").unwrap();
                status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches(" kB").trim().parse::<u64>().unwrap() * 1024
            };
            let bound = |over: u64| assert_eq!(setrlimit(RLIMIT_AS, &Rlimit { cur: vm_size() + over, max: old.max }), 0);
            let unbound = || assert_eq!(setrlimit(RLIMIT_AS, &old), 0);
            let mapped = held().mapped;
            // A system block of before the mode, shrunk: the system's, in place.
            bound(1 << 20);
            let shrunk = TeqAlloc.realloc(early, layout(SYSTEM, 8), SYSTEM / 2);
            unbound();
            assert!(!shrunk.is_null() && filled(shrunk as usize, SYSTEM / 2, 3), "a system block of before the mode was refused its shrink");
            assert_eq!(held().mapped, mapped, "a shrunk system block moved to a mapping");
            // Grown where a mapping is refused: the system's realloc grows it.
            bound(1 << 20);
            let grown = TeqAlloc.realloc(shrunk, layout(SYSTEM / 2, 8), SYSTEM / 2 + (512 << 10));
            unbound();
            assert!(!grown.is_null() && filled(grown as usize, SYSTEM / 2, 3), "a system block's growth was refused where its mapping was");
            assert_eq!(held().mapped, mapped);
            // A mapping shrunk far below its length, no room for another: in its own.
            unmap_kept();
            let m = fill(TeqAlloc.alloc(layout(MAPPED, 8)), MAPPED, 4);
            bound(0);
            let small = TeqAlloc.realloc(m, layout(MAPPED, 8), 256 << 10);
            unbound();
            assert!(small == m && filled(small as usize, 256 << 10, 4), "a mapping was refused its shrink");
            // A mapping grown where a new one is refused: the system gives it the new length.
            let g = TeqAlloc.realloc(small, layout(256 << 10, 8), MAPPED);
            assert!(!g.is_null());
            fill(g, MAPPED, 5);
            unmap_kept();
            bound(4 << 20);
            let longer = TeqAlloc.realloc(g, layout(MAPPED, 8), MAPPED + (2 << 20));
            unbound();
            assert!(!longer.is_null() && filled(longer as usize, MAPPED, 5), "a mapping was refused its growth where the system can give it the length");
            TeqAlloc.dealloc(longer, layout(MAPPED + (2 << 20), 8));
            TeqAlloc.dealloc(grown, layout(SYSTEM / 2 + (512 << 10), 8));
            idle();
        }
    }

    /// The bound of a test's own process on its address space (`RLIMIT_AS`), and its address space now.
    #[cfg(target_os = "linux")]
    mod bound {
        #[repr(C)]
        pub struct Rlimit {
            pub cur: u64,
            pub max: u64,
        }
        extern "C" {
            fn getrlimit(resource: i32, limit: *mut Rlimit) -> i32;
            fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
        }
        const RLIMIT_AS: i32 = 9; // libc's resource number on Linux

        pub fn vm_size() -> u64 {
            let status = std::fs::read_to_string("/proc/self/status").unwrap();
            status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches(" kB").trim().parse::<u64>().unwrap() * 1024
        }

        /// The bound set `over` bytes past the address space now; the bound before.
        pub fn set(over: u64) -> Rlimit {
            let mut old = Rlimit { cur: 0, max: 0 };
            // SAFETY: the kernel fills and reads a struct of its layout.
            unsafe {
                assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0);
                assert_eq!(setrlimit(RLIMIT_AS, &Rlimit { cur: vm_size() + over, max: old.max }), 0);
            }
            old
        }

        pub fn restore(old: &Rlimit) {
            // SAFETY: as above.
            unsafe { assert_eq!(setrlimit(RLIMIT_AS, old), 0) };
        }
    }

    /// The push pass of 6280b3c82, its first probe: a mapping of 10 MiB shrunk in place to 256 KiB with no
    /// address space to spare (the bound at the address space before the shrink), then grown to 288 KiB, given
    /// 288 KiB again, shrunk to 256 KiB and given 256 KiB again, all at its address: the shrink gives back the
    /// address space past the block's pages, as the system's allocator's does for a chunk it mapped, and the
    /// growth takes what it needs of it again. Contents kept.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_mapping_shrunk_at_the_bound_grows_again_in_place() {
        if !alone_in("a_mapping_shrunk_at_the_bound_grows_again_in_place", &[("MALLOC_ARENA_MAX", "1")]) {
            return;
        }
        unsafe {
            let early = fill(TeqAlloc.alloc(layout(64 << 10, 8)), 64 << 10, 9);
            let _serial = serial();
            let m = fill(TeqAlloc.alloc(layout(10 << 20, 8)), 10 << 20, 4);
            let old = bound::set(0);
            let shrunk = TeqAlloc.realloc(m, layout(10 << 20, 8), 256 << 10);
            let grown = if shrunk.is_null() { shrunk } else { TeqAlloc.realloc(shrunk, layout(256 << 10, 8), 288 << 10) };
            let same = if grown.is_null() { grown } else { TeqAlloc.realloc(grown, layout(288 << 10, 8), 288 << 10) };
            let back = if same.is_null() { same } else { TeqAlloc.realloc(same, layout(288 << 10, 8), 256 << 10) };
            let again = if back.is_null() { back } else { TeqAlloc.realloc(back, layout(256 << 10, 8), 256 << 10) };
            bound::restore(&old);
            assert!(!shrunk.is_null() && shrunk == m, "the shrink in place was refused");
            assert!(!grown.is_null() && grown == m, "a growth into the address space the shrink gave back was refused");
            assert!(!same.is_null() && same == m, "a realloc to the same size was refused");
            assert!(!again.is_null() && again == m && filled(again as usize, 256 << 10, 4));
            assert!(filled(early as usize, 64 << 10, 9));
            TeqAlloc.dealloc(again, layout(256 << 10, 8));
            TeqAlloc.dealloc(early, layout(64 << 10, 8));
            idle();
        }
    }

    /// The push pass of 6280b3c82, its second probe: a mapping of 10 MiB shrunk in place to 256 KiB, then with
    /// the bound at the address space after the shrink (none to spare), a realloc to the same 256 KiB and one
    /// within the block's last page: within the mapping's length, so served in place with no address space,
    /// contents kept.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_mapping_shrunk_at_the_bound_keeps_its_size_in_place() {
        if !alone_in("a_mapping_shrunk_at_the_bound_keeps_its_size_in_place", &[("MALLOC_ARENA_MAX", "1")]) {
            return;
        }
        // The block's last page: its length, the header's and the block's in whole pages.
        const WITHIN: usize = (256 << 10) + (3 << 10);
        unsafe {
            let early = fill(TeqAlloc.alloc(layout(64 << 10, 8)), 64 << 10, 9);
            let _serial = serial();
            let m = fill(TeqAlloc.alloc(layout(10 << 20, 8)), 10 << 20, 4);
            let shrunk = TeqAlloc.realloc(m, layout(10 << 20, 8), 256 << 10);
            assert!(!shrunk.is_null() && shrunk == m, "the shrink in place was refused");
            let old = bound::set(0);
            let same = TeqAlloc.realloc(shrunk, layout(256 << 10, 8), 256 << 10);
            let within = if same.is_null() { same } else { TeqAlloc.realloc(same, layout(256 << 10, 8), WITHIN) };
            bound::restore(&old);
            assert!(!same.is_null() && same == m, "a realloc to the same size was refused at the bound");
            assert!(!within.is_null() && within == m && filled(within as usize, 256 << 10, 4), "a growth within the block's last page was refused at the bound");
            assert!(filled(early as usize, 64 << 10, 9));
            TeqAlloc.dealloc(within, layout(WITHIN, 8));
            TeqAlloc.dealloc(early, layout(64 << 10, 8));
            idle();
        }
    }

    /// One request of the differential sequence: a slot, an action and a size.
    #[cfg(target_os = "linux")]
    #[derive(Clone, Copy, Debug)]
    enum Op {
        Alloc(usize, usize, usize),
        Realloc(usize, usize),
        Free(usize),
    }

    /// A seeded sequence of requests over 32 KB (the small blocks are master's own allocator on either
    /// side, the same): allocations aligned to 8 mostly and past 16 now and then, reallocs that shrink, keep
    /// the size, grow a little and grow past any room, and frees, over a handful of slots.
    #[cfg(target_os = "linux")]
    fn differential_ops(seed: u64, n: usize) -> Vec<Op> {
        const SLOTS: usize = 12;
        let mut state = seed | 1;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut sizes = [0usize; SLOTS];
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            let slot = (next() % SLOTS as u64) as usize;
            let pick = next() % 100;
            // Half the band, two fifths of a megabyte to 13 MB, a tenth just over the small blocks' bound.
            let fresh = |r: u64| {
                let tenth = r % 10;
                if tenth < 5 {
                    MAX_SIZE + 1 + (r as usize >> 8) % (1 << 20)
                } else if tenth < 9 {
                    (1 << 20) + (r as usize >> 8) % (12 << 20)
                } else {
                    MAX_SIZE + 1 + (r as usize >> 8) % (64 << 10)
                }
            };
            if sizes[slot] == 0 {
                let r = next();
                let align = if r % 9 == 0 { 64 } else if r % 13 == 0 { 4096 } else { 8 };
                sizes[slot] = fresh(next());
                ops.push(Op::Alloc(slot, sizes[slot], align));
            } else if pick < 15 {
                sizes[slot] = 0;
                ops.push(Op::Free(slot));
            } else {
                let old = sizes[slot];
                let r = next();
                let new = match pick % 4 {
                    0 => (old / 2 + (r as usize % (old / 2))).max(MAX_SIZE + 1),
                    1 => old,
                    2 => old + (r as usize % (32 << 10)),
                    _ => old * 2 + (r as usize % old),
                };
                sizes[slot] = new;
                ops.push(Op::Realloc(slot, new));
            }
        }
        ops
    }

    /// Runs the differential sequence in this process with the allocator `branch` names (the global one,
    /// or the system's alone): the first quarter before the process is resident and unbounded, the rest
    /// bounded at the address space it has then. A step's outcome is `1` served, `0` refused, `-` nothing
    /// asked (a request to a slot an earlier refusal left empty); a served block's contents are checked.
    #[cfg(target_os = "linux")]
    unsafe fn differential_run(branch: bool, seed: u64) -> String {
        const STEPS: usize = 400;
        let ops = differential_ops(seed, STEPS);
        let mut live: Vec<Option<(*mut u8, Layout, u8)>> = vec![None; 12];
        let mut outcomes = String::with_capacity(STEPS);
        let mut old = None;
        for (i, &op) in ops.iter().enumerate() {
            if i == STEPS / 4 {
                if branch {
                    resident();
                }
                old = Some(bound::set(0));
            }
            let mark = (i % 251) as u8;
            let outcome = match op {
                Op::Alloc(slot, size, align) => {
                    let l = layout(size, align);
                    let p = if branch { TeqAlloc.alloc(l) } else { System.alloc(l) };
                    if p.is_null() {
                        '0'
                    } else {
                        std::ptr::write_bytes(p, mark, size);
                        live[slot] = Some((p, l, mark));
                        '1'
                    }
                }
                Op::Realloc(slot, new) => match live[slot] {
                    None => '-',
                    Some((p, l, m)) => {
                        let q = if branch { TeqAlloc.realloc(p, l, new) } else { System.realloc(p, l, new) };
                        if q.is_null() {
                            assert!(filled(p as usize, l.size(), m), "step {i}: a refused realloc lost its block");
                            '0'
                        } else {
                            assert!(filled(q as usize, l.size().min(new), m), "step {i}: a realloc lost its contents");
                            std::ptr::write_bytes(q, m, new);
                            live[slot] = Some((q, Layout::from_size_align(new, l.align()).unwrap(), m));
                            '1'
                        }
                    }
                },
                Op::Free(slot) => match live[slot].take() {
                    None => '-',
                    Some((p, l, m)) => {
                        assert!(filled(p as usize, l.size(), m), "step {i}: a block lost its contents");
                        if branch {
                            TeqAlloc.dealloc(p, l)
                        } else {
                            System.dealloc(p, l)
                        }
                        '1'
                    }
                },
            };
            outcomes.push(outcome);
        }
        if let Some(old) = old {
            bound::restore(&old);
        }
        for (p, l, m) in live.into_iter().flatten() {
            assert!(filled(p as usize, l.size(), m));
            if branch {
                TeqAlloc.dealloc(p, l)
            } else {
                System.dealloc(p, l)
            }
        }
        outcomes
    }

    /// The differential test of the realloc rule (the push pass of 6280b3c82): seeded sequences of
    /// requests over 32 KB, before the process is resident and after, the rest bounded at the address
    /// space it has (no room for a request that needs address space of its own), each run once through
    /// the global allocator and once through the system's allocator alone, each in a process of its own
    /// on glibc's main arena; the global allocator may refuse a step only where the system's alone
    /// refused it too, and every block keeps its contents. glibc's threshold for mapping a chunk of its
    /// own is pinned at the small blocks' bound on both sides, so that it maps each block the global
    /// allocator maps: left to rise with the chunks freed, as it does, it serves blocks of these sizes from
    /// its heap, where at the bound whether a request fits depends on which neighbours of its block were
    /// freed before, and any difference of placement shows both ways. There the global allocator's
    /// outcomes are those of the global allocator never made resident, whose blocks are all the system's
    /// but for the word ahead of each, and that word alone changes two sequences in twenty: one step
    /// refused that the system's alone served, seven served that it refused.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_allocator_refuses_only_what_the_system_alone_refuses() {
        const SEEDS: [u64; 4] = [0x9e37_79b9, 0x85eb_ca6b, 0xc2b2_ae35, 0x27d4_eb2f];
        if let Some(run) = std::env::var_os("TEQ_ALLOC_DIFFERENTIAL") {
            let run = run.into_string().unwrap();
            let (side, seed) = run.split_once(':').unwrap();
            let outcomes = unsafe { differential_run(side == "branch", seed.parse().unwrap()) };
            println!("DIFFERENTIAL {outcomes}");
            return;
        }
        let name = "alloc::tests::the_allocator_refuses_only_what_the_system_alone_refuses";
        for seed in SEEDS {
            let run = |side: &str| -> String {
                let out = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", name, "--test-threads=1", "--nocapture"])
                    .env("TEQ_ALLOC_DIFFERENTIAL", format!("{side}:{seed}"))
                    .env("MALLOC_ARENA_MAX", "1")
                    .env("MALLOC_MMAP_THRESHOLD_", MAX_SIZE.to_string()) // glibc's M_MMAP_THRESHOLD
                    .output()
                    .unwrap();
                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                assert!(out.status.success(), "the {side} run of seed {seed}: {}\n{text}{}", out.status, String::from_utf8_lossy(&out.stderr));
                let at = text.find("DIFFERENTIAL ").unwrap_or_else(|| panic!("the {side} run of seed {seed} said nothing: {text}"));
                text[at + "DIFFERENTIAL ".len()..].split_whitespace().next().unwrap().to_string()
            };
            let (system, branch) = (run("system"), run("branch"));
            let ops = differential_ops(seed, system.len());
            let refused: Vec<String> = branch
                .chars()
                .zip(system.chars())
                .enumerate()
                .filter(|&(_, (b, s))| b == '0' && s == '1')
                .map(|(i, _)| format!("step {i} {:?}", ops[i]))
                .collect();
            assert!(refused.is_empty(), "seed {seed}: refused where the system's allocator alone served: {}\nbranch {branch}\nsystem {system}", refused.join(", "));
            assert!(system.contains('0'), "seed {seed}: the bound refused the system's allocator nothing; the sequence tests no refusal");
        }
    }

    /// Threads that free mappings and take them again while another unmaps the ones kept: no block's
    /// contents go, and an idle point after they ended leaves none kept.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn kept_mappings_unmapped_beside_threads_that_free_and_take_them() {
        if !alone("kept_mappings_unmapped_beside_threads_that_free_and_take_them") {
            return;
        }
        let _serial = serial();
        const THREADS: usize = 8;
        const ROUNDS: usize = 200;
        const SIZES: [usize; 3] = [64 << 10, 1 << 20, 3 << 20];
        let done = std::sync::Arc::new(AtomicUsize::new(0));
        let workers: Vec<_> = (0..THREADS)
            .map(|t| {
                let done = done.clone();
                spawn(move || {
                    for round in 0..ROUNDS {
                        let size = SIZES[(t + round) % SIZES.len()];
                        let byte = (t * ROUNDS + round) as u8;
                        unsafe {
                            let p = fill(TeqAlloc.alloc(layout(size, 8)), size, byte);
                            std::thread::yield_now();
                            assert!(filled(p as usize, size, byte), "a block's pages went while it was live");
                            TeqAlloc.dealloc(p, layout(size, 8));
                        }
                    }
                    done.fetch_add(1, Ordering::Relaxed);
                })
            })
            .collect();
        while done.load(Ordering::Relaxed) < THREADS {
            unmap_kept();
            std::thread::yield_now();
        }
        for w in workers {
            w.join().unwrap();
        }
        idle();
        assert_eq!((held().kept_mappings, held().mapped_kept), (0, 0));
    }

    #[test]
    fn a_thread_that_only_frees_hands_them_over() {
        if !alone("a_thread_that_only_frees_hands_them_over") {
            return;
        }
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
        ("src/alloc.rs", 6, "spawn, spawn_or_drop and spawn_in themselves, which enter first"),
        ("src/classpath.rs", 1, "alloc::spawn_in"),
        ("src/crew.rs", 2, "a builder for the merge threads, alloc::enter the first statement of the closure; the retired data's free through spawn"),
        ("src/emit/layout.rs", 2, "alloc::spawn_in for the scan of the anonymous classes' creations and for their shapes"),
        ("src/emit/mod.rs", 1, "alloc::spawn_in"),
        ("src/emit/outline.rs", 1, "alloc::spawn_in"),
        ("src/fix/engine.rs", 2, "a builder for the stacks' size of teq fix's rule workers (`Pool::add_worker`) and typing thread, alloc::enter the first statement of each closure"),
        ("src/frontend.rs", 1, "alloc::spawn_in"),
        ("src/interp/net.rs", 1, "a builder for the connect's waiter, alloc::enter the first statement of its closure"),
        ("src/interp/process.rs", 2, "a builder for the reaper's small stack, alloc::enter the first statement of its closure; the other starts a program's child under teq interp"),
        ("src/jvm/classfile.rs", 1, "alloc::spawn_in"),
        ("src/jvm/kept.rs", 1, "alloc::spawn_in for the slices of the kept class files' facts"),
        ("src/jvm/mod.rs", 2, "alloc::spawn_in for the class generation and for the class files' writing"),
        ("src/lsp/export.rs", 2, "alloc::spawn for the thread that waits on sbt; the other starts the sbt process"),
        ("src/lsp/mod.rs", 7, "alloc::spawn six times, the generators' worker among them; the seventh starts a child process"),
        ("src/lsp/sources.rs", 1, "alloc::spawn for the workers that fetch the libraries' sources"),
        ("src/main.rs", 1, "a builder for the stack's size; alloc::enter is the first statement of compile"),
        ("src/products.rs", 1, "alloc::spawn_in for the publication's steps"),
        ("src/task/client.rs", 1, "starts the daemon process"),
        ("src/task/daemon.rs", 5, "alloc::spawn for the watch, per connection, per test run's client watcher and per spent runner's quitting, alloc::spawn_in per resident's build"),
        ("src/task/export.rs", 1, "starts git check-ignore for the staleness check"),
        ("src/task/fetch.rs", 2, "starts curl; alloc::spawn_in per transfer worker of a cold fetch"),
        ("src/task/job.rs", 2, "starts dev's dev command and run's JVM, once per platform's branch; taskkill runs to its end through status"),
        ("src/task/jsrunner.rs", 2, "alloc::spawn for the threads that forward node's stdout and stderr; the other starts node"),
        ("src/task/resident.rs", 2, "alloc::spawn for the thread that drains the resident's stderr; the other starts the resident"),
        ("src/task/runner.rs", 3, "alloc::spawn for the threads that drain the test runner's stderr and a retired runner's stdout; the other starts the runner's JVM"),
        ("src/typer/thread.rs", 1, "a builder for the stack's size; alloc::enter is the first statement of the closure"),
        ("src/typer/inlining.rs", 1, "a builder for the inlining phase's workers' stack size, alloc::enter the first statement of each worker's closure"),
        ("src/typer/mod.rs", 1, "a builder for the stack's size, alloc::enter the first statement of each worker's closure (the merge's free of the other workers' variables' tables goes through crew::give_back)"),
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
        if !alone("what_a_thread_locals_destructor_frees_comes_back") {
            return;
        }
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
