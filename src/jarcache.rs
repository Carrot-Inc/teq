//! The jar cache: per jar, a file holding the TASTy entries builds
//! have read from it, inflated, so that the next build reads them without inflating. The file
//! is keyed by the jar's path, size and modification time and by the teq binary's version and
//! git hash; each entry carries the jar's CRC-32 for it and a checksum of the bytes. A missing,
//! stale or corrupt file is read around: the jar serves, and the file is written anew.
//!
//! Layout, little-endian: the magic, the header's length, the header (the key, the entry count,
//! per entry its index in the jar, the jar's CRC-32, the length and the checksum), the header's
//! checksum, then the entries' bytes in the header's order.

use crate::intern::FxMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"TEQJAR\x00\x01";
const ENTRY_BYTES: usize = 20;

/// Whether `--no-cache` or `TEQ_NO_CACHE` left the cache out.
pub fn disabled_by_env() -> bool {
    std::env::var_os("TEQ_NO_CACHE").map_or(false, |v| !v.is_empty() && v != "0")
}

/// teq's cache (`task::fetch::cache_root`): `$TEQ_CACHE_DIR`, else `$XDG_CACHE_HOME/teq`, else the
/// platform's cache directory: `~/Library/Caches/teq` on macOS, `%LOCALAPPDATA%\teq` on Windows,
/// `~/.cache/teq` elsewhere.
pub fn dir() -> Option<PathBuf> {
    crate::task::fetch::cache_root()
}

/// A 64-bit checksum, four lanes of 8 bytes at a time: a corrupt or truncated entry is found
/// at the speed of memory.
pub fn checksum(bytes: &[u8]) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mix = |h: u64, w: u64| (h.rotate_left(23) ^ w).wrapping_mul(K);
    let mut lanes = [bytes.len() as u64, 1, 2, 3];
    let mut blocks = bytes.chunks_exact(32);
    for b in &mut blocks {
        for (i, lane) in lanes.iter_mut().enumerate() {
            *lane = mix(*lane, u64::from_le_bytes(b[i * 8..i * 8 + 8].try_into().unwrap()));
        }
    }
    let mut h = lanes.iter().fold(0u64, |h, &l| mix(h, l));
    for c in blocks.remainder().chunks(8) {
        let mut w = [0u8; 8];
        w[..c.len()].copy_from_slice(c);
        h = mix(h, u64::from_le_bytes(w));
    }
    h ^ (h >> 29)
}

#[derive(Clone, Copy)]
struct Cached {
    crc: u32,
    offset: u64,
    len: u32,
    sum: u64,
}

/// The cache of one jar: its file, when there is a valid one, and the entries it holds.
pub struct JarCache {
    path: PathBuf,
    key: String,
    file: Option<File>,
    entries: FxMap<u32, Cached>,
    /// An entry's bytes did not match their checksum: nothing more is read from the file, and
    /// it is written anew.
    pub broken: bool,
}

impl JarCache {
    /// The cache of the jar at `jar`, with the entries of its file when the file is there and
    /// its key and header check out; `None` without a cache directory or the jar's metadata.
    pub fn open(jar: &str, dir: &Path) -> Option<JarCache> {
        let canonical = crate::source::canonicalize(jar).ok()?;
        let meta = std::fs::metadata(&canonical).ok()?;
        let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
        let canonical = canonical.to_string_lossy().into_owned();
        let key = format!("{}\n{}\n{}\n{}\n{}", canonical, meta.len(), mtime, env!("CARGO_PKG_VERSION"), env!("TEQ_GIT_HASH"));
        let stem = Path::new(&canonical).file_stem().map_or_else(|| "jar".to_string(), |s| s.to_string_lossy().into_owned());
        let path = dir.join(format!("{}-{:016x}.jarcache", stem, checksum(canonical.as_bytes())));
        let mut cache = JarCache { path, key, file: None, entries: FxMap::default(), broken: false };
        if let Some((file, entries)) = cache.read_header() {
            cache.file = Some(file);
            cache.entries = entries;
        }
        Some(cache)
    }

    fn read_header(&self) -> Option<(File, FxMap<u32, Cached>)> {
        let mut file = File::open(&self.path).ok()?;
        let mut head = [0u8; 12];
        file.read_exact(&mut head).ok()?;
        if &head[..8] != MAGIC {
            return None;
        }
        let len = u32::from_le_bytes(head[8..12].try_into().unwrap()) as usize;
        let mut header = vec![0u8; len + 8];
        file.read_exact(&mut header).ok()?;
        let sum = u64::from_le_bytes(header[len..].try_into().unwrap());
        let header = &header[..len];
        if checksum(header) != sum {
            return None;
        }
        let mut r = Cursor { bytes: header, pos: 0 };
        let key_len = r.u32()? as usize;
        if r.take(key_len)? != self.key.as_bytes() {
            return None;
        }
        let count = r.u32()? as usize;
        let mut offset = 12 + len as u64 + 8;
        let mut entries = FxMap::default();
        for _ in 0..count {
            let index = r.u32()?;
            let crc = r.u32()?;
            let len = r.u32()?;
            let sum = r.u64()?;
            entries.insert(index, Cached { crc, offset, len, sum });
            offset += len as u64;
        }
        Some((file, entries))
    }

    /// Whether the file holds entry `index` as the jar's directory describes it.
    pub fn has(&self, index: u32, crc: u32) -> bool {
        !self.broken && self.entries.get(&index).map_or(false, |c| c.crc == crc)
    }

    /// The bytes of entry `index`, when the file holds them for the jar's CRC-32 `crc` and they
    /// match their checksum.
    pub fn read(&mut self, index: u32, crc: u32) -> Option<Vec<u8>> {
        if !self.has(index, crc) {
            return None;
        }
        let c = self.entries[&index];
        let bytes = self.read_at(c)?;
        if checksum(&bytes) != c.sum {
            self.broken = true;
            return None;
        }
        Some(bytes)
    }

    fn read_at(&mut self, c: Cached) -> Option<Vec<u8>> {
        let file = self.file.as_mut()?;
        let mut bytes = vec![0u8; c.len as usize];
        file.seek(SeekFrom::Start(c.offset)).ok()?;
        match file.read_exact(&mut bytes) {
            Ok(()) => Some(bytes),
            Err(_) => {
                self.broken = true;
                None
            }
        }
    }

    /// Writes the file anew with the entries `read` (index in the jar, CRC-32, bytes) and those
    /// the file held already, by their index; the bytes written.
    pub fn save(&mut self, read: &[(u32, u32, &[u8])]) -> std::io::Result<usize> {
        let mut kept: Vec<(u32, u32, Vec<u8>)> = Vec::new();
        if !self.broken {
            let fresh: FxMap<u32, ()> = read.iter().map(|&(i, _, _)| (i, ())).collect();
            let mut old: Vec<(u32, Cached)> = self.entries.iter().filter(|(i, _)| !fresh.contains_key(i)).map(|(&i, &c)| (i, c)).collect();
            old.sort_by_key(|&(i, _)| i);
            for (i, c) in old {
                if let Some(bytes) = self.read_at(c).filter(|b| checksum(b) == c.sum) {
                    kept.push((i, c.crc, bytes));
                }
            }
        }
        let mut all: Vec<(u32, u32, &[u8])> = read.to_vec();
        all.extend(kept.iter().map(|(i, crc, b)| (*i, *crc, b.as_slice())));
        all.sort_by_key(|&(i, _, _)| i);
        let mut header = Vec::with_capacity(8 + self.key.len() + all.len() * ENTRY_BYTES);
        header.extend_from_slice(&(self.key.len() as u32).to_le_bytes());
        header.extend_from_slice(self.key.as_bytes());
        header.extend_from_slice(&(all.len() as u32).to_le_bytes());
        for &(i, crc, bytes) in &all {
            header.extend_from_slice(&i.to_le_bytes());
            header.extend_from_slice(&crc.to_le_bytes());
            header.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            header.extend_from_slice(&checksum(bytes).to_le_bytes());
        }
        let dir = self.path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;
        let tmp = self.path.with_extension(format!("tmp{}", std::process::id()));
        let written = (|| {
            let mut out = std::io::BufWriter::with_capacity(1 << 20, File::create(&tmp)?);
            out.write_all(MAGIC)?;
            out.write_all(&(header.len() as u32).to_le_bytes())?;
            out.write_all(&header)?;
            out.write_all(&checksum(&header).to_le_bytes())?;
            let mut n = 12 + header.len() + 8;
            for &(_, _, bytes) in &all {
                out.write_all(bytes)?;
                n += bytes.len();
            }
            out.flush()?;
            Ok(n)
        })();
        match written.and_then(|n| std::fs::rename(&tmp, &self.path).map(|_| n)) {
            Ok(n) => Ok(n),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                Err(e)
            }
        }
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let out = self.bytes.get(self.pos..self.pos.checked_add(n)?)?;
        self.pos += n;
        Some(out)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    }

    fn u64(&mut self) -> Option<u64> {
        self.take(8).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
    }
}
