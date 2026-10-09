//! Reads zip archives (jars): the central directory when the archive is opened, the data of an
//! entry when it is asked for. Stored and deflated entries; no zip64, no encryption.

use std::fs::File;

pub struct Entry {
    name_start: u32,
    name_len: u16,
    pub method: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub size: u32,
    header_offset: u32,
}

pub struct Zip {
    file: File,
    /// The central directory as it stands in the file; entry names point into it.
    directory: Vec<u8>,
    pub entries: Vec<Entry>,
}

const END_OF_DIRECTORY: u32 = 0x0605_4b50;
const DIRECTORY_ENTRY: u32 = 0x0201_4b50;
const LOCAL_HEADER: u32 = 0x0403_4b50;
const LOCAL_HEADER_SIZE: usize = 30;
/// Read along with the local header, so that one read serves an entry with a short extra field.
const EXTRA_GUESS: usize = 64;

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// `len` bytes from `offset`, or fewer at the file's end: a positional read, which moves no
/// cursor, since threads read one jar at once (a macro's resource beside the loader's classes);
/// Windows' leaves the cursor at the read's end, and nothing reads by the cursor.
fn read_at(file: &File, offset: u64, len: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; len];
    let mut filled = 0;
    while filled < len {
        #[cfg(unix)]
        let read = std::os::unix::fs::FileExt::read_at(file, &mut buf[filled..], offset + filled as u64);
        #[cfg(windows)]
        let read = std::os::windows::fs::FileExt::seek_read(file, &mut buf[filled..], offset + filled as u64);
        match read {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    buf.truncate(filled);
    Ok(buf)
}

impl Zip {
    pub fn open(path: &str) -> Result<Zip, String> {
        let file = File::open(path).map_err(|e| format!("cannot open {}: {}", path, e))?;
        let file_len = file.metadata().map_err(|e| e.to_string())?.len();
        let tail_len = file_len.min(22 + 65535) as usize;
        let tail = read_at(&file, file_len - tail_len as u64, tail_len)?;
        let end = (0..tail.len().saturating_sub(21))
            .rev()
            .find(|&i| u32_at(&tail, i) == END_OF_DIRECTORY)
            .ok_or_else(|| format!("{}: not a zip archive (no end of central directory)", path))?;
        let count = u16_at(&tail, end + 10) as usize;
        let dir_size = u32_at(&tail, end + 12);
        let dir_offset = u32_at(&tail, end + 16);
        if count == 0xffff || dir_size == u32::MAX || dir_offset == u32::MAX {
            return Err(format!("{}: zip64 archives are not supported", path));
        }
        let directory = read_at(&file, dir_offset as u64, dir_size as usize)?;
        let mut entries = Vec::with_capacity(count);
        let mut at = 0usize;
        while at + 46 <= directory.len() && u32_at(&directory, at) == DIRECTORY_ENTRY {
            let name_len = u16_at(&directory, at + 28);
            let extra_len = u16_at(&directory, at + 30) as usize;
            let comment_len = u16_at(&directory, at + 32) as usize;
            entries.push(Entry {
                name_start: (at + 46) as u32,
                name_len,
                method: u16_at(&directory, at + 10),
                crc32: u32_at(&directory, at + 16),
                compressed_size: u32_at(&directory, at + 20),
                size: u32_at(&directory, at + 24),
                header_offset: u32_at(&directory, at + 42),
            });
            at += 46 + name_len as usize + extra_len + comment_len;
        }
        if entries.len() != count {
            return Err(format!("{}: central directory lists {} entries, found {}", path, count, entries.len()));
        }
        Ok(Zip { file, directory, entries })
    }

    /// The index of the entry of that name, if any.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| self.name(e) == name)
    }

    pub fn name(&self, e: &Entry) -> &str {
        let bytes = &self.directory[e.name_start as usize..e.name_start as usize + e.name_len as usize];
        std::str::from_utf8(bytes).unwrap_or("")
    }

    /// The uncompressed data of the entry, checked against its CRC-32.
    pub fn read(&self, index: usize) -> Result<Vec<u8>, String> {
        let e = &self.entries[index];
        let wanted = LOCAL_HEADER_SIZE + e.name_len as usize + EXTRA_GUESS + e.compressed_size as usize;
        let mut raw = read_at(&self.file, e.header_offset as u64, wanted)?;
        if raw.len() < LOCAL_HEADER_SIZE || u32_at(&raw, 0) != LOCAL_HEADER {
            return Err(format!("{}: bad local header", self.name(e)));
        }
        let data_start = LOCAL_HEADER_SIZE + u16_at(&raw, 26) as usize + u16_at(&raw, 28) as usize;
        let data_end = data_start + e.compressed_size as usize;
        if data_end > raw.len() {
            raw = read_at(&self.file, e.header_offset as u64, data_end)?;
            if data_end > raw.len() {
                return Err(format!("{}: truncated entry", self.name(e)));
            }
        }
        let data = match e.method {
            0 => {
                raw.truncate(data_end);
                raw.drain(..data_start);
                raw
            }
            8 => inflate(&raw[data_start..data_end], e.size as usize).map_err(|m| format!("{}: {}", self.name(e), m))?,
            m => return Err(format!("{}: unsupported compression method {}", self.name(e), m)),
        };
        if data.len() != e.size as usize || crc32(&data) != e.crc32 {
            return Err(format!("{}: CRC-32 or size mismatch", self.name(e)));
        }
        Ok(data)
    }
}

// ---- CRC-32 (slicing by 8) ----

static CRC_TABLES: std::sync::OnceLock<Box<[[u32; 256]; 8]>> = std::sync::OnceLock::new();

fn crc_tables() -> &'static [[u32; 256]; 8] {
    CRC_TABLES.get_or_init(|| {
        let mut t = Box::new([[0u32; 256]; 8]);
        for i in 0..256u32 {
            let mut c = i;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            t[0][i as usize] = c;
        }
        for i in 0..256 {
            for k in 1..8 {
                let prev = t[k - 1][i];
                t[k][i] = (prev >> 8) ^ t[0][(prev & 0xff) as usize];
            }
        }
        t
    })
}

pub fn crc32(data: &[u8]) -> u32 {
    let t = crc_tables();
    let mut crc = !0u32;
    let mut chunks = data.chunks_exact(8);
    for c in &mut chunks {
        let lo = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ crc;
        let hi = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
        crc = t[7][(lo & 0xff) as usize]
            ^ t[6][((lo >> 8) & 0xff) as usize]
            ^ t[5][((lo >> 16) & 0xff) as usize]
            ^ t[4][(lo >> 24) as usize]
            ^ t[3][(hi & 0xff) as usize]
            ^ t[2][((hi >> 8) & 0xff) as usize]
            ^ t[1][((hi >> 16) & 0xff) as usize]
            ^ t[0][(hi >> 24) as usize];
    }
    for &b in chunks.remainder() {
        crc = t[0][((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    !crc
}

// ---- DEFLATE (RFC 1951) ----

const FAST_BITS: u32 = 10;
const MAX_CODE_LEN: usize = 15;

/// A canonical Huffman code: codes of up to `FAST_BITS` bits are decoded with one table lookup
/// (`symbol << 4 | length`, 0 for none), longer ones bit by bit over the counts per length.
struct Huffman {
    fast: Vec<u16>,
    count: [u16; MAX_CODE_LEN + 1],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Huffman, &'static str> {
        let mut count = [0u16; MAX_CODE_LEN + 1];
        for &l in lengths {
            count[l as usize] += 1;
        }
        count[0] = 0;
        let mut left = 1i32;
        for len in 1..=MAX_CODE_LEN {
            left = (left << 1) - count[len] as i32;
            if left < 0 {
                return Err("over-subscribed Huffman code");
            }
        }
        let mut offsets = [0u16; MAX_CODE_LEN + 2];
        let mut next_code = [0u32; MAX_CODE_LEN + 2];
        for len in 1..=MAX_CODE_LEN {
            offsets[len + 1] = offsets[len] + count[len];
            next_code[len + 1] = (next_code[len] + count[len] as u32) << 1;
        }
        let mut symbols = vec![0u16; lengths.iter().filter(|&&l| l != 0).count()];
        let mut fast = vec![0u16; 1 << FAST_BITS];
        for (sym, &l) in lengths.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let len = l as usize;
            symbols[offsets[len] as usize] = sym as u16;
            offsets[len] += 1;
            let code = next_code[len];
            next_code[len] += 1;
            if len as u32 <= FAST_BITS {
                let reversed = code.reverse_bits() >> (32 - len as u32);
                let entry = ((sym as u16) << 4) | len as u16;
                let mut i = reversed as usize;
                while i < fast.len() {
                    fast[i] = entry;
                    i += 1 << len;
                }
            }
        }
        Ok(Huffman { fast, count, symbols })
    }
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u64,
    n: u32,
}

impl<'a> Bits<'a> {
    #[inline]
    fn refill(&mut self) {
        while self.n <= 56 && self.pos < self.data.len() {
            self.buf |= (self.data[self.pos] as u64) << self.n;
            self.pos += 1;
            self.n += 8;
        }
    }

    #[inline]
    fn take(&mut self, count: u32) -> Result<u32, &'static str> {
        if self.n < count {
            self.refill();
            if self.n < count {
                return Err("unexpected end of deflate stream");
            }
        }
        let v = (self.buf & ((1u64 << count) - 1)) as u32;
        self.buf >>= count;
        self.n -= count;
        Ok(v)
    }

    #[inline]
    fn decode(&mut self, h: &Huffman) -> Result<u16, &'static str> {
        if self.n < MAX_CODE_LEN as u32 {
            self.refill();
        }
        let entry = h.fast[(self.buf & ((1 << FAST_BITS) - 1)) as usize];
        if entry != 0 {
            let len = (entry & 15) as u32;
            if len > self.n {
                return Err("unexpected end of deflate stream");
            }
            self.buf >>= len;
            self.n -= len;
            return Ok(entry >> 4);
        }
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        let mut bits = self.buf;
        for len in 1..=MAX_CODE_LEN {
            if len as u32 > self.n {
                return Err("unexpected end of deflate stream");
            }
            code |= (bits & 1) as i32;
            bits >>= 1;
            let count = h.count[len] as i32;
            if code - count < first {
                self.buf >>= len;
                self.n -= len as u32;
                return Ok(h.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("invalid Huffman code")
    }
}

const LENGTH_BASE: [u16; 29] =
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LENGTH_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

static FIXED: std::sync::OnceLock<(Huffman, Huffman)> = std::sync::OnceLock::new();

fn fixed_codes() -> &'static (Huffman, Huffman) {
    FIXED.get_or_init(|| {
        let mut lit = [8u8; 288];
        lit[144..256].fill(9);
        lit[256..280].fill(7);
        (Huffman::new(&lit).unwrap(), Huffman::new(&[5u8; 30]).unwrap())
    })
}

fn dynamic_codes(bits: &mut Bits) -> Result<(Huffman, Huffman), &'static str> {
    let n_lit = bits.take(5)? as usize + 257;
    let n_dist = bits.take(5)? as usize + 1;
    let n_code = bits.take(4)? as usize + 4;
    if n_lit > 286 || n_dist > 30 {
        return Err("too many length or distance codes");
    }
    let mut code_lengths = [0u8; 19];
    for &slot in &CODE_LENGTH_ORDER[..n_code] {
        code_lengths[slot] = bits.take(3)? as u8;
    }
    let code_code = Huffman::new(&code_lengths)?;
    let mut lengths = [0u8; 286 + 30];
    let mut i = 0;
    while i < n_lit + n_dist {
        let sym = bits.decode(&code_code)?;
        let (value, repeat) = match sym {
            0..=15 => (sym as u8, 1),
            16 if i == 0 => return Err("repeat without a previous length"),
            16 => (lengths[i - 1], 3 + bits.take(2)? as usize),
            17 => (0, 3 + bits.take(3)? as usize),
            _ => (0, 11 + bits.take(7)? as usize),
        };
        if i + repeat > n_lit + n_dist {
            return Err("too many code lengths");
        }
        lengths[i..i + repeat].fill(value);
        i += repeat;
    }
    if lengths[256] == 0 {
        return Err("no end-of-block code");
    }
    Ok((Huffman::new(&lengths[..n_lit])?, Huffman::new(&lengths[n_lit..n_lit + n_dist])?))
}

pub fn inflate(data: &[u8], size_hint: usize) -> Result<Vec<u8>, &'static str> {
    let mut out: Vec<u8> = Vec::with_capacity(size_hint);
    let mut bits = Bits { data, pos: 0, buf: 0, n: 0 };
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => {
                let skip = bits.n % 8;
                bits.take(skip)?;
                let len = bits.take(16)? as usize;
                let complement = bits.take(16)? as usize;
                if len != !complement & 0xffff {
                    return Err("stored block length mismatch");
                }
                // The bit buffer holds whole bytes here; hand them back before copying.
                let start = bits.pos - (bits.n / 8) as usize;
                if start + len > data.len() {
                    return Err("unexpected end of deflate stream");
                }
                out.extend_from_slice(&data[start..start + len]);
                bits = Bits { data, pos: start + len, buf: 0, n: 0 };
            }
            kind @ (1 | 2) => {
                let dynamic;
                let (lit, dist) = if kind == 1 {
                    let f = fixed_codes();
                    (&f.0, &f.1)
                } else {
                    dynamic = dynamic_codes(&mut bits)?;
                    (&dynamic.0, &dynamic.1)
                };
                loop {
                    let sym = bits.decode(lit)? as usize;
                    if sym < 256 {
                        out.push(sym as u8);
                        continue;
                    }
                    if sym == 256 {
                        break;
                    }
                    let li = sym - 257;
                    if li >= 29 {
                        return Err("invalid length symbol");
                    }
                    let len = LENGTH_BASE[li] as usize + bits.take(LENGTH_EXTRA[li] as u32)? as usize;
                    let di = bits.decode(dist)? as usize;
                    if di >= 30 {
                        return Err("invalid distance symbol");
                    }
                    let distance = DIST_BASE[di] as usize + bits.take(DIST_EXTRA[di] as u32)? as usize;
                    if distance > out.len() {
                        return Err("distance too far back");
                    }
                    let from = out.len() - distance;
                    if distance >= len {
                        out.extend_from_within(from..from + len);
                    } else {
                        for k in 0..len {
                            let b = out[from + k];
                            out.push(b);
                        }
                    }
                }
            }
            _ => return Err("invalid block type"),
        }
        if last {
            return Ok(out);
        }
    }
}

#[cfg(test)]
mod tests {
    /// Threads reading one jar at once each read their own entry's bytes: a read moves nothing
    /// another thread's read relies on (a macro's resource on a worker beside the loader's
    /// classes, which seeking a shared cursor gave a bad header or a mismatched checksum).
    #[test]
    fn threads_read_one_jar_at_once() {
        let dir = std::env::temp_dir().join(format!("teq-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("many.jar");
        let path = path.to_str().unwrap();
        let classes: Vec<(String, Vec<u8>)> = (0..64).map(|i| (format!("p/C{i}"), vec![i as u8; 4096 + i * 37])).collect();
        crate::jvm::classfile::write_jar(path, None, &classes).unwrap();
        let zip = super::Zip::open(path).unwrap();
        std::thread::scope(|s| {
            for t in 0..8 {
                let (zip, classes) = (&zip, &classes);
                s.spawn(move || {
                    for round in 0..300 {
                        let (name, bytes) = &classes[(t * 7 + round) % classes.len()];
                        let i = zip.find(&format!("{name}.class")).unwrap();
                        assert_eq!(&zip.read(i).unwrap(), bytes, "{name} read by thread {t}");
                    }
                });
            }
        });
        let _ = std::fs::remove_dir_all(&dir);
    }
}
