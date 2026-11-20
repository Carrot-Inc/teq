//! The JDK's runtime image, `<home>/lib/modules` (the jimage format of JDK 9 and later), read for the class files
//! `ct.sym` strips: their `Code`, their static initializers and their private methods, which the interpreter's proof
//! of an empty construction reads (`classfile::ctor`). The home is the one typing opened `ct.sym` in.
//!
//! The format (JDK 24's `jdk.internal.jimage.BasicImageReader`, `ImageHeader`, `ImageLocation`,
//! `ImageStringsReader`): a header of seven 32-bit words in the image's own byte order (`jlink --endian` writes
//! either; the magic 0xCAFEDADA tells which), then the redirect table and the offsets table (a word per location
//! each), the locations (attribute streams) and the strings (modified UTF-8, NUL-terminated); the resources follow
//! this index. A name is found as `getLocationIndex` finds it: its hash picks a redirect entry, which is either the
//! location's index (negative, as `-index - 1`) or the seed of a second hash that gives it; the location's own name
//! is then compared, since a name the image lacks hashes somewhere too. A location's attributes, each a kind byte
//! (`kind << 3 | length - 1`) and a big-endian value: the module, the parent (the package path), the base (the
//! simple name), the extension, the resource's offset, its compressed size and its uncompressed size. A compressed
//! resource (`jlink --compress`) has its own framing and transformations, which this reader does not undo: it is
//! reported as unread, and what needs it stays unproven.
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const MAGIC: u32 = 0xCAFE_DADA;
const HEADER_SIZE: u64 = 7 * 4;
const HASH_MULTIPLIER: u32 = 0x0100_0193;
const POSITIVE_MASK: u32 = 0x7FFF_FFFF;
const ATTRIBUTE_END: u8 = 0;
const ATTRIBUTE_MODULE: usize = 1;
const ATTRIBUTE_PARENT: usize = 2;
const ATTRIBUTE_BASE: usize = 3;
const ATTRIBUTE_EXTENSION: usize = 4;
const ATTRIBUTE_OFFSET: usize = 5;
const ATTRIBUTE_COMPRESSED: usize = 6;
const ATTRIBUTE_UNCOMPRESSED: usize = 7;
const ATTRIBUTE_COUNT: usize = 8;

/// What an image is read from: the file, or bytes in memory for the tests.
pub trait Source: Read + Seek + Send {}
impl<T: Read + Seek + Send> Source for T {}

/// An opened image: its header checked, everything else read where a lookup needs it.
pub struct Image {
    src: Box<dyn Source>,
    big_endian: bool,
    table_length: u32,
    redirect_at: u64,
    offsets_at: u64,
    locations_at: u64,
    locations_size: u64,
    strings_at: u64,
    strings_size: u64,
    index_size: u64,
    len: u64,
}

/// Why a class's bytes are not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unread {
    /// The image does not hold the name.
    Missing,
    /// The resource is compressed.
    Compressed,
    /// The image could not be read where the lookup went: what failed.
    Image(String),
}

impl std::fmt::Display for Unread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unread::Missing => write!(f, "the runtime image has no such class"),
            Unread::Compressed => write!(f, "its class file is compressed in the runtime image"),
            Unread::Image(why) => write!(f, "the runtime image cannot be read: {}", why),
        }
    }
}

/// The modified UTF-8 of `s` (JVMS 4.4.7), which the image's strings and its hash are made of: each UTF-16 unit
/// alone, NUL as two bytes.
fn encode_modified_utf8(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for u in s.encode_utf16() {
        match u {
            0 => out.extend_from_slice(&[0xC0, 0x80]),
            1..=0x7F => out.push(u as u8),
            0x80..=0x7FF => out.extend_from_slice(&[0xC0 | (u >> 6) as u8, 0x80 | (u & 0x3F) as u8]),
            _ => out.extend_from_slice(&[0xE0 | (u >> 12) as u8, 0x80 | ((u >> 6) & 0x3F) as u8, 0x80 | (u & 0x3F) as u8]),
        }
    }
    out
}

/// `ImageStringsReader.unmaskedHashCode` over a name's modified UTF-8, masked positive.
fn hash(bytes: &[u8], seed: u32) -> u32 {
    bytes.iter().fold(seed, |h, &b| h.wrapping_mul(HASH_MULTIPLIER) ^ b as u32) & POSITIVE_MASK
}

impl Image {
    /// The image of the JDK at `home`.
    pub fn open(home: &Path) -> Result<Image, String> {
        let path = home.join("lib").join("modules");
        let file = std::fs::File::open(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        Image::read(Box::new(file)).map_err(|e| format!("{}: {}", path.display(), e))
    }

    /// An image read from `src`: the magic in either byte order and the version 1.0 checked, and the tables'
    /// sizes against the source's length, before any count is used.
    pub fn read(mut src: Box<dyn Source>) -> Result<Image, String> {
        let len = src.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        if len < HEADER_SIZE {
            return Err("not a runtime image (shorter than its header)".to_string());
        }
        let mut head = [0u8; HEADER_SIZE as usize];
        src.seek(SeekFrom::Start(0)).and_then(|_| src.read_exact(&mut head)).map_err(|e| e.to_string())?;
        let big_endian = match u32::from_le_bytes([head[0], head[1], head[2], head[3]]) {
            MAGIC => false,
            m if m.swap_bytes() == MAGIC => true,
            _ => return Err("not a runtime image (its magic is not 0xCAFEDADA in either byte order)".to_string()),
        };
        let word = |i: usize| {
            let b = [head[i * 4], head[i * 4 + 1], head[i * 4 + 2], head[i * 4 + 3]];
            if big_endian {
                u32::from_be_bytes(b)
            } else {
                u32::from_le_bytes(b)
            }
        };
        let (major, minor) = (word(1) >> 16, word(1) & 0xFFFF);
        if (major, minor) != (1, 0) {
            return Err(format!("a runtime image of version {}.{} (this reader reads 1.0)", major, minor));
        }
        let table_length = word(4);
        let (locations_size, strings_size) = (word(5) as u64, word(6) as u64);
        let redirect_at = HEADER_SIZE;
        let offsets_at = redirect_at + table_length as u64 * 4;
        let locations_at = offsets_at + table_length as u64 * 4;
        let strings_at = locations_at + locations_size;
        let index_size = strings_at + strings_size;
        if index_size > len {
            return Err(format!("its index of {} bytes is longer than the file's {}", index_size, len));
        }
        Ok(Image { src, big_endian, table_length, redirect_at, offsets_at, locations_at, locations_size, strings_at, strings_size, index_size, len })
    }

    /// Whether the image is written big-endian.
    #[cfg(test)]
    fn big_endian(&self) -> bool {
        self.big_endian
    }

    fn bytes_at(&mut self, at: u64, n: usize) -> Result<Vec<u8>, Unread> {
        if at.checked_add(n as u64).map_or(true, |end| end > self.len) {
            return Err(Unread::Image(format!("{} bytes at {} are past its end", n, at)));
        }
        let mut out = vec![0u8; n];
        self.src.seek(SeekFrom::Start(at)).and_then(|_| self.src.read_exact(&mut out)).map_err(|e| Unread::Image(e.to_string()))?;
        Ok(out)
    }

    fn word_at(&mut self, at: u64) -> Result<u32, Unread> {
        let b = self.bytes_at(at, 4)?;
        let b = [b[0], b[1], b[2], b[3]];
        Ok(if self.big_endian { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    }

    /// The string at `offset` of the strings table, as its raw modified UTF-8.
    fn string(&mut self, offset: u64) -> Result<Vec<u8>, Unread> {
        if offset >= self.strings_size {
            return Err(Unread::Image(format!("a string offset {} past the strings table", offset)));
        }
        let mut out = Vec::new();
        let mut at = self.strings_at + offset;
        let end = self.strings_at + self.strings_size;
        while at < end {
            let chunk = self.bytes_at(at, (end - at).min(256) as usize)?;
            if let Some(nul) = chunk.iter().position(|&b| b == 0) {
                out.extend_from_slice(&chunk[..nul]);
                return Ok(out);
            }
            at += chunk.len() as u64;
            out.extend_from_slice(&chunk);
        }
        Err(Unread::Image("an unterminated string".to_string()))
    }

    /// The attributes of the location at `offset` of the locations table, by kind.
    fn attributes(&mut self, offset: u64) -> Result<[u64; ATTRIBUTE_COUNT], Unread> {
        if offset >= self.locations_size {
            return Err(Unread::Image(format!("a location offset {} past the locations table", offset)));
        }
        // At most one attribute of each kind, eight bytes of value each, and the end.
        let n = (self.locations_size - offset).min((ATTRIBUTE_COUNT * 9 + 1) as u64) as usize;
        let stream = self.bytes_at(self.locations_at + offset, n)?;
        let mut out = [0u64; ATTRIBUTE_COUNT];
        let mut i = 0;
        while let Some(&b) = stream.get(i) {
            let kind = (b >> 3) as usize;
            if kind == ATTRIBUTE_END as usize {
                return Ok(out);
            }
            let len = (b & 7) as usize + 1;
            let value = stream.get(i + 1..i + 1 + len).ok_or_else(|| Unread::Image("a truncated location".to_string()))?;
            if kind >= ATTRIBUTE_COUNT {
                return Err(Unread::Image(format!("a location attribute of kind {}", kind)));
            }
            out[kind] = value.iter().fold(0u64, |v, &x| (v << 8) | x as u64);
            i += 1 + len;
        }
        Err(Unread::Image("a location without its end".to_string()))
    }

    /// The location of `name` (`/java.base/java/lang/Object.class`), by `BasicImageReader.getLocationIndex` and
    /// `ImageLocation.verify`.
    fn location(&mut self, name: &[u8]) -> Result<[u64; ATTRIBUTE_COUNT], Unread> {
        let count = self.table_length;
        if count == 0 {
            return Err(Unread::Missing);
        }
        let redirect = self.word_at(self.redirect_at + (hash(name, HASH_MULTIPLIER) % count) as u64 * 4)? as i32;
        let index = match redirect {
            0 => return Err(Unread::Missing),
            r if r < 0 => (-(r as i64) - 1) as u64,
            r => (hash(name, r as u32) % count) as u64,
        };
        if index >= count as u64 {
            return Err(Unread::Image(format!("a redirect to location {} of {}", index, count)));
        }
        let offset = self.word_at(self.offsets_at + index * 4)? as u64;
        let a = self.attributes(offset)?;
        let mut full = Vec::with_capacity(name.len());
        let module = self.string(a[ATTRIBUTE_MODULE])?;
        if !module.is_empty() {
            full.push(b'/');
            full.extend_from_slice(&module);
            full.push(b'/');
        }
        let parent = self.string(a[ATTRIBUTE_PARENT])?;
        if !parent.is_empty() {
            full.extend_from_slice(&parent);
            full.push(b'/');
        }
        full.extend_from_slice(&self.string(a[ATTRIBUTE_BASE])?);
        let extension = self.string(a[ATTRIBUTE_EXTENSION])?;
        if !extension.is_empty() {
            full.push(b'.');
            full.extend_from_slice(&extension);
        }
        if full != name {
            return Err(Unread::Missing);
        }
        Ok(a)
    }

    /// The resource `name`, uncompressed.
    pub fn resource(&mut self, name: &str) -> Result<Vec<u8>, Unread> {
        let a = self.location(&encode_modified_utf8(name))?;
        if a[ATTRIBUTE_COMPRESSED] != 0 {
            return Err(Unread::Compressed);
        }
        let size = usize::try_from(a[ATTRIBUTE_UNCOMPRESSED]).map_err(|_| Unread::Image("a resource too large".to_string()))?;
        let at = self.index_size.checked_add(a[ATTRIBUTE_OFFSET]).ok_or_else(|| Unread::Image("a resource offset past its end".to_string()))?;
        self.bytes_at(at, size)
    }

    /// The class file of `internal` (`java/util/AbstractList`) in `module` (`java.base`).
    #[cfg(test)]
    pub(crate) fn class_bytes(&mut self, module: &str, internal: &str) -> Result<Vec<u8>, Unread> {
        self.resource(&format!("/{}/{}.class", module, internal))
    }
}

/// The image of the process, of the one home asked for: opened at the first proof that needs it, never before, and
/// kept, its opening's failure too.
static SHARED: std::sync::Mutex<Option<(std::path::PathBuf, Result<Image, String>)>> = std::sync::Mutex::new(None);

/// `f` on the runtime image of the JDK at `home`, which the process opens once.
pub fn with_shared<T>(home: &Path, f: impl FnOnce(&mut Image) -> T) -> Result<T, String> {
    let mut shared = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    if shared.as_ref().map_or(true, |(h, _)| h != home) {
        *shared = Some((home.to_path_buf(), Image::open(home)));
    }
    match shared.as_mut().map(|(_, img)| img) {
        Some(Ok(img)) => Ok(f(img)),
        Some(Err(e)) => Err(e.clone()),
        None => unreachable!(),
    }
}

/// Writes an image of `resources` (`(module, parent, base, extension, bytes, compressed)`) in either byte order, its
/// redirect table made as `jdk.tools.jlink.internal.PerfectHashBuilder` makes it: the buckets of more than one name
/// each given the first seed that sends their names to free slots, then each single name its slot directly.
#[cfg(test)]
pub(crate) fn write_image(resources: &[(&str, &str, &str, &str, &[u8], bool)], big_endian: bool) -> Vec<u8> {
    let n = resources.len() as u32;
    let mut strings: Vec<u8> = vec![0];
    let mut string = |s: &str| -> u64 {
        if s.is_empty() {
            return 0;
        }
        let at = strings.len() as u64;
        strings.extend_from_slice(&encode_modified_utf8(s));
        strings.push(0);
        at
    };
    let mut locations: Vec<u8> = Vec::new();
    let mut offsets = Vec::new();
    let mut names = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    for &(module, parent, base, extension, bytes, compressed) in resources {
        offsets.push(locations.len() as u32);
        let mut full = String::new();
        if !module.is_empty() {
            full.push_str(&format!("/{}/", module));
        }
        if !parent.is_empty() {
            full.push_str(&format!("{}/", parent));
        }
        full.push_str(base);
        if !extension.is_empty() {
            full.push_str(&format!(".{}", extension));
        }
        names.push(encode_modified_utf8(&full));
        let values = [
            (ATTRIBUTE_MODULE, string(module)),
            (ATTRIBUTE_PARENT, string(parent)),
            (ATTRIBUTE_BASE, string(base)),
            (ATTRIBUTE_EXTENSION, string(extension)),
            (ATTRIBUTE_OFFSET, data.len() as u64),
            (ATTRIBUTE_COMPRESSED, if compressed { bytes.len() as u64 } else { 0 }),
            (ATTRIBUTE_UNCOMPRESSED, bytes.len() as u64),
        ];
        for (kind, v) in values {
            if v == 0 {
                continue;
            }
            let len = (8 - v.leading_zeros() as usize / 8).max(1);
            locations.push(((kind as u8) << 3) | (len as u8 - 1));
            locations.extend_from_slice(&v.to_be_bytes()[8 - len..]);
        }
        locations.push(ATTRIBUTE_END);
        data.extend_from_slice(bytes);
    }
    let mut redirect = vec![0i32; n as usize];
    let mut slots = vec![None::<usize>; n as usize];
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); n as usize];
    for (i, name) in names.iter().enumerate() {
        buckets[(hash(name, HASH_MULTIPLIER) % n) as usize].push(i);
    }
    let mut order: Vec<usize> = (0..n as usize).collect();
    order.sort_by_key(|&b| std::cmp::Reverse(buckets[b].len()));
    for &b in order.iter().filter(|&&b| buckets[b].len() > 1) {
        let seed = (1..).find(|&s| {
            let mut taken: Vec<u32> = buckets[b].iter().map(|&i| hash(&names[i], s) % n).collect();
            let free = taken.iter().all(|&t| slots[t as usize].is_none());
            taken.sort_unstable();
            taken.dedup();
            free && taken.len() == buckets[b].len()
        });
        let seed = seed.unwrap();
        for &i in &buckets[b] {
            slots[(hash(&names[i], seed) % n) as usize] = Some(i);
        }
        redirect[b] = seed as i32;
    }
    for &b in order.iter().filter(|&&b| buckets[b].len() == 1) {
        let slot = slots.iter().position(|s| s.is_none()).unwrap();
        slots[slot] = Some(buckets[b][0]);
        redirect[b] = -(slot as i32) - 1;
    }
    let word = |w: u32| if big_endian { w.to_be_bytes() } else { w.to_le_bytes() };
    let mut out = Vec::new();
    for w in [MAGIC, 1 << 16, 0, n, n, locations.len() as u32, strings.len() as u32] {
        out.extend_from_slice(&word(w));
    }
    for r in &redirect {
        out.extend_from_slice(&word(*r as u32));
    }
    for s in &slots {
        out.extend_from_slice(&word(offsets[s.unwrap()]));
    }
    out.extend_from_slice(&locations);
    out.extend_from_slice(&strings);
    out.extend_from_slice(&data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn image(bytes: Vec<u8>) -> Result<Image, String> {
        Image::read(Box::new(Cursor::new(bytes)))
    }

    const RESOURCES: &[(&str, &str, &str, &str, &[u8], bool)] = &[
        ("mod.a", "pkg/one", "Alpha", "class", b"alpha bytes", false),
        ("mod.a", "pkg/one", "Beta", "class", b"beta", false),
        ("mod.b", "pkg/two", "Gamma", "class", b"gamma gamma", true),
        ("mod.b", "pkg/two", "Delta", "class", b"d", false),
        ("mod.b", "pkg/tw\u{e9}", "Eps\u{0}ilon", "class", b"accented", false),
        ("", "", "packages", "", b"top", false),
    ];

    #[test]
    fn resources_in_either_byte_order() {
        for big in [false, true] {
            let mut img = image(write_image(RESOURCES, big)).unwrap();
            assert_eq!(img.big_endian(), big);
            assert_eq!(img.class_bytes("mod.a", "pkg/one/Alpha").unwrap(), b"alpha bytes");
            assert_eq!(img.class_bytes("mod.a", "pkg/one/Beta").unwrap(), b"beta");
            assert_eq!(img.class_bytes("mod.b", "pkg/two/Delta").unwrap(), b"d");
            assert_eq!(img.class_bytes("mod.b", "pkg/tw\u{e9}/Eps\u{0}ilon").unwrap(), b"accented");
            assert_eq!(img.resource("packages").unwrap(), b"top");
            assert_eq!(img.class_bytes("mod.b", "pkg/two/Gamma"), Err(Unread::Compressed));
            assert_eq!(img.class_bytes("mod.a", "pkg/one/Zeta"), Err(Unread::Missing));
            assert_eq!(img.class_bytes("mod.b", "pkg/one/Alpha"), Err(Unread::Missing));
        }
    }

    #[test]
    fn a_big_endian_header_gives_its_own_counts() {
        // jlink's `--endian big`: the words a little-endian reading takes for a table of 1,444,872,192 locations
        // are read as the 8,022 they are.
        let bytes = write_image(RESOURCES, true);
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xDA, 0xDA]); // the jdk's ImageHeader magic, big-endian
        let img = image(bytes).unwrap();
        assert_eq!(img.table_length, RESOURCES.len() as u32);
        let mut foreign = write_image(RESOURCES, false);
        foreign[..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // not the jdk's ImageHeader magic
        assert!(image(foreign).err().unwrap().contains("magic"));
    }

    #[test]
    fn a_foreign_version_or_an_index_past_the_end_is_refused() {
        let mut bytes = write_image(RESOURCES, false);
        bytes[4..8].copy_from_slice(&(2u32 << 16).to_le_bytes()); // the jdk's ImageHeader version word: 2.0
        assert!(image(bytes).err().unwrap().contains("version 2.0"));
        let mut bytes = write_image(RESOURCES, false);
        bytes[24..28].copy_from_slice(&u32::MAX.to_le_bytes()); // the jdk's ImageHeader strings size word
        assert!(image(bytes).err().unwrap().contains("longer than the file"));
        assert!(image(vec![0xCA, 0xFE]).is_err());
    }

    #[test]
    fn the_jdks_own_image() {
        let home = crate::classfile::jdk::find_home().expect("the unit tests read the JDK's runtime image: set JAVA_HOME or put java on the path");
        let mut img = Image::open(&home).unwrap();
        for class in ["java/lang/Object", "java/util/AbstractList", "java/util/HashMap"] {
            let bytes = img.class_bytes("java.base", class).unwrap();
            assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "{}", class); // the jdk's class file magic
        }
        assert_eq!(img.class_bytes("java.base", "java/util/NoSuchClassHere"), Err(Unread::Missing));
        assert_eq!(img.class_bytes("java.desktop", "java/util/HashMap"), Err(Unread::Missing));
    }
}
