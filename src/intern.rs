use crate::shared::{hash_of, Serial, SlabVec, Table};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// The algorithm of rustc-hash 1.1.0's `FxHasher` (https://github.com/rust-lang/rustc-hash,
/// Copyright 2015 The Rust Project Developers, MIT or Apache-2.0, taken under Apache-2.0; see
/// `NOTICE`): each word rotated into the hash, xored and multiplied by the seed. The words are
/// read 8 bytes at a time, little-endian, the tail padded with zeros.
#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.add(u64::from_le_bytes(c.try_into().unwrap()));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut buf = [0u8; 8];
            buf[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(buf));
        }
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64)
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(i as u64)
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i)
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64)
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type FxBuild = BuildHasherDefault<FxHasher>;
pub type FxMap<K, V> = HashMap<K, V, FxBuild>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Name(pub u32);

/// The names, interned once: shared by reference across the parallel typer's workers
/// (`shared.rs`), whose reads take no lock and whose inserts take one unless one thread owns
/// the interner (`set_exclusive`).
pub struct Interner {
    serial: Serial,
    strs: SlabVec<&'static str>,
    index: Table,
    chunk: UnsafeCell<String>,
    full_chunks: UnsafeCell<Vec<String>>,
}

unsafe impl Sync for Interner {}

const CHUNK_SIZE: usize = 64 * 1024;

impl Interner {
    pub fn new() -> Self {
        let mut i = Interner {
            serial: Serial::new(true, crate::measure::Wait::Interner),
            strs: SlabVec::with_capacity(4096),
            index: Table::with_capacity(4096),
            chunk: UnsafeCell::new(String::with_capacity(CHUNK_SIZE)),
            full_chunks: UnsafeCell::new(Vec::new()),
        };
        crate::names::prefill(&mut i);
        i
    }

    /// Frees the buffers the interner outgrew (`TypeStore::release_retired`).
    pub fn release_retired(&self) {
        unsafe {
            self.strs.release_retired();
            self.index.release_retired();
        }
    }

    /// Whether one thread owns the interner, whose inserts then take no lock.
    pub fn set_exclusive(&self, exclusive: bool) {
        self.serial.set_exclusive(exclusive);
    }

    #[inline]
    fn find(&self, h: u64, s: &str) -> Option<Name> {
        self.index.find(h, #[inline(always)] |i| *self.strs.get(i as usize) == s).map(Name)
    }

    pub fn intern(&self, s: &str) -> Name {
        let h = hash_of(s);
        if let Some(n) = self.find(h, s) {
            return n;
        }
        let held = self.serial.hold();
        if held.is_some() {
            if let Some(n) = self.find(h, s) {
                return n;
            }
        }
        // The writer's, under the lock: the chunks are appended and never reallocated
        // (the capacity is checked), and live as long as the interner.
        let stored: &'static str = unsafe {
            let chunk = &mut *self.chunk.get();
            if chunk.capacity() - chunk.len() < s.len() {
                let cap = CHUNK_SIZE.max(s.len());
                let old = std::mem::replace(chunk, String::with_capacity(cap));
                (*self.full_chunks.get()).push(old);
            }
            let start = chunk.len();
            chunk.push_str(s);
            &*(&chunk[start..] as *const str)
        };
        let n = self.strs.push(stored) as u32;
        self.index.insert(h, n);
        Name(n)
    }

    pub fn len(&self) -> usize {
        self.strs.len()
    }

    /// The bytes the names hold: their texts' chunks, the table and the list. Only ever more:
    /// a name stays for the session.
    pub fn held(&self) -> usize {
        // SAFETY: a read of the writer's chunks between two inserts.
        let chunks = unsafe { (*self.chunk.get()).capacity() + (*self.full_chunks.get()).iter().map(String::capacity).sum::<usize>() + crate::held::array(&*self.full_chunks.get()) };
        chunks + self.strs.held() + self.index.held()
    }

    /// The name of `s` when it was interned, without interning it.
    pub fn lookup(&self, s: &str) -> Option<Name> {
        self.find(hash_of(s), s)
    }

    #[inline]
    pub fn get(&self, n: Name) -> &str {
        self.strs.get(n.0 as usize)
    }
}
