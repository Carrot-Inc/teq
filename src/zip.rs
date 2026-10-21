//! Reads zip archives (jars): the central directory when the archive is opened, the data of an
//! entry when it is asked for. Stored and deflated entries; no zip64, no encryption.

use std::fs::File;

pub struct Entry {
    name_start: u32,
    name_len: u16,
    pub method: u16,
    /// The DOS date and time, the date in the high half.
    pub dos_time: u32,
    pub crc32: u32,
    pub compressed_size: u32,
    pub size: u32,
    header_offset: u32,
    extra_len: u16,
    comment_len: u16,
}

pub struct Zip {
    file: File,
    /// The file's length when it was opened: what the directory says is read only within it.
    len: u64,
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
        // The directory lies before its end record and holds an entry's fixed 46 bytes for each entry it
        // lists: both checked before it is read, so that no size the end record declares is allocated
        // past what the file holds.
        let end_at = file_len - tail_len as u64 + end as u64;
        if dir_offset as u64 + dir_size as u64 > end_at {
            return Err(format!("{}: central directory of {} bytes at {} runs past its end record at {}", path, dir_size, dir_offset, end_at));
        }
        if count * 46 > dir_size as usize {
            return Err(format!("{}: central directory lists {} entries in {} bytes, fewer than the 46 each takes", path, count, dir_size));
        }
        let directory = read_at(&file, dir_offset as u64, dir_size as usize)?;
        let mut entries = Vec::with_capacity(count);
        let mut at = 0usize;
        while at + 46 <= directory.len() && u32_at(&directory, at) == DIRECTORY_ENTRY {
            let name_len = u16_at(&directory, at + 28);
            let extra_len = u16_at(&directory, at + 30) as usize;
            let comment_len = u16_at(&directory, at + 32) as usize;
            // An entry's name, extra field and comment lie within the directory, which every reader
            // of them slices.
            if at + 46 + name_len as usize + extra_len + comment_len > directory.len() {
                return Err(format!("{}: central directory entry {} runs past the directory's {} bytes", path, entries.len(), directory.len()));
            }
            entries.push(Entry {
                name_start: (at + 46) as u32,
                name_len,
                method: u16_at(&directory, at + 10),
                dos_time: u32_at(&directory, at + 12),
                crc32: u32_at(&directory, at + 16),
                compressed_size: u32_at(&directory, at + 20),
                size: u32_at(&directory, at + 24),
                header_offset: u32_at(&directory, at + 42),
                extra_len: extra_len as u16,
                comment_len: comment_len as u16,
            });
            at += 46 + name_len as usize + extra_len + comment_len;
        }
        if entries.len() != count {
            return Err(format!("{}: central directory lists {} entries, found {}", path, count, entries.len()));
        }
        Ok(Zip { file, len: file_len, directory, entries })
    }

    /// An archive opened whole: its central directory read (`open`), then every entry's local
    /// header read where the directory places it, its signature checked and its name, its extra field
    /// and its data, by the local header's own lengths, within the file. The one validation of a
    /// library's sources jar, wherever it lies (the language server's fetch and its sessions' opening
    /// alike), so that an archive taken is one each of whose entries reads as far as its size goes.
    pub fn open_valid(path: &str) -> Result<Zip, String> {
        let zip = Zip::open(path)?;
        let len = zip.len;
        for e in &zip.entries {
            let header = read_at(&zip.file, e.header_offset as u64, LOCAL_HEADER_SIZE)?;
            if header.len() < LOCAL_HEADER_SIZE || u32_at(&header, 0) != LOCAL_HEADER {
                return Err(format!("{}: {}: no local header at {}", path, zip.name(e), e.header_offset));
            }
            let end = e.header_offset as u64 + LOCAL_HEADER_SIZE as u64 + u16_at(&header, 26) as u64 + u16_at(&header, 28) as u64 + e.compressed_size as u64;
            if end > len {
                return Err(format!("{}: {}: its local header and data end at {}, past the file's {} bytes", path, zip.name(e), end, len));
            }
        }
        Ok(zip)
    }

    /// The size and modification time of the file this archive was opened from, read through its
    /// handle: the opened file's, whatever stands at its path now.
    pub fn stamp(&self) -> Option<(u64, Option<std::time::SystemTime>)> {
        self.file.metadata().ok().map(|m| (m.len(), m.modified().ok()))
    }

    /// The sha1 of the file this archive was opened from, its bytes read through its handle.
    pub fn sha1(&self) -> Result<String, String> {
        let mut sha1 = crate::task::fetch::Sha1::new();
        let mut at = 0u64;
        loop {
            let chunk = read_at(&self.file, at, 1 << 16)?;
            if chunk.is_empty() {
                break;
            }
            sha1.update(&chunk);
            at += chunk.len() as u64;
        }
        Ok(sha1.hex())
    }

    /// The index of the entry of that name, if any.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| self.name(e) == name)
    }

    /// The entry's extra field in the central directory, and its comment.
    pub fn extra(&self, e: &Entry) -> &[u8] {
        let at = e.name_start as usize + e.name_len as usize;
        &self.directory[at..at + e.extra_len as usize]
    }

    pub fn comment(&self, e: &Entry) -> &[u8] {
        let at = e.name_start as usize + e.name_len as usize + e.extra_len as usize;
        &self.directory[at..at + e.comment_len as usize]
    }

    pub fn name(&self, e: &Entry) -> &str {
        let bytes = &self.directory[e.name_start as usize..e.name_start as usize + e.name_len as usize];
        std::str::from_utf8(bytes).unwrap_or("")
    }

    /// The uncompressed data of the entry, checked against its CRC-32.
    pub fn read(&self, index: usize) -> Result<Vec<u8>, String> {
        let e = &self.entries[index];
        // The local header and the data lie within the file, checked before a read is allocated for the
        // size the directory declares.
        if e.header_offset as u64 + LOCAL_HEADER_SIZE as u64 + e.compressed_size as u64 > self.len {
            return Err(format!("{}: truncated entry", self.name(e)));
        }
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
            // The size declared reserved up to the file's length, the output growing past it only as the
            // stream gives it.
            8 => inflate(&raw[data_start..data_end], (e.size as u64).min(self.len) as usize).map_err(|m| format!("{}: {}", self.name(e), m))?,
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
    crc32_update(0, data)
}

/// The CRC-32 of the bytes that gave `crc` followed by `data`.
pub fn crc32_update(crc: u32, data: &[u8]) -> u32 {
    let t = crc_tables();
    let mut crc = !crc;
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
    inflate_counted(data, size_hint).map(|(out, _)| out)
}

/// `inflate`, and how many bytes of `data` the stream took: what follows it (a gzip member's
/// trailer, the next entry of a zip stream) starts there.
pub fn inflate_counted(data: &[u8], size_hint: usize) -> Result<(Vec<u8>, usize), &'static str> {
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
            let used = bits.pos - (bits.n / 8) as usize;
            return Ok((out, used));
        }
    }
}

/// A DEFLATE stream inflated as its input comes, `java.util.zip.Inflater` under `teq interp`: a
/// run decodes what the input holds and stops where it ends, before a block's header, inside a
/// stored block or before a symbol, and the next run goes on from there, so that input fed in
/// pieces is read once. zlib's wrapper (its header, the Adler-32 after the data) unless `raw`. The
/// messages of a malformed stream are zlib's.
pub struct Inflating {
    raw: bool,
    input: Vec<u8>,
    /// The reader's place: the next byte of `input`, and the bits taken in ahead of it.
    pos: usize,
    buf: u64,
    n: u32,
    state: InflateState,
    last: bool,
    /// The last 32 KiB written, which a distance reaches back into.
    window: Vec<u8>,
    adler: u32,
    /// Bytes given in all, and since the last run.
    fed: u64,
    fresh: u64,
}

enum InflateState {
    Wrapper,
    Block,
    Stored(usize),
    Fixed,
    Dynamic(Box<(Huffman, Huffman)>),
    Trailer,
    Done,
}

const TRUNCATED: &str = "unexpected end of deflate stream";

fn zlib_message(m: &'static str) -> &'static str {
    match m {
        "stored block length mismatch" => "invalid stored block lengths",
        "distance too far back" => "invalid distance too far back",
        "invalid length symbol" | "invalid Huffman code" => "invalid literal/length code",
        "invalid distance symbol" => "invalid distance code",
        "too many length or distance codes" => "too many length or distance symbols",
        "repeat without a previous length" | "too many code lengths" => "invalid bit length repeat",
        "no end-of-block code" => "invalid code -- missing end-of-block",
        "over-subscribed Huffman code" => "invalid code lengths set",
        m => m,
    }
}

fn adler_update(adler: u32, data: &[u8]) -> u32 {
    let (mut a, mut b) = (adler & 0xffff, adler >> 16);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

impl Inflating {
    pub fn new(raw: bool) -> Inflating {
        Inflating { raw, input: Vec::new(), pos: 0, buf: 0, n: 0, state: if raw { InflateState::Block } else { InflateState::Wrapper }, last: false, window: Vec::new(), adler: 1, fed: 0, fresh: 0 }
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.input.extend_from_slice(data);
        self.fed += data.len() as u64;
        self.fresh += data.len() as u64;
    }

    pub fn finished(&self) -> bool {
        matches!(self.state, InflateState::Done)
    }

    /// The input not taken: past the stream's end once it is finished, else what came since the
    /// last run.
    pub fn remaining(&self) -> u64 {
        if self.finished() {
            (self.input.len() - (self.pos - (self.n / 8) as usize)) as u64
        } else {
            self.fresh
        }
    }

    pub fn fed(&self) -> u64 {
        self.fed
    }

    pub fn reset(&mut self) {
        *self = Inflating::new(self.raw);
    }

    /// Decodes what the input holds: the bytes written, or zlib's message for a malformed stream.
    pub fn run(&mut self) -> Result<Vec<u8>, &'static str> {
        self.fresh = 0;
        let start = self.window.len();
        let mut hist = std::mem::take(&mut self.window);
        let mut bits = Bits { data: &self.input, pos: self.pos, buf: self.buf, n: self.n };
        let outcome: Result<(), &'static str> = loop {
            let saved = (bits.pos, bits.buf, bits.n);
            let restore = |bits: &mut Bits| (bits.pos, bits.buf, bits.n) = saved;
            match &mut self.state {
                InflateState::Done => break Ok(()),
                InflateState::Wrapper => match (bits.take(8), bits.take(8)) {
                    (Ok(cmf), Ok(flg)) => {
                        if (cmf * 256 + flg) % 31 != 0 {
                            break Err("incorrect header check");
                        }
                        if cmf & 0x0f != 8 {
                            break Err("unknown compression method");
                        }
                        if flg & 0x20 != 0 {
                            break Err("need dictionary");
                        }
                        self.state = InflateState::Block;
                    }
                    _ => {
                        restore(&mut bits);
                        break Ok(());
                    }
                },
                InflateState::Block => {
                    let header = (|| -> Result<(bool, InflateState), &'static str> {
                        let last = bits.take(1)? == 1;
                        let state = match bits.take(2)? {
                            0 => {
                                let skip = bits.n % 8;
                                bits.take(skip)?;
                                let len = bits.take(16)? as usize;
                                let complement = bits.take(16)? as usize;
                                if len != !complement & 0xffff {
                                    return Err("stored block length mismatch");
                                }
                                InflateState::Stored(len)
                            }
                            1 => InflateState::Fixed,
                            2 => InflateState::Dynamic(Box::new(dynamic_codes(&mut bits)?)),
                            _ => return Err("invalid block type"),
                        };
                        Ok((last, state))
                    })();
                    match header {
                        Ok((last, state)) => {
                            self.last = last;
                            self.state = state;
                        }
                        Err(TRUNCATED) => {
                            restore(&mut bits);
                            break Ok(());
                        }
                        Err(m) => break Err(zlib_message(m)),
                    }
                }
                InflateState::Stored(left) => {
                    // The bit buffer holds whole bytes here: handed back before the copy.
                    let from = bits.pos - (bits.n / 8) as usize;
                    let k = (*left).min(self.input.len() - from);
                    hist.extend_from_slice(&self.input[from..from + k]);
                    bits = Bits { data: &self.input, pos: from + k, buf: 0, n: 0 };
                    *left -= k;
                    if *left > 0 {
                        break Ok(());
                    }
                    self.state = if !self.last { InflateState::Block } else if self.raw { InflateState::Done } else { InflateState::Trailer };
                }
                InflateState::Fixed | InflateState::Dynamic(_) => {
                    let (lit, dist) = match &self.state {
                        InflateState::Dynamic(codes) => (&codes.0, &codes.1),
                        _ => {
                            let f = fixed_codes();
                            (&f.0, &f.1)
                        }
                    };
                    // Each symbol whole or not at all: the input ending inside one is read again.
                    let symbols = loop {
                        let at = (bits.pos, bits.buf, bits.n);
                        let symbol = (|| -> Result<Option<usize>, &'static str> {
                            let sym = bits.decode(lit)? as usize;
                            if sym < 256 {
                                hist.push(sym as u8);
                                return Ok(Some(0));
                            }
                            if sym == 256 {
                                return Ok(None);
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
                            if distance > hist.len() {
                                return Err("distance too far back");
                            }
                            let from = hist.len() - distance;
                            if distance >= len {
                                hist.extend_from_within(from..from + len);
                            } else {
                                for k in 0..len {
                                    let b = hist[from + k];
                                    hist.push(b);
                                }
                            }
                            Ok(Some(0))
                        })();
                        match symbol {
                            Ok(Some(_)) => {}
                            Ok(None) => break Ok(true),
                            Err(TRUNCATED) => {
                                (bits.pos, bits.buf, bits.n) = at;
                                break Ok(false);
                            }
                            Err(m) => break Err(zlib_message(m)),
                        }
                    };
                    match symbols {
                        Ok(true) => self.state = if !self.last { InflateState::Block } else if self.raw { InflateState::Done } else { InflateState::Trailer },
                        Ok(false) => break Ok(()),
                        Err(m) => break Err(m),
                    }
                }
                InflateState::Trailer => {
                    let check = (|| -> Result<u32, &'static str> {
                        let skip = bits.n % 8;
                        bits.take(skip)?;
                        let mut v = 0u32;
                        for _ in 0..4 {
                            v = (v << 8) | bits.take(8)?;
                        }
                        Ok(v)
                    })();
                    match check {
                        Ok(v) => {
                            if v != adler_update(self.adler, &hist[start..]) {
                                break Err("incorrect data check");
                            }
                            self.state = InflateState::Done;
                        }
                        Err(_) => {
                            restore(&mut bits);
                            break Ok(());
                        }
                    }
                }
            }
        };
        let (pos, buf, n) = (bits.pos, bits.buf, bits.n);
        (self.pos, self.buf, self.n) = (pos, buf, n);
        let out = hist[start..].to_vec();
        self.adler = adler_update(self.adler, &out);
        let keep = hist.len().saturating_sub(32 * 1024);
        hist.drain(..keep);
        self.window = hist;
        // What every decoding is past goes, the reader's place moved with it.
        let used = self.pos - (self.n / 8) as usize;
        if used > 64 * 1024 && !self.finished() {
            self.input.drain(..self.pos);
            self.pos = 0;
        }
        outcome.map(|()| out)
    }
}

#[cfg(test)]
mod inflating_tests {
    use super::Inflating;

    /// A stream fed in pieces of every size from 1 byte, raw and with zlib's wrapper, at three
    /// levels, decodes to what it encodes, the bytes past its end left over.
    #[test]
    fn pieces_decode_as_the_whole() {
        let text: Vec<u8> = (0..60_000u32).map(|i| ((i * 7919) % 251) as u8 ^ if i % 3 == 0 { b'a' } else { 0 }).collect();
        for wrap in [false, true] {
            for level in [0, 1, 6, 9] {
                let mut packed = crate::deflate::deflate(&text, level, wrap);
                packed.extend_from_slice(b"TAIL");
                for piece in [1usize, 2, 3, 7, 512, 4096, 1 << 20] {
                    let mut inf = Inflating::new(!wrap);
                    let mut out = Vec::new();
                    for chunk in packed.chunks(piece) {
                        if inf.finished() {
                            inf.feed(chunk);
                            continue;
                        }
                        inf.feed(chunk);
                        out.extend(inf.run().unwrap());
                    }
                    assert!(inf.finished(), "level {level} wrap {wrap} piece {piece}");
                    assert_eq!(out, text, "level {level} wrap {wrap} piece {piece}");
                    assert_eq!(inf.remaining(), 4, "level {level} wrap {wrap} piece {piece}");
                }
            }
        }
    }

    #[test]
    fn a_bad_stream_is_zlibs_message() {
        let mut inf = Inflating::new(false);
        inf.feed(&[0x78, 0x9d]);
        assert_eq!(inf.run(), Err("incorrect header check"));
        let mut raw = Inflating::new(true);
        raw.feed(&[0x07]);
        assert_eq!(raw.run(), Err("invalid block type"));
    }
}

#[cfg(test)]
mod tests {
    /// CRC-32's check value (`java.util.zip.CRC32` of "123456789", bench/survey/oracle.scala), whole
    /// and continued across a split at every length the slicing by 8 takes apart.
    #[test]
    fn crc32_is_the_jdks() {
        assert_eq!(super::crc32(b"123456789"), 0xcbf4_3926);
        let data: Vec<u8> = (0..100u32).map(|i| (i * 31 % 256) as u8).collect();
        for cut in 0..=data.len() {
            assert_eq!(super::crc32_update(super::crc32(&data[..cut]), &data[cut..]), super::crc32(&data), "cut at {cut}");
        }
    }

    /// A central directory whose declared size ends inside an entry's name (the end record's size
    /// field set short): refused at the opening, never sliced past its end when a name is read.
    #[test]
    fn an_entry_past_its_directory_is_refused() {
        let dir = std::env::temp_dir().join(format!("teq-zip-directory-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut bytes = include_bytes!("../tests/lsp/libs/lib/liba-1.0-sources.jar").to_vec();
        let end = bytes.windows(4).rposition(|w| w == [0x50, 0x4b, 0x05, 0x06]).unwrap();
        bytes[end + 12..end + 16].copy_from_slice(&46u32.to_le_bytes());
        let (short, whole) = (dir.join("short.jar"), dir.join("whole.jar"));
        std::fs::write(&short, &bytes).unwrap();
        std::fs::write(&whole, include_bytes!("../tests/lsp/libs/lib/liba-1.0-sources.jar")).unwrap();
        let refused = super::Zip::open(short.to_str().unwrap()).err();
        let opened = super::Zip::open_valid(whole.to_str().unwrap()).map(|z| z.entries.len());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(refused.is_some_and(|e| e.contains("runs past the directory")));
        assert_eq!(opened, Ok(1));
    }

    /// An end record whose directory does not lie before it (its size 2 GB, or its offset at the file's
    /// end) or lists more entries than its size holds, and an entry whose data the directory places
    /// past the file or whose size it declares at 4 GB: each refused with the reader's message, nothing
    /// allocated for the size declared.
    #[test]
    fn sizes_declared_past_the_file_are_refused() {
        let dir = std::env::temp_dir().join(format!("teq-zip-declared-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let whole = include_bytes!("../tests/lsp/libs/lib/liba-1.0-sources.jar");
        let end = whole.windows(4).rposition(|w| w == [0x50, 0x4b, 0x05, 0x06]).unwrap();
        let entry = whole.windows(4).position(|w| w == [0x50, 0x4b, 0x01, 0x02]).unwrap();
        let damaged = |at: usize, bytes: &[u8]| {
            let mut copy = whole.to_vec();
            copy[at..at + bytes.len()].copy_from_slice(bytes);
            copy
        };
        let write = |name: &str, bytes: &[u8]| {
            let path = dir.join(format!("{name}.jar"));
            std::fs::write(&path, bytes).unwrap();
            path.to_str().unwrap().to_string()
        };
        let opened = |bytes: Vec<u8>, name: &str| super::Zip::open(&write(name, &bytes)).err().unwrap_or_default();
        let size = opened(damaged(end + 12, &0x8000_004bu32.to_le_bytes()), "size");
        let offset = opened(damaged(end + 16, &(whole.len() as u32).to_le_bytes()), "offset");
        let count = opened(damaged(end + 10, &1000u16.to_le_bytes()), "count");
        let read = |bytes: Vec<u8>, name: &str| super::Zip::open(&write(name, &bytes)).map(|z| z.read(0).err().unwrap_or_default());
        let compressed = read(damaged(entry + 20, &0x7fff_0000u32.to_le_bytes()), "compressed");
        let inflated = read(damaged(entry + 24, &0xffff_fff0u32.to_le_bytes()), "inflated");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(size.contains("central directory of 2147483723 bytes at") && size.contains("runs past its end record"), "{size}");
        assert!(offset.contains("runs past its end record"), "{offset}");
        assert!(count.contains("lists 1000 entries in") && count.contains("fewer than the 46 each takes"), "{count}");
        assert!(compressed.as_ref().is_ok_and(|e| e.ends_with(": truncated entry")), "{compressed:?}");
        assert!(inflated.as_ref().is_ok_and(|e| e.ends_with(": CRC-32 or size mismatch")), "{inflated:?}");
    }

    /// An archive whose directory reads but whose entry does not lie where it says: a local header's
    /// offset past the file, a local extra field's length (byte 28 of the header) running past it, a
    /// local header's signature overwritten. Each opens, and each is refused as no whole archive.
    #[test]
    fn an_entry_past_the_file_is_no_whole_archive() {
        let dir = std::env::temp_dir().join(format!("teq-zip-local-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let whole = include_bytes!("../tests/lsp/libs/lib/liba-1.0-sources.jar");
        let entry = whole.windows(4).position(|w| w == [0x50, 0x4b, 0x01, 0x02]).unwrap();
        let damaged = |at: usize, bytes: &[u8]| {
            let mut copy = whole.to_vec();
            copy[at..at + bytes.len()].copy_from_slice(bytes);
            copy
        };
        let cases = [
            ("offset", damaged(entry + 42, &(whole.len() as u32 + 100).to_le_bytes())),
            ("extra", damaged(28, &65535u16.to_le_bytes())),
            ("signature", damaged(0, b"PK\x09\x09")),
        ];
        let mut verdicts = Vec::new();
        for (name, bytes) in &cases {
            let path = dir.join(format!("{name}.jar"));
            std::fs::write(&path, bytes).unwrap();
            let path = path.to_str().unwrap().to_string();
            verdicts.push((*name, super::Zip::open(&path).is_ok(), super::Zip::open_valid(&path).is_ok()));
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(verdicts, [("offset", true, false), ("extra", true, false), ("signature", true, false)]);
    }

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
