//! The natives of `std/javalib/zip.scala` and `std/javalib/time.scala`: zip archives' central
//! directories and entries read (`src/zip.rs`), DEFLATE inflated as its input comes
//! (`crate::zip::Inflating`, behind `Inflater`) and written as zlib writes it (`src/deflate.rs`),
//! CRC-32, the digests `MessageDigest` offers (MD5, SHA-1, SHA-256), and the machine's local time,
//! which DOS timestamps and `java.time` read.

use super::builtins::{reg, Table};
use super::value::*;
use super::*;

type A<'a> = &'a [Value];

thread_local! {
    static ZIPS: RefCell<Vec<Option<crate::zip::Zip>>> = const { RefCell::new(Vec::new()) };
    static DIGESTS: RefCell<Vec<Digest>> = const { RefCell::new(Vec::new()) };
    static INFLATERS: RefCell<Vec<Option<crate::zip::Inflating>>> = const { RefCell::new(Vec::new()) };
}

/// `f` of the inflater, or the JDK's `NullPointerException` for one that was ended.
fn with_inflater(it: &mut Interp, id: i32, f: impl FnOnce(&mut crate::zip::Inflating) -> Value) -> R {
    match INFLATERS.with(|i| i.borrow_mut().get_mut(id as usize).and_then(|x| x.as_mut()).map(f)) {
        Some(v) => Ok(v),
        None => it.throw_named("NullPointerException", "Inflater has been closed"),
    }
}

fn int(it: &mut Interp, a: A, i: usize) -> R<i32> {
    match a.get(i).and_then(|v| v.as_i32()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Int", i)),
    }
}

fn long(it: &mut Interp, a: A, i: usize) -> R<i64> {
    match a.get(i).and_then(|v| v.as_i64()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Long", i)),
    }
}

/// The bytes of an `Array[Byte]` argument, `off` and `len` of them when given.
fn bytes(it: &mut Interp, a: A, i: usize, range: Option<(usize, usize)>) -> R<Vec<u8>> {
    let Some(Value::Array(items)) = a.get(i) else { return it.throw_named("NullPointerException", "") };
    let items = items.borrow();
    let (off, len) = range.unwrap_or((0, items.len()));
    if off + len > items.len() {
        let n = items.len();
        drop(items);
        return it.throw_named("ArrayIndexOutOfBoundsException", &format!("Range [{}, {} + {}) out of bounds for length {}", off, off, len, n));
    }
    Ok(items[off..off + len].iter().map(|v| v.as_i64().unwrap_or(0) as u8).collect())
}

fn byte_array(bytes: &[u8]) -> Value {
    Value::array(bytes.iter().map(|&b| Value::Byte(b as i8)).collect())
}

fn zip_exception<T>(it: &mut Interp, message: &str) -> R<T> {
    it.throw_new(&["java", "util", "zip", "ZipException"], vec![Value::str(message)])
}

// ---- digests ----

enum Digest {
    Md5(Md5),
    Sha1(crate::task::fetch::Sha1),
    Sha256(crate::task::sha256::Sha256),
}

impl Digest {
    fn named(name: &str) -> Option<Digest> {
        match name.to_ascii_uppercase().as_str() {
            "MD5" => Some(Digest::Md5(Md5::new())),
            "SHA-1" | "SHA1" | "SHA" => Some(Digest::Sha1(crate::task::fetch::Sha1::new())),
            "SHA-256" | "SHA256" => Some(Digest::Sha256(crate::task::sha256::Sha256::new())),
            _ => None,
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Digest::Md5(d) => d.update(data),
            Digest::Sha1(d) => d.update(data),
            Digest::Sha256(d) => d.update(data),
        }
    }

    /// The digest, the state reset for the next, as `MessageDigest.digest` leaves it.
    fn finish(&mut self) -> Vec<u8> {
        let hex = match std::mem::replace(self, Digest::Md5(Md5::new())) {
            Digest::Md5(d) => {
                *self = Digest::Md5(Md5::new());
                return d.finish().to_vec();
            }
            Digest::Sha1(d) => {
                *self = Digest::Sha1(crate::task::fetch::Sha1::new());
                d.hex()
            }
            Digest::Sha256(d) => {
                *self = Digest::Sha256(crate::task::sha256::Sha256::new());
                d.hex()
            }
        };
        (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0)).collect()
    }
}

/// MD5 (RFC 1321).
struct Md5 {
    state: [u32; 4],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

impl Md5 {
    fn new() -> Md5 {
        Md5 { state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476], block: [0; 64], filled: 0, length: 0 }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        while !data.is_empty() {
            let take = (64 - self.filled).min(data.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&data[..take]);
            self.filled += take;
            data = &data[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let m: Vec<u32> = block.chunks_exact(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let [mut a, mut b, mut c, mut d] = self.state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let rotated = a.wrapping_add(f).wrapping_add(MD5_K[i]).wrapping_add(m[g]).rotate_left(MD5_S[i]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rotated);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d]) {
            *s = s.wrapping_add(v);
        }
    }

    fn finish(mut self) -> [u8; 16] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_le_bytes());
        let mut out = [0u8; 16];
        for (i, w) in self.state.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
}

// ---- gzip and zip streams ----

/// The members of a gzip stream unpacked one after another, each header's flags honoured and
/// each trailer checked, as `GZIPInputStream` reads them; the JDK's messages for what is wrong.
pub(super) fn offset_at(epoch_seconds: i64) -> i32 {
    os::offset_at(epoch_seconds)
}

/// The instant of a local date and time, in seconds since the epoch: the earlier offset where the
/// time comes twice, the time moved on by the gap where it is skipped.
fn local_to_epoch(fields: [i32; 6]) -> i64 {
    let [y, mo, d, h, mi, s] = fields;
    let naive = days_from_civil(y as i64, mo as u32, d as u32) * 86400 + (h as i64) * 3600 + (mi as i64) * 60 + s as i64;
    // The offsets just before and after; the earlier one that maps back to the same local time.
    let before = offset_at(naive - 86400) as i64;
    let after = offset_at(naive + 86400) as i64;
    for off in [before, after] {
        let t = naive - off;
        if offset_at(t) as i64 == off {
            return t;
        }
    }
    naive - before
}

/// The days from 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's algorithm).
pub(super) fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date of a day counted from 1970-01-01: year, month (1 to 12) and day.
#[cfg(windows)]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}

/// The zone's name as the JDK takes the machine's: `TZ`, else the zone `/etc/localtime` links to
/// (`/etc/timezone` first on Debian), else UTC; on Windows the offset's `GMT+hh:mm`.
pub(super) fn zone_name() -> String {
    os::zone_name()
}

pub(super) fn install(it: &mut Table) {
    reg!(it, "java.util.zip.crc32Update", |it, a| {
        let crc = int(it, a, 0)? as u32;
        let (off, len) = (int(it, a, 2)?, int(it, a, 3)?);
        let data = bytes(it, a, 1, Some((off.max(0) as usize, len.max(0) as usize)))?;
        Ok(Value::Int(crate::zip::crc32_update(crc, &data) as i32))
    });
    reg!(it, "java.util.zip.deflateAll", |it, a| {
        let data = bytes(it, a, 0, None)?;
        let level = int(it, a, 1)?;
        let nowrap = matches!(a.get(2), Some(Value::Bool(true)));
        Ok(byte_array(&crate::deflate::deflate(&data, level, !nowrap)))
    });
    // A raw or zlib DEFLATE stream unpacked whole, and how much of the input it took.
    reg!(it, "java.util.zip.zipOpen", |it, a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the file system");
        }
        let path = it.str_arg(a, 0)?;
        if let Err(e) = std::fs::metadata(&*path) {
            return super::files::fail(it, &e, &path);
        }
        match crate::zip::Zip::open(&path) {
            Ok(zip) => Ok(Value::Int(ZIPS.with(|z| {
                let mut zips = z.borrow_mut();
                zips.push(Some(zip));
                (zips.len() - 1) as i32
            }))),
            Err(m) if m.contains("no end of central directory") => zip_exception(it, "zip END header not found"),
            Err(m) => zip_exception(it, &m),
        }
    });
    // The entries: per entry its name, then method, DOS time, CRC-32, compressed size, size, then
    // its extra field in the central directory (null for none) and its comment (null for none).
    reg!(it, "java.util.zip.zipEntries", |it, a| {
        let id = int(it, a, 0)?;
        type Listed = (String, [i64; 5], Vec<u8>, Option<String>);
        let entries = ZIPS.with(|z| {
            let zips = z.borrow();
            let zip = zips.get(id as usize)?.as_ref()?;
            Some(
                zip.entries
                    .iter()
                    .map(|e| -> Listed {
                        let comment = zip.comment(e);
                        let comment = (!comment.is_empty()).then(|| String::from_utf8_lossy(comment).into_owned());
                        (zip.name(e).to_string(), [e.method as i64, e.dos_time as i64, e.crc32 as i64, e.compressed_size as i64, e.size as i64], zip.extra(e).to_vec(), comment)
                    })
                    .collect::<Vec<_>>(),
            )
        });
        let Some(entries) = entries else { return it.throw_named("IllegalStateException", "zip file closed") };
        let mut out = Vec::new();
        for (name, fields, extra, comment) in entries {
            out.push(Value::string(name));
            out.extend(fields.iter().map(|&f| Value::Long(f)));
            out.push(if extra.is_empty() { Value::Null } else { byte_array(&extra) });
            out.push(comment.map_or(Value::Null, Value::string));
        }
        Ok(Value::array(out))
    });
    // `Inflater`'s streams, decoded as their input comes (`crate::zip::Inflating`).
    reg!(it, "java.util.zip.inflaterNew", |it, a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("an inflater");
        }
        let raw = matches!(a.first(), Some(Value::Bool(true)));
        Ok(Value::Int(INFLATERS.with(|i| {
            let mut all = i.borrow_mut();
            all.push(Some(crate::zip::Inflating::new(raw)));
            (all.len() - 1) as i32
        })))
    });
    reg!(it, "java.util.zip.inflaterInput", |it, a| {
        let id = int(it, a, 0)?;
        let (off, len) = (int(it, a, 2)?, int(it, a, 3)?);
        let data = bytes(it, a, 1, Some((off as usize, len as usize)))?;
        with_inflater(it, id, |inf| {
            inf.feed(&data);
            Value::Unit
        })
    });
    // What the input decodes to now, empty when it needs more; zlib's message for a malformed
    // stream as `DataFormatException`.
    reg!(it, "java.util.zip.inflaterRun", |it, a| {
        let id = int(it, a, 0)?;
        let run = INFLATERS.with(|i| i.borrow_mut().get_mut(id as usize).and_then(|x| x.as_mut()).map(|inf| inf.run()));
        match run {
            None => it.throw_named("NullPointerException", "Inflater has been closed"),
            Some(Ok(out)) => Ok(byte_array(&out)),
            Some(Err(m)) => it.throw_new(&["java", "util", "zip", "DataFormatException"], vec![Value::str(m)]),
        }
    });
    // Whether it finished, the input it has not taken, the input it was given.
    reg!(it, "java.util.zip.inflaterInfo", |it, a| {
        let id = int(it, a, 0)?;
        with_inflater(it, id, |inf| Value::array(vec![Value::Long(inf.finished() as i64), Value::Long(inf.remaining() as i64), Value::Long(inf.fed() as i64)]))
    });
    reg!(it, "java.util.zip.inflaterReset", |it, a| {
        let id = int(it, a, 0)?;
        with_inflater(it, id, |inf| {
            inf.reset();
            Value::Unit
        })
    });
    reg!(it, "java.util.zip.inflaterEnd", |it, a| {
        let id = int(it, a, 0)?;
        INFLATERS.with(|i| {
            if let Some(slot) = i.borrow_mut().get_mut(id as usize) {
                *slot = None;
            }
        });
        Ok(Value::Unit)
    });
    reg!(it, "java.util.zip.zipRead", |it, a| {
        let id = int(it, a, 0)?;
        let index = int(it, a, 1)?;
        let read = ZIPS.with(|z| {
            let zips = z.borrow();
            zips.get(id as usize).and_then(|z| z.as_ref()).map(|zip| zip.read(index as usize))
        });
        match read {
            None => it.throw_named("IllegalStateException", "zip file closed"),
            Some(Ok(data)) => Ok(byte_array(&data)),
            Some(Err(m)) => zip_exception(it, &m),
        }
    });
    reg!(it, "java.util.zip.zipClose", |it, a| {
        let id = int(it, a, 0)?;
        ZIPS.with(|z| {
            if let Some(slot) = z.borrow_mut().get_mut(id as usize) {
                *slot = None;
            }
        });
        Ok(Value::Unit)
    });
    reg!(it, "java.security.digestNew", |it, a| {
        let name = it.str_arg(a, 0)?;
        match Digest::named(&name) {
            Some(d) => Ok(Value::Int(DIGESTS.with(|ds| {
                let mut ds = ds.borrow_mut();
                ds.push(d);
                (ds.len() - 1) as i32
            }))),
            None => it.throw_new(&["java", "security", "NoSuchAlgorithmException"], vec![Value::string(format!("{} MessageDigest not available", name))]),
        }
    });
    reg!(it, "java.security.digestUpdate", |it, a| {
        let id = int(it, a, 0)?;
        let (off, len) = (int(it, a, 2)?, int(it, a, 3)?);
        if off < 0 || len < 0 {
            return it.throw_named("IllegalArgumentException", "Bad arguments");
        }
        let data = bytes(it, a, 1, Some((off as usize, len as usize)))?;
        DIGESTS.with(|ds| ds.borrow_mut()[id as usize].update(&data));
        Ok(Value::Unit)
    });
    reg!(it, "java.security.digestFinish", |it, a| {
        let id = int(it, a, 0)?;
        Ok(byte_array(&DIGESTS.with(|ds| ds.borrow_mut()[id as usize].finish())))
    });
    // The offset of the machine's zone from UTC at an instant, in seconds.
    reg!(it, "java.util.zip.localOffset", |it, a| {
        let seconds = long(it, a, 0)?;
        if it.pure {
            return it.impure("the machine's time zone");
        }
        Ok(Value::Int(offset_at(seconds)))
    });
    // The instant, in seconds, of a local date and time in the machine's zone.
    reg!(it, "java.util.zip.localInstant", |it, a| {
        if it.pure {
            return it.impure("the machine's time zone");
        }
        let mut f = [0i32; 6];
        for (i, slot) in f.iter_mut().enumerate() {
            *slot = int(it, a, i)?;
        }
        Ok(Value::Long(local_to_epoch(f)))
    });
    // `Instant.now()`: the seconds and nanoseconds since the epoch.
    reg!(it, "java.time.timeNow", |it, _a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the clock");
        }
        let (secs, nanos) = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => (d.as_secs() as i64, d.subsec_nanos() as i64),
            Err(e) => {
                let d = e.duration();
                let (s, n) = (-(d.as_secs() as i64), d.subsec_nanos() as i64);
                if n == 0 { (s, 0) } else { (s - 1, 1_000_000_000 - n) }
            }
        };
        Ok(Value::array(vec![Value::Long(secs), Value::Long(nanos)]))
    });
    reg!(it, "java.time.timeOffset", |it, a| {
        let seconds = long(it, a, 0)?;
        if it.pure {
            return it.impure("the machine's time zone");
        }
        Ok(Value::Int(offset_at(seconds)))
    });
    reg!(it, "java.time.timeLocalInstant", |it, a| {
        if it.pure {
            return it.impure("the machine's time zone");
        }
        let mut f = [0i32; 6];
        for (i, slot) in f.iter_mut().enumerate() {
            *slot = int(it, a, i)?;
        }
        Ok(Value::Long(local_to_epoch(f)))
    });
    reg!(it, "java.time.timeZoneName", |it, _a| {
        if it.pure {
            return it.impure("the machine's time zone");
        }
        Ok(Value::string(zone_name()))
    });
    reg!(it, "java.util.zip.zoneName", |it, _a| {
        if it.pure {
            return it.impure("the machine's time zone");
        }
        Ok(Value::string(zone_name()))
    });
}

#[cfg(unix)]
mod os {
    use std::os::raw::{c_char, c_int, c_long};

    #[repr(C)]
    struct Tm {
        sec: c_int,
        min: c_int,
        hour: c_int,
        mday: c_int,
        mon: c_int,
        year: c_int,
        wday: c_int,
        yday: c_int,
        isdst: c_int,
        gmtoff: c_long,
        zone: *const c_char,
    }

    extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
        fn tzset();
    }

    pub fn offset_at(seconds: i64) -> i32 {
        static INIT: std::sync::Once = std::sync::Once::new();
        // SAFETY: tzset reads the environment once; localtime_r writes the structure given.
        unsafe {
            INIT.call_once(|| tzset());
            let mut tm: Tm = std::mem::zeroed();
            if localtime_r(&seconds, &mut tm).is_null() {
                return 0;
            }
            tm.gmtoff as i32
        }
    }

    pub fn zone_name() -> String {
        if let Ok(tz) = std::env::var("TZ") {
            let tz = tz.trim_start_matches(':');
            if !tz.is_empty() && !tz.starts_with('/') {
                return tz.to_string();
            }
        }
        if let Ok(name) = std::fs::read_to_string("/etc/timezone") {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        if let Ok(target) = std::fs::read_link("/etc/localtime") {
            let text = target.to_string_lossy();
            if let Some(at) = text.find("zoneinfo/") {
                return text[at + "zoneinfo/".len()..].to_string();
            }
        }
        "UTC".to_string()
    }
}

#[cfg(windows)]
mod os {
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct SystemTime {
        year: u16,
        month: u16,
        weekday: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn SystemTimeToTzSpecificLocalTime(zone: *const u8, utc: *const SystemTime, local: *mut SystemTime) -> i32;
    }

    pub fn offset_at(seconds: i64) -> i32 {
        let (y, m, d) = super::civil_from_days(seconds.div_euclid(86400));
        let rem = seconds.rem_euclid(86400);
        let utc = SystemTime { year: y as u16, month: m as u16, weekday: 0, day: d as u16, hour: (rem / 3600) as u16, minute: (rem / 60 % 60) as u16, second: (rem % 60) as u16, millis: 0 };
        let mut local = SystemTime::default();
        // SAFETY: the current zone (null) and two structures of the documented layout.
        if unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) } == 0 {
            return 0;
        }
        let local_secs = super::days_from_civil(local.year as i64, local.month as u32, local.day as u32) * 86400 + local.hour as i64 * 3600 + local.minute as i64 * 60 + local.second as i64;
        (local_secs - seconds) as i32
    }

    pub fn zone_name() -> String {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let off = offset_at(now);
        if off == 0 {
            return "GMT".to_string();
        }
        let sign = if off < 0 { '-' } else { '+' };
        let a = off.unsigned_abs();
        format!("GMT{}{:02}:{:02}", sign, a / 3600, a / 60 % 60)
    }
}
