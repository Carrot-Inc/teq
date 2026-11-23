//! A jar of the class path as Scala Native 0.5.12's build reads it, through JDK 24's two readers in their order
//! (tools/src/main/scala/scala/scalanative/build/NativeLib.scala): `findNativeLibs` opens every jar as a
//! java.util.zip.ZipFile and takes it as a library where an entry's name is a native file's (`IO.existsInJar`,
//! build/IO.scala 135-144); `unpackNativeCode` copies the whole of a library out through the JDK's zip file system
//! (`IO.unzip` and `copyRecursive`, build/IO.scala 226-276). Ported here from the file's bytes, each reader's finding of
//! the central directory, its reading of every entry, the zip file system's keys of the names, and the copy's reading
//! of an entry's data; and refused, before anything is unpacked, what the two readers or teq's class path read would
//! take differently, and what the copy cannot write as the jar holds it.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use super::inflate::Stop;

type Res<T> = Result<T, String>;

/// The bytes of a jar: its length, and what it holds at a place.
pub trait Bytes {
    fn len(&self) -> u64;
    /// Up to `len` bytes at `pos`: fewer where the file ends, none from a place past it.
    fn at(&self, pos: u64, len: usize) -> Vec<u8>;
}

impl Bytes for Vec<u8> {
    fn len(&self) -> u64 {
        self.as_slice().len() as u64
    }
    fn at(&self, pos: u64, len: usize) -> Vec<u8> {
        let start = pos.min(self.as_slice().len() as u64) as usize;
        self[start..(start + len).min(self.as_slice().len())].to_vec()
    }
}

/// A jar's file, read where asked.
pub struct JarFile {
    file: std::fs::File,
    len: u64,
}

impl JarFile {
    pub fn open(path: &Path) -> Res<JarFile> {
        let file = std::fs::File::open(path).map_err(|e| format!("cannot open {}: {}", path.display(), e))?;
        let len = file.metadata().map_err(|e| format!("{}: {}", path.display(), e))?.len();
        Ok(JarFile { file, len })
    }
}

impl Bytes for JarFile {
    fn len(&self) -> u64 {
        self.len
    }
    fn at(&self, pos: u64, len: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let mut file = &self.file;
        if pos >= self.len || file.seek(SeekFrom::Start(pos)).is_err() {
            return out;
        }
        let _ = file.take(len as u64).read_to_end(&mut out);
        out
    }
}

/// The little-endian number of `width` bytes at `at`.
fn little(bytes: &[u8], at: usize, width: usize) -> u64 {
    (0..width).fold(0, |value, i| value | (bytes[at + i] as u64) << (8 * i))
}

const ENDSIG: u64 = 0x0605_4b50;
const CENSIG: u64 = 0x0201_4b50;
const LOCSIG: u64 = 0x0403_4b50;
const ZIP64_ENDSIG: u64 = 0x0606_4b50;
const ZIP64_LOCSIG: u64 = 0x0706_4b50;
/// The fixed bytes of an end record, a central directory header, a local header, a zip64 locator and a zip64 end
/// record (the JDK's `ENDHDR`, `CENHDR`, `LOCHDR`, `ZIP64_LOCHDR`, `ZIP64_ENDHDR`).
const ENDHDR: i64 = 22;
const CENHDR: usize = 46;
const LOCHDR: usize = 30;
const ZIP64_LOCHDR: i64 = 20;
const ZIP64_ENDHDR: usize = 56;
/// How far back from the file's end the readers look for an end record, and the blocks they look in (`END_MAXLEN`,
/// `READBLOCKSZ`).
const END_MAXLEN: i64 = 0xFFFF + ENDHDR;
const READBLOCKSZ: i64 = 128;
/// The value a size or an offset holds where its zip64 field holds it, and the count's and the disk number's
/// (`ZIP64_MAGICVAL`, `ZIP64_MAGICCOUNT`); the zip64 extra block's tag (`EXTID_ZIP64`).
const ZIP64_FIELD: u64 = 0xFFFF_FFFF;
const ZIP64_COUNT: u64 = 0xFFFF;
const ZIP64_TAG: u64 = 0x0001;
/// ZipFile's bound of a central directory's size (`MAX_CEN_SIZE`, jdk.internal.util.ArraysSupport's
/// `SOFT_MAX_ARRAY_LENGTH`).
const MAX_CEN_SIZE: i64 = i32::MAX as i64 - 8;

/// The source extensions Scala Native compiles (`LLVM.srcExtensions`).
pub const SOURCE_EXTENSIONS: [&str; 3] = [".c", ".cpp", ".S"];
/// The directory of a library's C code.
pub const CODE_DIR: &str = "scala-native";

/// One of the two readers.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Reader {
    /// java.util.zip.ZipFile, which `findNativeLibs` opens every jar with.
    ZipFile,
    /// The JDK's zip file system, which `unpackNativeCode` copies a library out through.
    ZipFs,
}

/// An end record as a reader takes it: where it lies (the zip64 end record's place where the reader takes one), the
/// directory's size and offset and the count of entries.
#[derive(Clone, Copy, PartialEq, Debug)]
struct End {
    endpos: i64,
    cenlen: i64,
    cenoff: i64,
    centot: i64,
}

/// The four bytes at `pos` as a signature, where the file holds them.
fn signature(jar: &dyn Bytes, pos: i64) -> Option<u64> {
    let bytes = jar.at(pos as u64, 4);
    (bytes.len() == 4).then(|| little(&bytes, 0, 4))
}

/// The end record a reader finds, scanning back from the file's end in blocks of 128 bytes, those before the file's
/// start taken as NUL bytes (ZipFile.java `Source.findEND` 1561-1647; ZipFileSystem.java `findEND` 1254-1306): the
/// first signature whose comment reaches the file's end; ZipFile takes besides one whose directory and first local
/// header have their signatures where its sizes put them (1584-1599), and refuses one whose comment runs past the
/// file (1600-1605); then the zip64 locator in the 20 bytes before it (`zip64_end`).
fn find_end(jar: &dyn Bytes, reader: Reader) -> Res<End> {
    let ziplen = jar.len() as i64;
    if reader == Reader::ZipFile && ziplen <= 0 {
        return Err("the file is empty".to_string());
    }
    let min_hdr = (ziplen - END_MAXLEN).max(0);
    let min_pos = min_hdr - (READBLOCKSZ - ENDHDR);
    let mut pos = ziplen - READBLOCKSZ;
    while pos >= min_pos {
        let off = if pos < 0 { (-pos) as usize } else { 0 };
        let mut block = vec![0u8; READBLOCKSZ as usize];
        let read = jar.at((pos + off as i64) as u64, READBLOCKSZ as usize - off);
        if read.len() != READBLOCKSZ as usize - off {
            return Err("no end record of a central directory".to_string());
        }
        block[off..].copy_from_slice(&read);
        for i in (0..=(READBLOCKSZ - ENDHDR) as usize).rev() {
            if little(&block, i, 4) != ENDSIG {
                continue;
            }
            let end = End {
                endpos: pos + i as i64,
                centot: little(&block, i + 10, 2) as i64,
                cenlen: little(&block, i + 12, 4) as i64,
                cenoff: little(&block, i + 16, 4) as i64,
            };
            let comlen = little(&block, i + 20, 2) as i64;
            if end.endpos + ENDHDR + comlen != ziplen {
                if reader == Reader::ZipFs {
                    continue;
                }
                let cenpos = end.endpos - end.cenlen;
                let locpos = cenpos - end.cenoff;
                if cenpos < 0 || locpos < 0 || signature(jar, cenpos) != Some(CENSIG) || signature(jar, locpos) != Some(LOCSIG) {
                    continue;
                }
                if end.endpos + ENDHDR + comlen > ziplen {
                    return Err("its end record's comment runs past the file".to_string());
                }
            }
            return zip64_end(jar, end, reader);
        }
        pos -= READBLOCKSZ - ENDHDR;
    }
    Err("no end record of a central directory".to_string())
}

/// The zip64 end record a reader takes the end record's place from (ZipFile.java 1613-1645, ZipFileSystem.java
/// 1271-1301): a zip64 locator in the 20 bytes before the end record, a zip64 end record where it points, and of its
/// size, offset and count each the end record's own or the end record's field its mark. A locator's negative offset
/// ZipFile passes over (its seek's IOException, which its `try` takes) and the zip file system cannot seek to (an
/// IllegalArgumentException that opens nothing); the zip file system takes the count as an int.
fn zip64_end(jar: &dyn Bytes, end: End, reader: Reader) -> Res<End> {
    if end.endpos < ZIP64_LOCHDR {
        return Ok(end);
    }
    let locator = jar.at((end.endpos - ZIP64_LOCHDR) as u64, ZIP64_LOCHDR as usize);
    if locator.len() != ZIP64_LOCHDR as usize || little(&locator, 0, 4) != ZIP64_LOCSIG {
        return Ok(end);
    }
    let end64pos = little(&locator, 8, 8) as i64;
    if end64pos < 0 {
        return match reader {
            Reader::ZipFile => Ok(end),
            Reader::ZipFs => Err("its zip64 locator gives a negative offset, which the JDK's zip file system cannot seek to".to_string()),
        };
    }
    let record = jar.at(end64pos as u64, ZIP64_ENDHDR);
    if record.len() != ZIP64_ENDHDR || little(&record, 0, 4) != ZIP64_ENDSIG {
        return Ok(end);
    }
    let (centot, cenlen, cenoff) = (little(&record, 32, 8) as i64, little(&record, 40, 8) as i64, little(&record, 48, 8) as i64);
    if cenlen != end.cenlen && end.cenlen != ZIP64_FIELD as i64 || cenoff != end.cenoff && end.cenoff != ZIP64_FIELD as i64 || centot != end.centot && end.centot != ZIP64_COUNT as i64 {
        return Ok(end);
    }
    let centot = if reader == Reader::ZipFs { centot as i32 as i64 } else { centot };
    Ok(End { endpos: end64pos, cenlen, cenoff, centot })
}

/// Where a reader takes a jar's central directory to lie, by its end record (ZipFile.java `Source.initCEN` 1652-1690,
/// ZipFileSystem.java `initCEN` 1526-1548): none where the end record is the file's start (no entry); else its place
/// `endpos - cenlen`, its size, and the place the entries' offsets count from (`locpos`, past a stub before the jar),
/// each refused where it runs before the file's start; ZipFile refuses a size over its bound or a count more than the
/// size holds room for, and both a directory past the file's end (the zip file system reads 22 bytes beyond it, and
/// takes the size and the count as ints).
fn directory_place(jar: &dyn Bytes, end: End, reader: Reader) -> Res<Option<(i64, i64, i64)>> {
    if end.endpos == 0 {
        return Ok(None);
    }
    if end.cenlen > end.endpos {
        return Err("its end record's directory size runs before the file's start".to_string());
    }
    let cenpos = end.endpos - end.cenlen;
    let locpos = cenpos - end.cenoff;
    if locpos < 0 {
        return Err("its end record's directory offset runs before the file's start".to_string());
    }
    let read = match reader {
        Reader::ZipFile => {
            if end.cenlen > MAX_CEN_SIZE {
                return Err("its end record's directory size is over the JDK's bound".to_string());
            }
            if end.centot < 0 || end.centot > end.cenlen / CENHDR as i64 {
                return Err("its end record counts more entries than its directory holds room for".to_string());
            }
            end.cenlen
        }
        Reader::ZipFs => {
            if (end.cenlen + ENDHDR) as i32 as i64 != end.cenlen + ENDHDR || (end.centot as i32).wrapping_add(1) < 0 {
                return Err("its end record's directory size or count is over the JDK's zip file system's bound".to_string());
            }
            end.cenlen + ENDHDR
        }
    };
    if (jar.at(cenpos as u64, read as usize).len() as i64) != read {
        return Err("its central directory runs past the file".to_string());
    }
    Ok(Some((cenpos, end.cenlen, locpos)))
}

/// An entry of a central directory as the JDK's readers take it: its name, method, compressed size and the offset of
/// its local header (each from its zip64 block where its header marks the field).
#[derive(Clone, Debug, PartialEq)]
pub struct Central {
    pub name: Vec<u8>,
    pub method: u64,
    pub csize: u64,
    pub offset: u64,
}

/// An entry's extra field as the JDK's readers judge it (ZipFile.java's `checkExtraFields`,
/// `checkZip64ExtraFieldValues` and `isZip64ExtBlockSizeValid`, 1229-1382; ZipFileSystem.java's, 1620-1764, alike):
/// each block within the field, and a zip64 block of exactly the fields the header marks (the uncompressed size, the
/// compressed size and the offset 8 bytes each, the disk number 4), none negative; an empty zip64 block where the
/// header marks none. The compressed size and the offset its zip64 block gives, in the block's order (the uncompressed
/// size, the compressed size, the offset: ZipFileSystem.java `Entry.readExtra` 3264-3290).
fn check_extra(extra: &[u8], csize: u64, size: u64, offset: u64, disk: u64) -> Result<(u64, u64), String> {
    // The fields a zip64 block holds, in its order; the first block's values taken.
    let mut fields = [(size, "uncompressed size"), (csize, "compressed size"), (offset, "offset")];
    let mut taken = false;
    let mut at = 0;
    while at + 4 <= extra.len() {
        let tag = little(extra, at, 2);
        let block = little(extra, at + 2, 2) as usize;
        at += 4;
        if at + block > extra.len() {
            return Err(format!("its extra field's block of tag 0x{:04x} runs past the field", tag));
        }
        if tag == ZIP64_TAG {
            if block == 0 {
                if csize == ZIP64_FIELD || size == ZIP64_FIELD || offset == ZIP64_FIELD || disk == ZIP64_COUNT {
                    return Err("its zip64 block is empty where its header marks fields of it".to_string());
                }
            } else {
                let marked = |field: u64, marker: u64, width: usize| if field == marker { width } else { 0 };
                let wanted = marked(csize, ZIP64_FIELD, 8) + marked(size, ZIP64_FIELD, 8) + marked(offset, ZIP64_FIELD, 8) + marked(disk, ZIP64_COUNT, 4);
                if wanted != block {
                    return Err(format!("its zip64 block holds {} bytes where its header marks {}", block, wanted));
                }
                let mut off = at;
                for (k, header) in [size, csize, offset].into_iter().enumerate() {
                    if header == ZIP64_FIELD {
                        let value = little(extra, off, 8);
                        if value >> 63 == 1 {
                            return Err(format!("its zip64 block gives a negative {}", fields[k].1));
                        }
                        if !taken {
                            fields[k].0 = value;
                        }
                        off += 8;
                    }
                }
                taken = true;
            }
        }
        at += block;
    }
    Ok((fields[1].0, fields[2].0))
}

/// A central directory as java.util.zip.ZipFile reads it (`Source.initCEN` 1691-1776 and `checkAndAddEntry`
/// 1161-1219): entry after entry while a header's 46 bytes remain, the jar refused for bytes there that are no entry's,
/// an encrypted entry, a method neither stored nor deflated, a header over 65535 bytes or past the directory, its extra
/// field (`check_extra`), or marks of zip64 without one, and a name or a comment that is not UTF-8 (ZipCoder.java's
/// UTF-8 `checkedHash` 254-273 and `toString`; the jar's charset is UTF-8, so an entry's UTF-8 flag changes nothing);
/// then bytes after the last entry (1774-1776).
fn central_entries(cen: &[u8]) -> Res<Vec<Central>> {
    let mut entries = Vec::new();
    let mut pos = 0;
    while pos + CENHDR <= cen.len() {
        if little(cen, pos, 4) != CENSIG {
            return Err(format!("the central directory's bytes at {} are no entry's", pos));
        }
        let (flag, method) = (little(cen, pos + 8, 2), little(cen, pos + 10, 2));
        let (csize, size) = (little(cen, pos + 20, 4), little(cen, pos + 24, 4));
        let (nlen, elen, clen) = (little(cen, pos + 28, 2) as usize, little(cen, pos + 30, 2) as usize, little(cen, pos + 32, 2) as usize);
        let (disk, offset) = (little(cen, pos + 34, 2), little(cen, pos + 42, 4));
        let header = CENHDR + nlen + clen + elen;
        let entry = || match cen.get(pos + CENHDR..pos + CENHDR + nlen) {
            Some(name) => format!("the entry named {}", shown(name)),
            None => format!("the central directory's entry at {}", pos),
        };
        if flag & 1 != 0 {
            return Err(format!("{}: it is encrypted", entry()));
        }
        if method != 0 && method != 8 {
            return Err(format!("{}: its compression method is {}, neither stored nor deflated", entry(), method));
        }
        if header > 0xFFFF || pos + header > cen.len() {
            return Err(format!("{}: its central directory header of {} bytes is over 65535 or runs past the directory", entry(), header));
        }
        let name = &cen[pos + CENHDR..pos + CENHDR + nlen];
        let (csize, offset) = if elen > 0 {
            check_extra(&cen[pos + CENHDR + nlen..pos + CENHDR + nlen + elen], csize, size, offset, disk).map_err(|why| format!("{}: {}", entry(), why))?
        } else if csize == ZIP64_FIELD || size == ZIP64_FIELD || offset == ZIP64_FIELD || disk == ZIP64_COUNT {
            return Err(format!("{}: its header marks fields of zip64 and it has no extra field", entry()));
        } else {
            (csize, offset)
        };
        if std::str::from_utf8(name).is_err() {
            return Err(format!("{}: the name is not UTF-8", entry()));
        }
        if std::str::from_utf8(&cen[pos + CENHDR + nlen + elen..pos + header]).is_err() {
            return Err(format!("{}: its comment is not UTF-8", entry()));
        }
        entries.push(Central { name: name.to_vec(), method, csize, offset });
        pos += header;
    }
    if pos != cen.len() {
        return Err(format!("the central directory holds {} bytes after its last entry", cen.len() - pos));
    }
    Ok(entries)
}

/// What the file system of the work directory takes: the longest piece of a name and the longest path, in bytes.
#[derive(Clone, Copy)]
pub struct Limits {
    pub name: usize,
    pub path: usize,
}

/// POSIX `pathconf` of the directory: `_PC_NAME_MAX` and `_PC_PATH_MAX` (a path that long, its terminating NUL
/// counted, is refused), each without a bound where the system gives none.
#[cfg(unix)]
pub fn limits(dir: &Path) -> Res<Limits> {
    use std::os::unix::ffi::OsStrExt;
    extern "C" {
        fn pathconf(path: *const std::ffi::c_char, name: std::ffi::c_int) -> std::ffi::c_long;
    }
    #[cfg(target_os = "linux")]
    const NAME_AND_PATH: (std::ffi::c_int, std::ffi::c_int) = (3, 4);
    #[cfg(not(target_os = "linux"))]
    const NAME_AND_PATH: (std::ffi::c_int, std::ffi::c_int) = (4, 5);
    let path = std::ffi::CString::new(dir.as_os_str().as_bytes()).map_err(|_| format!("{}: a NUL in the name", dir.display()))?;
    let ask = |name| usize::try_from(unsafe { pathconf(path.as_ptr(), name) }).unwrap_or(usize::MAX);
    Ok(Limits { name: ask(NAME_AND_PATH.0), path: ask(NAME_AND_PATH.1) })
}

#[cfg(not(unix))]
pub fn limits(_: &Path) -> Res<Limits> {
    Ok(Limits { name: 255, path: usize::MAX })
}

/// A name's bytes as a refusal shows them: printable ASCII as it is, but the backslash doubled; every other byte
/// `\xHH`.
pub fn shown(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{:02X}", b)),
        }
    }
    out
}

/// `NativeLib.isNativeFile`: whether a name is all of `^scala-native/(.+)(\.c|\.cpp|\.S)$` (`Matcher.matches`), where
/// `.` is any character but a line terminator (java.util.regex.Pattern: `\n`, `\r`, U+0085, U+2028, U+2029).
fn is_native_file(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("scala-native/") else { return false };
    SOURCE_EXTENSIONS.iter().any(|ext| rest.strip_suffix(ext).is_some_and(|stem| !stem.is_empty() && !stem.contains(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}'])))
}

/// A name as the JDK's zip file system keys it (ZipFileSystem.java, the `IndexNode(byte[], int, int)` constructor and
/// `normalize`, 2709-2765): a trailing slash marks a directory and goes, a slash is put before a name that does not
/// start with one, a run of slashes is one slash, and a trailing slash left goes.
fn node_name(name: &[u8]) -> (Vec<u8>, bool) {
    let (body, dir) = match name.split_last() {
        Some((b'/', body)) => (body, true),
        _ => (name, false),
    };
    let mut key = Vec::with_capacity(body.len() + 1);
    if body.first() != Some(&b'/') {
        key.push(b'/');
    }
    for &b in body {
        if b != b'/' || key.last() != Some(&b'/') {
            key.push(b);
        }
    }
    if key.len() > 1 && key.last() == Some(&b'/') {
        key.pop();
    }
    (key, dir)
}

/// `IndexNode.pathHasDotOrDotDot` (ZipFileSystem.java 2795-2816): a piece of a key that is `.` or `..`.
fn has_dot_piece(key: &[u8]) -> bool {
    key.split(|&b| b == b'/').skip(1).any(|piece| piece == b"." || piece == b"..")
}

/// What unpacking a library's jar comes to (`read_jar`): where its entries' offsets count from; the files under
/// scala-native/ by their names below the jar's root, each with the entry it holds; and every entry the JDK's copy
/// reads.
#[derive(Debug)]
pub struct Unpacking {
    pub locpos: i64,
    pub code: Vec<(String, Central)>,
    pub read: Vec<Central>,
}

/// What Scala Native's build makes of a jar (the module's doc): none for a jar that is no library, else what unpacking
/// it comes to, in the order of its keys' first entries; `linked` is where teq's class path read took its central
/// directory to lie, and its size.
///
/// ZipFile finds the end record (`find_end`), its directory (`directory_place`), reads it (`central_entries`) and
/// decides whether the jar is a library. For one, the zip file system finds its own (it reads each entry as ZipFile
/// has, but refuses a key with a `.` or `..` piece, in the entries' order: `initCEN` 1587-1590); of entries of one key
/// the last is read (1591), the directories above a key are the pieces before its last (`getParentOff`,
/// `buildNodeTree`, `makeParentDirs`: 1152-1158, 1309-1359), and the copy walks them from the root. Refused, naming
/// the jar: the two readers' directories where they differ, and the JDK's where teq's class path read took another or
/// counts the entries' offsets from another place (the link took the program's NIR from that one: one jar, one
/// directory, one base); then what the copy cannot write as the jar
/// holds it: a name that is a file and a directory both (the copy keeps the file and leaves out whatever lies under the
/// directory), an empty name (the zip file system's root, then a file), a NUL, a piece longer than the work
/// directory's file system takes, and a path under `dest` longer than it takes.
pub fn read_jar(jar: &dyn Bytes, linked: Option<(u64, u64)>, dest: &Path, limits: Limits) -> Res<Option<Unpacking>> {
    let found = find_end(jar, Reader::ZipFile)?;
    let Some((cenpos, cenlen, locpos)) = directory_place(jar, found, Reader::ZipFile)? else { return Ok(None) };
    if let Some((at, size)) = linked {
        // src/zip.rs counts an entry's offset from the file's start.
        if (cenpos as u64, cenlen as u64, locpos) != (at, size, 0) {
            return Err(format!(
                "not supported on native: the JDK finds its central directory at byte {} ({} bytes), its entries' offsets counted from byte {}, \
                 teq's class path read at byte {} ({} bytes), from byte 0",
                cenpos, cenlen, locpos, at, size
            ));
        }
    }
    let entries = central_entries(&jar.at(cenpos as u64, cenlen as usize))?;
    if !entries.iter().any(|e| std::str::from_utf8(&e.name).is_ok_and(is_native_file)) {
        return Ok(None);
    }
    let unzipped = find_end(jar, Reader::ZipFs).and_then(|end| directory_place(jar, end, Reader::ZipFs)).map_err(|why| format!("the JDK's zip file system: {}", why))?;
    if unzipped != Some((cenpos, cenlen, locpos)) {
        let at = unzipped.map_or_else(|| "no central directory".to_string(), |(pos, len, _)| format!("one at byte {} ({} bytes)", pos, len));
        return Err(format!("not supported on native: java.util.zip.ZipFile finds its central directory at byte {} ({} bytes), the JDK's zip file system {}", cenpos, cenlen, at));
    }
    let entry = |at: usize| format!("the entry named {}", shown(&entries[at].name));
    let keyed: Vec<(Vec<u8>, bool)> = entries.iter().map(|e| node_name(&e.name)).collect();
    if let Some(at) = keyed.iter().position(|(key, _)| has_dot_piece(key)) {
        return Err(format!("{}: a piece of the name is . or ..", entry(at)));
    }
    // Each key in the order of its first entry, with its last entry.
    let mut order: Vec<&[u8]> = Vec::new();
    let mut last: crate::intern::FxMap<&[u8], usize> = crate::intern::FxMap::default();
    for (at, (key, dir)) in keyed.iter().enumerate() {
        if entries[at].name.is_empty() {
            return Err("not supported on native: an entry with an empty name".to_string());
        }
        match last.insert(key, at) {
            None => order.push(key),
            Some(earlier) if keyed[earlier].1 != *dir => {
                return Err(format!("the entries named {} and {}: not supported on native: a file and a directory of one name", shown(&entries[earlier].name), shown(&entries[at].name)));
            }
            Some(_) => {}
        }
    }
    for key in &order {
        let at = last[key];
        let above = (1..key.len()).filter(|&end| key[end] == b'/').map(|end| &key[..end]);
        if let Some(file) = above.filter_map(|parent| last.get(parent)).find(|&&holder| !keyed[holder].1) {
            return Err(format!("the entries named {} and {}: not supported on native: a file and a directory of one name", shown(&entries[*file].name), shown(&entries[at].name)));
        }
    }
    let base = dest.as_os_str().len() + 1;
    for key in &order {
        let at = last[key];
        let name = &key[1..];
        if name.contains(&0) {
            return Err(format!("{}: the name holds a NUL", entry(at)));
        }
        if name.split(|&b| b == b'/').any(|piece| piece.len() > limits.name) {
            return Err(format!("{}: a piece of the name is longer than the work directory's file system takes ({} bytes)", entry(at), limits.name));
        }
        if base + name.len() >= limits.path {
            return Err(format!("{}: unpacked under {}, its path is longer than the system takes ({} bytes)", entry(at), dest.display(), limits.path - 1));
        }
    }
    let code = format!("/{}/", CODE_DIR);
    let written: Vec<&&[u8]> = order.iter().filter(|key| !keyed[last[*key]].1).collect();
    Ok(Some(Unpacking {
        locpos,
        code: written
            .iter()
            .filter(|key| key.starts_with(code.as_bytes()))
            .map(|key| (String::from_utf8_lossy(&key[1..]).into_owned(), entries[last[**key]].clone()))
            .collect(),
        read: written.iter().map(|key| entries[last[**key]].clone()).collect(),
    }))
}

/// An entry's data as `Files.copy` reads it out of the JDK's zip file system (ZipFileSystem.java `getInputStream`
/// 2397-2457 and `EntryInputStream` 2459-2560): from the local header at the entry's offset past `locpos`, its
/// signature checked and its name's and extra field's lengths giving where the data starts (`initDataPos` 2546-2558),
/// the compressed size of bytes, fewer where the file ends (the stream ends there); stored data as it is; deflated
/// data inflated to its stream's end with no bound from the size, one zero byte given after the compressed bytes as
/// the inflater's stream gives one at their end (2434-2441), a stream that has not ended then an error, inflated by
/// zlib's inflate as the JDK bundles it (`inflate`); neither the size nor the CRC-32 checked.
pub fn copy_entry(jar: &dyn Bytes, locpos: i64, e: &Central) -> Res<Vec<u8>> {
    let entry = format!("the entry named {}", shown(&e.name));
    let pos = (locpos as u64).saturating_add(e.offset);
    let header = jar.at(pos, LOCHDR);
    if header.len() != LOCHDR {
        return Err(format!("{}: no local header at {}", entry, pos));
    }
    if little(&header, 0, 4) != LOCSIG {
        return Err(format!("{}: the bytes at {} are no local header", entry, pos));
    }
    let start = pos.saturating_add(LOCHDR as u64 + little(&header, 26, 2) + little(&header, 28, 2));
    let data = jar.at(start, e.csize.min(jar.len()) as usize);
    match e.method {
        0 => Ok(data),
        _ => {
            let mut stream = data;
            stream.push(0);
            super::inflate::inflate(&stream).map_err(|stop| match stop {
                Stop::Refused(why) => format!("{}: its deflated data: {}", entry, why),
                Stop::Short => format!("{}: its deflated data: unexpected end of deflate stream", entry),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A central directory entry for a test: its name, comment, flag, method, extra field, the sizes, offset and disk
    /// number its header holds.
    #[derive(Clone, Copy)]
    struct Cen<'a> {
        name: &'a [u8],
        comment: &'a [u8],
        flag: u16,
        method: u16,
        extra: &'a [u8],
        csize: u32,
        size: u32,
        offset: u32,
        disk: u16,
    }

    fn plain(name: &[u8]) -> Cen<'_> {
        Cen { name, comment: b"", flag: 0, method: 0, extra: b"", csize: 0, size: 0, offset: 0, disk: 0 }
    }

    /// The bytes of a central directory of the entries (APPNOTE 4.3.12).
    fn directory(entries: &[Cen]) -> Vec<u8> {
        let mut out = Vec::new();
        for e in entries {
            out.extend(0x0201_4b50u32.to_le_bytes());
            out.extend([0x14, 0x03, 20, 0]);
            out.extend(e.flag.to_le_bytes());
            out.extend(e.method.to_le_bytes());
            out.extend([0, 0, 0x21, 0, 0, 0, 0, 0]);
            out.extend(e.csize.to_le_bytes());
            out.extend(e.size.to_le_bytes());
            out.extend((e.name.len() as u16).to_le_bytes());
            out.extend((e.extra.len() as u16).to_le_bytes());
            out.extend((e.comment.len() as u16).to_le_bytes());
            out.extend(e.disk.to_le_bytes());
            out.extend([0, 0, 0, 0, 0, 0]);
            out.extend(e.offset.to_le_bytes());
            out.extend(e.name);
            out.extend(e.extra);
            out.extend(e.comment);
        }
        out
    }

    /// An end record: its count, the directory's size and offset, and its comment.
    fn end_record(count: u16, size: u32, offset: u32, comment: &[u8]) -> Vec<u8> {
        let mut out = 0x0605_4b50u32.to_le_bytes().to_vec();
        out.extend([0, 0, 0, 0]);
        out.extend(count.to_le_bytes());
        out.extend(count.to_le_bytes());
        out.extend(size.to_le_bytes());
        out.extend(offset.to_le_bytes());
        out.extend((comment.len() as u16).to_le_bytes());
        out.extend(comment);
        out
    }

    /// A jar of no data whose file is the directory then its end record.
    fn jar_of(cen: &[u8], count: usize) -> Vec<u8> {
        let mut out = cen.to_vec();
        out.extend(end_record(count as u16, cen.len() as u32, 0, b""));
        out
    }

    const ROOMY: Limits = Limits { name: 255, path: 4096 };
    const DEST: &str = "/w/dependencies/lib-0";

    fn read_directory(cen: &[u8], count: usize, limits: Limits) -> Result<Option<Vec<(String, usize)>>, String> {
        let jar = jar_of(cen, count);
        let names: Vec<Vec<u8>> = central_entries(cen).map(|entries| entries.into_iter().map(|e| e.name).collect()).unwrap_or_default();
        read_jar(&jar, None, Path::new(DEST), limits).map(|read| {
            read.map(|unpacking| unpacking.code.into_iter().map(|(name, e)| (name, names.iter().rposition(|n| *n == e.name).unwrap_or(usize::MAX))).collect())
        })
    }

    fn read(entries: &[Cen]) -> Result<Option<Vec<(String, usize)>>, String> {
        read_directory(&directory(entries), entries.len(), ROOMY)
    }

    fn reading(names: &[&[u8]]) -> Result<Option<Vec<(String, usize)>>, String> {
        read(&names.iter().map(|name| plain(name)).collect::<Vec<_>>())
    }

    fn files(pairs: &[(&str, usize)]) -> Option<Vec<(String, usize)>> {
        Some(pairs.iter().map(|(name, at)| (name.to_string(), *at)).collect())
    }

    /// What the JDK's readers make of a jar's names (`read_jar`): a jar is a library where a name is a native file's;
    /// names are read as the zip file system keys them, the last entry of a key taken.
    #[test]
    fn reads_a_jars_names_as_the_jdks_readers_do() {
        assert_eq!(reading(&[b"scala-native/scala-native.properties", b"scala-native/jk.c"]), Ok(files(&[("scala-native/scala-native.properties", 0), ("scala-native/jk.c", 1)])));
        // No native file: a header, a leading slash, a backslash, a line terminator in the stem, an empty stem, a
        // directory; and a `..` piece then refused by nothing, as the zip file system never opens such a jar.
        for name in [&b"scala-native/x.h"[..], b"/scala-native/abs.c", b"scala-native\\bs.c", b"scala-native/a\nb.c", b"scala-native/a\xc2\x85b.c", b"scala-native/a\xe2\x80\xa8b.c", b"scala-native/.c", b"scala-native/d.c/", b"Scala-native/x.c", b"scala-native/x.C"] {
            assert_eq!(reading(&[name, b"include/../x.h"]), Ok(None), "{}", shown(name));
        }
        assert_eq!(reading(&[b"scala-native/jk.c", b"/scala-native/abs.c", b"scala-native//two.c", b"scala-native/sub///three.c/x.c"]), Ok(files(&[("scala-native/jk.c", 0), ("scala-native/abs.c", 1), ("scala-native/two.c", 2), ("scala-native/sub/three.c/x.c", 3)])));
        // Of one key the last entry, in the place of the first; a directory entry, `t.c.` and `d./u.c` as they are.
        let read = read(&[plain(b"scala-native/jk.c"), plain(b"scala-native/x.c"), Cen { comment: b"last", ..plain(b"/scala-native/jk.c") }, plain(b"scala-native/d./u.c"), plain(b"scala-native/t.c."), plain(b"scala-native/sub/")]);
        assert_eq!(read, Ok(files(&[("scala-native/jk.c", 2), ("scala-native/x.c", 1), ("scala-native/d./u.c", 3), ("scala-native/t.c.", 4)])));
        // Entries outside scala-native/ are judged and not unpacked.
        assert_eq!(reading(&[b"META-INF/MANIFEST.MF", b"scala-native/jk.c", b"x/y.class"]), Ok(files(&[("scala-native/jk.c", 1)])));
    }

    /// The jars `read_jar` refuses, each by the entry it names, whatever the entry's place in the jar.
    #[test]
    fn refuses_the_names_the_jdks_readers_or_the_copy_cannot_take() {
        let refusal = |names: &[&[u8]]| reading(names).expect_err(&names.iter().map(|n| shown(n)).collect::<Vec<_>>().join(" "));
        for bad in [&b"scala-native/m\xe9thode.c"[..], b"lib/\xc0\xaf.class", b"scala-native/\xed\xa0\x80.c", b"scala-native/\xf4\x90\x80\x80.c", b"scala-native/\xe2\x82.c"] {
            for names in [[bad, b"scala-native/jk.c"], [b"scala-native/jk.c", bad]] {
                assert_eq!(refusal(&names), format!("the entry named {}: the name is not UTF-8", shown(bad)));
            }
            assert_eq!(refusal(&[bad, b"x.class"]), format!("the entry named {}: the name is not UTF-8", shown(bad)), "a jar of no C code too");
        }
        let comment = read(&[plain(b"scala-native/jk.c"), Cen { comment: b"caf\xe9", ..plain(b"a.class") }]);
        assert_eq!(comment, Err("the entry named a.class: its comment is not UTF-8".to_string()));
        for bad in [&b"include/../x.h"[..], b"scala-native/./a.c", b"./a.c", b"scala-native/x/..", b"a/./", b"scala-native//../a.c"] {
            for names in [[bad, b"scala-native/jk.c"], [b"scala-native/jk.c", bad]] {
                assert_eq!(refusal(&names), format!("the entry named {}: a piece of the name is . or ..", shown(bad)));
            }
        }
        assert_eq!(refusal(&[b"scala-native/jk.c", b""]), "not supported on native: an entry with an empty name");
        for (one, other) in [(&b"scala-native/x"[..], &b"scala-native/x/y.c"[..]), (b"scala-native/x/y.c", b"scala-native/x"), (b"scala-native/d.c", b"scala-native/d.c/"), (b"scala-native/d.c/", b"scala-native/d.c"), (b"a", b"/a/b/c")] {
            let said = refusal(&[b"scala-native/jk.c", one, other]);
            assert!(said.ends_with("not supported on native: a file and a directory of one name") && said.contains(&shown(one)) && said.contains(&shown(other)), "{said}");
        }
        assert_eq!(refusal(&[b"scala-native/jk.c", b"scala-native/a\0b.c"]), "the entry named scala-native/a\\x00b.c: the name holds a NUL");
        let tight = Limits { name: 12, path: 40 };
        let judged = |names: &[&[u8]]| read_directory(&directory(&names.iter().map(|name| plain(name)).collect::<Vec<_>>()), names.len(), tight);
        assert!(judged(&[b"scala-native/jk.c", b"x/1234567890123"]).unwrap_err().ends_with("a piece of the name is longer than the work directory's file system takes (12 bytes)"));
        assert!(judged(&[b"scala-native/jk.c", b"x/123456789012"]).is_ok());
        // The path: 21 bytes of the directory, a slash, then the name; 40 counts the terminating NUL.
        assert!(judged(&[b"scala-native/jk.c", b"x/1/2/3/4/5/6/7/8"]).is_ok());
        assert!(judged(&[b"scala-native/jk.c", b"x/1/2/3/4/5/6/7/89"]).unwrap_err().ends_with("its path is longer than the system takes (39 bytes)"));
        assert_eq!(shown(b"a\\b\x01\x7f\xff z"), "a\\\\b\\x01\\x7F\\xFF z");
    }

    /// What ZipFile refuses of a central directory entry besides its name (`central_entries`): each refused naming the
    /// entry, whatever the jar holds; the values its zip64 block gives.
    #[test]
    fn refuses_the_entries_the_jdks_readers_refuse() {
        let jk = plain(b"scala-native/jk.c");
        let x = plain(b"x.class");
        let refusal = |entries: &[Cen]| read(entries).expect_err("taken");
        assert_eq!(refusal(&[jk, Cen { flag: 1, ..x }]), "the entry named x.class: it is encrypted");
        assert_eq!(refusal(&[jk, Cen { method: 12, ..x }]), "the entry named x.class: its compression method is 12, neither stored nor deflated");
        assert!(read(&[jk, Cen { method: 8, flag: 0x808, ..x }]).is_ok(), "deflated, a UTF-8 name and a data descriptor");
        let at_limit = vec![b'c'; 0xFFFF - CENHDR - 7];
        assert!(read(&[jk, Cen { comment: &at_limit, ..x }]).is_ok(), "a header of 65535 bytes");
        let over = vec![b'c'; 0xFFFF - CENHDR - 6];
        assert!(refusal(&[jk, Cen { comment: &over, ..x }]).ends_with("its central directory header of 65536 bytes is over 65535 or runs past the directory"));
        // Zip64: a block of the marked fields alone, none negative, its values taken; marks without a block; a block
        // past the field.
        let mut sizes = vec![0x01, 0x00, 16, 0];
        sizes.extend(5u64.to_le_bytes());
        sizes.extend(7u64.to_le_bytes());
        let marked = Cen { csize: u32::MAX, size: u32::MAX, extra: &sizes, ..x };
        assert_eq!(central_entries(&directory(&[marked])).map(|e| e[0].csize), Ok(7), "the uncompressed size first, then the compressed");
        assert!(read(&[jk, Cen { name: b"scala-native/z.c", ..marked }]).is_ok(), "a C file of zip64's sizes, read as the JDK reads it");
        assert_eq!(refusal(&[jk, Cen { size: u32::MAX, extra: &sizes, ..x }]), "the entry named x.class: its zip64 block holds 16 bytes where its header marks 8");
        let mut negative = vec![0x01, 0x00, 8, 0];
        negative.extend(u64::MAX.to_le_bytes());
        assert_eq!(refusal(&[jk, Cen { size: u32::MAX, extra: &negative, ..x }]), "the entry named x.class: its zip64 block gives a negative uncompressed size");
        assert_eq!(refusal(&[jk, Cen { offset: u32::MAX, extra: &[0x01, 0x00, 0, 0], ..x }]), "the entry named x.class: its zip64 block is empty where its header marks fields of it");
        assert!(read(&[jk, Cen { extra: &[0x01, 0x00, 0, 0], ..x }]).is_ok(), "an empty zip64 block where none is marked");
        assert_eq!(refusal(&[jk, Cen { extra: &[0x55, 0x54, 9, 0, 1, 2], ..x }]), "the entry named x.class: its extra field's block of tag 0x5455 runs past the field");
        assert!(read(&[jk, Cen { extra: &[0x55, 0x54, 1, 0, 1, 9], ..x }]).is_ok(), "a block of another tag, and bytes too few for another block");
        for marks in [Cen { csize: u32::MAX, ..x }, Cen { size: u32::MAX, ..x }, Cen { offset: u32::MAX, ..x }, Cen { disk: 0xFFFF, ..x }] {
            assert_eq!(refusal(&[jk, marks]), "the entry named x.class: its header marks fields of zip64 and it has no extra field");
        }
        // Bytes after the last entry, of any jar: no entry's where a header's 46 remain, else bytes after it.
        let mut trailing = directory(&[jk]);
        trailing.extend([0u8; 46]);
        assert_eq!(read_directory(&trailing, 1, ROOMY), Err(format!("the central directory's bytes at {} are no entry's", trailing.len() - 46)));
        let mut few = directory(&[jk]);
        few.extend([7u8; 5]);
        assert_eq!(read_directory(&few, 1, ROOMY), Err("the central directory holds 5 bytes after its last entry".to_string()));
        let mut few_of_none = directory(&[x]);
        few_of_none.extend([7u8; 5]);
        assert_eq!(read_directory(&few_of_none, 1, ROOMY), Err("the central directory holds 5 bytes after its last entry".to_string()));
    }

    /// The end record each reader finds and the directory it takes: a second end record in the comment, a stub before
    /// the jar, a zip64 locator and end record taken where their values agree, the place teq's class path read took.
    #[test]
    fn finds_the_directory_as_each_jdk_reader_does() {
        let cen = directory(&[plain(b"scala-native/jk.c")]);
        let jar = jar_of(&cen, 1);
        let place = |jar: &Vec<u8>, reader| find_end(jar, reader).and_then(|end| directory_place(jar, end, reader));
        assert_eq!(place(&jar, Reader::ZipFile), Ok(Some((0, cen.len() as i64, 0))));
        assert_eq!(place(&jar, Reader::ZipFs), Ok(Some((0, cen.len() as i64, 0))));
        assert!(read_jar(&jar, Some((0, cen.len() as u64)), Path::new(DEST), ROOMY).is_ok());
        assert!(read_jar(&jar, Some((0, cen.len() as u64 + 1)), Path::new(DEST), ROOMY).unwrap_err().contains("teq's class path read at byte 0"));
        // An end record in the comment, its directory none: the comment's record does not reach the file's end, and
        // ZipFile's check of its directory's signature fails: both take the outer one.
        let mut fake = cen.clone();
        let mut inner = end_record(1, 9, 3, b"");
        inner.extend(b"xx");
        fake.extend(end_record(1, cen.len() as u32, 0, &inner));
        assert_eq!(place(&fake, Reader::ZipFile), Ok(Some((0, cen.len() as i64, 0))));
        assert_eq!(place(&fake, Reader::ZipFs), Ok(Some((0, cen.len() as i64, 0))));
        // A stub before the jar: the entries' offsets count from past it.
        let mut stubbed = vec![b'#'; 7];
        stubbed.extend(&cen);
        stubbed.extend(end_record(1, cen.len() as u32, 0, b""));
        assert_eq!(place(&stubbed, Reader::ZipFile), Ok(Some((7, cen.len() as i64, 7))));
        // The same directory where teq's class path read took it, its offsets counted from the file's start there: the
        // link would read the program's NIR from other local headers than the JDK's copy reads.
        assert!(read_jar(&stubbed, Some((7, cen.len() as u64)), Path::new(DEST), ROOMY).unwrap_err().contains("its entries' offsets counted from byte 7"));
        // Padding after the end record: the zip file system finds none; ZipFile takes it where the directory's and the
        // first local header's signatures stand (here the local header's does not).
        let mut padded = jar.clone();
        padded.extend([0u8; 3]);
        assert!(place(&padded, Reader::ZipFs).unwrap_err().contains("no end record"));
        assert!(place(&padded, Reader::ZipFile).unwrap_err().contains("no end record"));
        // A zip64 locator and end record whose values are the end record's: its place the end's; one whose values
        // differ, passed over; one at a negative offset: ZipFile passes it over, the zip file system cannot seek.
        let zip64 = |size: u64, offset: u64, count: u64, at: u64| {
            let mut out = cen.clone();
            let record_at = out.len() as u64;
            out.extend(0x0606_4b50u32.to_le_bytes());
            out.extend(44u64.to_le_bytes());
            out.extend([45, 0, 45, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(count.to_le_bytes());
            out.extend(count.to_le_bytes());
            out.extend(size.to_le_bytes());
            out.extend(offset.to_le_bytes());
            out.extend(0x0706_4b50u32.to_le_bytes());
            out.extend(0u32.to_le_bytes());
            out.extend(if at == u64::MAX { record_at } else { at }.to_le_bytes());
            out.extend(1u32.to_le_bytes());
            out.extend(end_record(1, cen.len() as u32, 0, b""));
            (out, record_at as i64)
        };
        let (agreeing, record_at) = zip64(cen.len() as u64, 0, 1, u64::MAX);
        assert_eq!(find_end(&agreeing, Reader::ZipFile).map(|e| e.endpos), Ok(record_at));
        assert_eq!(place(&agreeing, Reader::ZipFile), place(&agreeing, Reader::ZipFs));
        let (differing, _) = zip64(cen.len() as u64 + 1, 0, 1, u64::MAX);
        assert_eq!(find_end(&differing, Reader::ZipFs).map(|e| e.endpos), Ok(differing.len() as i64 - ENDHDR));
        let (negative, _) = zip64(cen.len() as u64, 0, 1, 1 << 63);
        assert_eq!(find_end(&negative, Reader::ZipFile).map(|e| e.endpos), Ok(negative.len() as i64 - ENDHDR));
        assert!(find_end(&negative, Reader::ZipFs).unwrap_err().contains("negative offset"));
        // The end record's sizes and count judged.
        let mut short = cen.clone();
        short.extend(end_record(1, cen.len() as u32 + 100, 0, b""));
        assert!(place(&short, Reader::ZipFile).unwrap_err().contains("directory size runs before the file's start"));
        let mut many = cen.clone();
        many.extend(end_record(2, cen.len() as u32, 0, b""));
        assert!(place(&many, Reader::ZipFile).unwrap_err().contains("counts more entries"));
        assert!(place(&many, Reader::ZipFs).is_ok(), "the zip file system takes no count from it");
        assert_eq!(place(&end_record(0, 0, 0, b""), Reader::ZipFile), Ok(None), "an end record alone");
    }

    /// The copy of an entry's data (`copy_entry`): the local header's own lengths, the compressed size's bytes, a
    /// deflated stream inflated to its end whatever the size says, a cut stream refused, no CRC-32 or size checked.
    #[test]
    fn copies_an_entry_as_the_jdks_zip_file_system_reads_it() {
        let local = |name: &[u8], extra: &[u8], data: &[u8]| {
            let mut out = 0x0403_4b50u32.to_le_bytes().to_vec();
            out.extend([20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0, 0xde, 0xad, 0xbe, 0xef]);
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend((extra.len() as u16).to_le_bytes());
            out.extend(name);
            out.extend(extra);
            out.extend(data);
            out
        };
        let text = b"compile.c.options = -DFOO=1\n".repeat(40);
        let deflated = crate::deflate::deflate(&text, 6, false);
        let file = local(b"x.properties", &[0u8; 200], &deflated);
        let e = |method: u64, csize: u64| Central { name: b"x.properties".to_vec(), method, csize, offset: 0 };
        assert_eq!(copy_entry(&file, 0, &e(8, deflated.len() as u64)), Ok(text.clone()), "whatever the size says, a long local extra field");
        assert!(copy_entry(&file, 0, &e(8, deflated.len() as u64 - 9)).unwrap_err().contains("its deflated data: unexpected end"));
        let stored = local(b"crc.c", &[], b"int x;\n");
        let c = |csize: u64, offset: u64| Central { name: b"crc.c".to_vec(), method: 0, csize, offset };
        assert_eq!(copy_entry(&stored, 0, &c(7, 0)), Ok(b"int x;\n".to_vec()), "no CRC-32 checked (the local header's is 0xefbeadde)");
        assert_eq!(copy_entry(&stored, 0, &c(70, 0)), Ok(b"int x;\n".to_vec()), "stored data cut by the file's end");
        assert!(copy_entry(&stored, 0, &c(7, 1)).unwrap_err().ends_with("are no local header"));
        let mut stubbed = vec![9u8; 5];
        stubbed.extend(&stored);
        assert_eq!(copy_entry(&stubbed, 5, &c(7, 0)), Ok(b"int x;\n".to_vec()), "offsets past a stub");
    }

    /// The table of flipped streams (tests/native-build/inflate-flips.expected, written by jar-names-oracle.scala
    /// through the JDK's zip file system): each stream, the deflated data of an entry, copied as the JDK copied it,
    /// refused for the same reason or the same bytes.
    #[test]
    fn copies_every_flipped_stream_as_the_jdks_zip_file_system_does() {
        let table = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/native-build/inflate-flips.expected")).unwrap();
        let hex = |text: &str| (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let bases: Vec<Vec<u8>> = table.lines().filter_map(|line| line.strip_prefix("base ")).map(|rest| hex(rest.split(' ').nth(1).unwrap())).collect();
        let name = b"scala-native/flip.h";
        let (mut flips, mut unlike) = (0, Vec::new());
        for line in table.lines().filter(|line| line.starts_with("flip ")) {
            let fields: Vec<&str> = line.splitn(5, ' ').collect();
            let mut stream = bases[fields[2].parse::<usize>().unwrap()].clone();
            for bit in fields[3].split(',').map(|b| b.parse::<usize>().unwrap()) {
                stream[bit / 8] ^= 1 << (bit % 8);
            }
            let mut jar = (LOCSIG as u32).to_le_bytes().to_vec();
            jar.extend([20, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            jar.extend((stream.len() as u32).to_le_bytes());
            jar.extend([0, 0, 0, 0]);
            jar.extend((name.len() as u16).to_le_bytes());
            jar.extend([0, 0]);
            jar.extend(name);
            jar.extend(&stream);
            let entry = Central { name: name.to_vec(), method: 8, csize: stream.len() as u64, offset: 0 };
            let copied = match copy_entry(&jar, 0, &entry) {
                Ok(bytes) => format!("bytes {}", &crate::task::sha256::hex(&bytes)[..16]), // the JDK side's 16 digits
                Err(why) if why.ends_with("its deflated data: unexpected end of deflate stream") => "refused java.io.EOFException: Unexpected end of ZLIB input stream".to_string(),
                Err(why) => match why.split_once("its deflated data: ") {
                    Some((_, zlib)) => format!("refused java.util.zip.ZipException: {}", zlib),
                    None => format!("refused otherwise: {}", why),
                },
            };
            flips += 1;
            if copied != fields[4] {
                unlike.push(format!("{}: the JDK {}, the copy {}", fields[1], fields[4], copied));
            }
        }
        eprintln!("inflate flips: {} streams copied, {} unlike the JDK", flips, unlike.len());
        assert!(flips >= 3000 && unlike.is_empty(), "{} of {} unlike: {:?}", unlike.len(), flips, &unlike[..unlike.len().min(5)]);
    }
}
