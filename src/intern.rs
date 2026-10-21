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
/// A name's text, and its encoded form once read (`Interner::encoded`): the encoded name's id plus one, 0 before.
struct Slot {
    text: &'static str,
    encoded: std::sync::atomic::AtomicU32,
}

pub struct Interner {
    serial: Serial,
    strs: SlabVec<Slot>,
    index: Table,
    chunk: UnsafeCell<String>,
    full_chunks: UnsafeCell<Vec<String>>,
    /// The derived names made (`unique`), by separator and number: the writer's, under the lock.
    unique: UnsafeCell<Vec<(&'static str, u32, Name)>>,
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
            unique: UnsafeCell::new(Vec::new()),
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
        self.index.find(h, #[inline(always)] |i| self.strs.get(i as usize).text == s).map(Name)
    }

    /// A derived name of a `UniqueNameKind` (dotty's `NameKinds`, `ContextBoundParamName`'s `evidence$` and its
    /// number): its text the separator and the number, which it is shown as, and its identity none of a written
    /// identifier's, the index holding it not (`intern("evidence$1")` gives another name); one per separator and
    /// number, as dotty's derived names are.
    #[cold]
    #[inline(never)]
    pub fn unique(&self, separator: &'static str, num: u32) -> Name {
        let _held = self.serial.hold();
        // SAFETY: the writer's list, read and appended under the lock (or by the thread that owns the interner).
        let made = unsafe { &mut *self.unique.get() };
        if let Some(&(_, _, n)) = made.iter().find(|&&(s, k, _)| s == separator && k == num) {
            return n;
        }
        let n = Name(self.add_slot(&format!("{}{}", separator, num)));
        made.push((separator, num, n));
        n
    }

    /// Every derived name (`unique`) with its separator and number: what a merge of this interner's names into another
    /// makes again there by its kind, never by its text.
    pub fn derived_names(&self) -> Vec<(&'static str, u32, Name)> {
        let _held = self.serial.hold();
        // SAFETY: read under the lock, as `unique` writes it.
        unsafe { (*self.unique.get()).clone() }
    }

    /// The separator and the number of the derived name `n` (`unique`), none for a written one.
    #[cold]
    #[inline(never)]
    pub fn unique_parts(&self, n: Name) -> Option<(&'static str, u32)> {
        let _held = self.serial.hold();
        // SAFETY: read under the lock, as `unique` writes it.
        unsafe { (*self.unique.get()).iter().find(|&&(_, _, m)| m == n).map(|&(s, k, _)| (s, k)) }
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
        let n = self.add_slot(s);
        self.index.insert(h, n);
        Name(n)
    }

    /// A new name's slot of the text `s`, unindexed: `intern`'s and `unique`'s one append, the writer's, under the lock
    /// (the slots' one push site, which keeps their growth inside it).
    #[inline(never)]
    fn add_slot(&self, s: &str) -> u32 {
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
        self.strs.push(Slot { text: stored, encoded: std::sync::atomic::AtomicU32::new(0) }) as u32
    }

    /// The encoded form of `n` (`NameTransformer.encode`, dotty's `SimpleName.mangle`), `n` itself where encoding
    /// leaves it as it is: read the first time a name is asked, a load beside its text after.
    #[inline(always)]
    pub fn encoded(&self, n: Name) -> Name {
        match self.strs.get(n.0 as usize).encoded.load(std::sync::atomic::Ordering::Relaxed) {
            0 => self.encoded_now(n),
            v => Name(v - 1),
        }
    }

    /// `encoded`'s first read of `n`, kept in its slot under the writer's lock. A growth of the slots copies them
    /// byte for byte, the writer's, under that lock: a store beside it would race the copy, or land in the buffer it
    /// retires. The slot is found after the lock is taken, in the buffer the last growth published.
    #[cold]
    #[inline(never)]
    fn encoded_now(&self, n: Name) -> Name {
        let encoded = match crate::jvm::names::encode_name(self.get(n)) {
            std::borrow::Cow::Owned(e) if e != self.get(n) => self.intern(&e),
            _ => n,
        };
        let _held = self.serial.hold();
        self.strs.get(n.0 as usize).encoded.store(encoded.0 + 1, std::sync::atomic::Ordering::Relaxed);
        encoded
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
        self.strs.get(n.0 as usize).text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hash of teq's maps, a pin of its algorithm: the order an `FxMap` is walked in follows
    /// it, and a walk that reaches an output makes it part of the output. rustc-hash 1.1.0, whose
    /// word step this is, reads a tail by 4, 2 and 1 bytes and gives `c360d75917ea8923` here.
    /// The encoded-name cache written while another worker interns: the interner filled to the slab's capacity
    /// (32,768 slots), its names encoded by two threads while a third interns 40,000 more, which grows the slab
    /// twice. Every name's cached form is its encoding. Under ThreadSanitizer, with a std built by it, the run
    /// reports no race: `RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Zsanitizer=thread cargo test -Zbuild-std
    /// --target x86_64-unknown-linux-gnu --target-dir <dir> encoded_cache_beside_growth`.
    #[test]
    fn encoded_cache_beside_growth() {
        let interner = Interner::new();
        interner.set_exclusive(false);
        let mut k = 0;
        while interner.len() < 32_768 {
            interner.intern(&format!("n{k}-x"));
            k += 1;
        }
        let existing: Vec<Name> = (0..32_768u32).map(Name).collect();
        std::thread::scope(|s| {
            s.spawn(|| {
                for j in 0..40_000 {
                    interner.intern(&format!("m{j}"));
                }
            });
            for t in 0..2 {
                let (interner, existing) = (&interner, &existing);
                s.spawn(move || {
                    for &n in existing.iter().skip(t).step_by(2) {
                        interner.encoded(n);
                    }
                });
            }
        });
        for &n in &existing {
            assert_eq!(interner.get(interner.encoded(n)), crate::jvm::names::encode_name(interner.get(n)));
        }
    }

    #[test]
    fn fx_hasher_reads_a_tail_padded_with_zeros() {
        let mut h = FxHasher::default();
        h.write(b"abc");
        assert_eq!(h.finish(), 0x8c41_a58a_f392_0c75);
    }
}
