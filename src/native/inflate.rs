//! zlib 1.3.1's inflate of a raw deflate stream, as the JDK 24 bundles it (src/java.base/share/native/libzip/zlib:
//! inflate.c, inftrees.c, inffast.c) and the JDK's zip file system runs it to copy a deflated entry: the bytes of every
//! stream zlib takes, and for every stream it refuses its refusal with zlib's message. A refusal and a request for more
//! input fall where zlib's do, so that a stream cut short is short where zlib asks for more input, not refused. The
//! class path read's inflater (src/zip.rs) is laxer, taking an incomplete code; the copy runs this one.

/// Why the stream does not inflate: zlib's message for a stream it refuses, or more input asked for after the last
/// byte.
#[derive(Debug, PartialEq)]
pub enum Stop {
    Refused(&'static str),
    Short,
}

type Res<T> = Result<T, Stop>;

/// The input as zlib's bit buffer reads it: low bits of each byte first.
struct Input<'a> {
    data: &'a [u8],
    bit: usize,
}

impl Input<'_> {
    /// `NEEDBITS(n)`: `n` bits at hand, else more input asked for.
    fn need(&self, n: usize) -> Res<()> {
        if self.bit + n <= self.data.len() * 8 {
            Ok(())
        } else {
            Err(Stop::Short)
        }
    }

    fn take(&mut self, n: usize) -> Res<usize> {
        self.need(n)?;
        let mut value = 0;
        for i in 0..n {
            let at = self.bit + i;
            value |= ((self.data[at / 8] >> (at % 8)) as usize & 1) << i;
        }
        self.bit += n;
        Ok(value)
    }

    /// `BYTEBITS()`: the bits left of the byte under way dropped.
    fn to_byte(&mut self) {
        self.bit = self.bit.div_ceil(8) * 8;
    }
}

/// A code's longest length.
const MAX_BITS: usize = 15; // RFC 1951 3.2.7

/// The three tables `inflate_table` builds (inftrees.h's `codetype`).
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Codes,
    Lens,
    Dists,
}

/// A canonical code as `inflate_table` takes it (inftrees.c 124-157): none for an over-subscribed set of lengths, nor
/// for an incomplete one but a single code of one bit in a literal/length or distance table; for a set of no lengths
/// at all, a table that holds no code (`empty`).
struct Code {
    kind: Kind,
    count: [usize; 16],
    symbols: Vec<u16>,
    empty: bool,
    incomplete: bool,
}

impl Code {
    fn new(lens: &[u16], kind: Kind) -> Option<Code> {
        let mut count = [0usize; 16];
        for &len in lens {
            count[len as usize] += 1;
        }
        let max = (1..=MAX_BITS).rev().find(|&len| count[len] != 0).unwrap_or(0);
        if max == 0 {
            return Some(Code { kind, count, symbols: Vec::new(), empty: true, incomplete: false });
        }
        let mut left: i64 = 1;
        for &n in &count[1..] {
            left = (left << 1) - n as i64;
            if left < 0 {
                return None;
            }
        }
        if left > 0 && (kind == Kind::Codes || max != 1) {
            return None;
        }
        let mut offs = [0usize; 16];
        for len in 1..MAX_BITS {
            offs[len + 1] = offs[len] + count[len];
        }
        let mut symbols = vec![0u16; lens.iter().filter(|&&len| len != 0).count()];
        for (sym, &len) in lens.iter().enumerate() {
            if len != 0 {
                symbols[offs[len as usize]] = sym as u16;
                offs[len as usize] += 1;
            }
        }
        Some(Code { kind, count, symbols, empty: false, incomplete: left > 0 })
    }

    /// The next symbol as zlib's table gives it, its code's bits taken (each needing to be at hand); none for an entry
    /// the table marks invalid, one bit taken: the missing code of an incomplete single code, and every code of an
    /// empty table but the code-length table's, whose entries read as length 0 (inflate.c 961-967 reads the value of
    /// an invalid entry there).
    fn decode(&self, input: &mut Input) -> Res<Option<u16>> {
        if self.empty {
            input.take(1)?;
            return Ok((self.kind == Kind::Codes).then_some(0));
        }
        let (mut code, mut first, mut index) = (0usize, 0usize, 0usize);
        for len in 1..=MAX_BITS {
            code |= input.take(1)?;
            let count = self.count[len];
            if code < first + count {
                return Ok(Some(self.symbols[index + code - first]));
            }
            if self.incomplete {
                return Ok(None);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Ok(None)
    }
}

/// The order of the code length code's lengths (inflate.c `order`).
const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
/// Length codes 257..285: base and extra bits (inftrees.c `lbase`, `lext`).
const LBASE: [usize; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEXT: [usize; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
/// Distance codes 0..29: base and extra bits (`dbase`, `dext`).
const DBASE: [usize; 30] =
    [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DEXT: [usize; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// A raw deflate stream inflated as zlib inflates it, block after block to the last (inflate.c 851-1209): its bytes,
/// or where zlib stops. What follows the last block is not read.
pub fn inflate(data: &[u8]) -> Res<Vec<u8>> {
    let mut input = Input { data, bit: 0 };
    let mut out = Vec::new();
    loop {
        input.need(3)?;
        let last = input.take(1)? == 1;
        match input.take(2)? {
            0 => stored(&mut input, &mut out)?,
            1 => {
                let lit = Code::new(&fixed_lens(), Kind::Lens).expect("the fixed literal/length code");
                let dist = Code::new(&[5u16; 32], Kind::Dists).expect("the fixed distance code");
                codes(&mut input, &mut out, &lit, &dist)?;
            }
            2 => {
                let (lit, dist) = table(&mut input)?;
                codes(&mut input, &mut out, &lit, &dist)?;
            }
            _ => return Err(Stop::Refused("invalid block type")),
        }
        if last {
            return Ok(out);
        }
    }
}

/// The fixed literal/length code's lengths.
fn fixed_lens() -> [u16; 288] {
    let mut lens = [8u16; 288];
    lens[144..256].fill(9); // RFC 1951 3.2.6
    lens[256..280].fill(7); // RFC 1951 3.2.6
    lens
}

/// A stored block (inflate.c 886-918): its length and that length's complement, then its bytes.
fn stored(input: &mut Input, out: &mut Vec<u8>) -> Res<()> {
    input.to_byte();
    input.need(32)?;
    let length = input.take(16)?; // RFC 1951 3.2.4: LEN
    if length != input.take(16)? ^ 0xffff { // NLEN, RFC 1951 3.2.4
        return Err(Stop::Refused("invalid stored block lengths"));
    }
    let from = input.bit / 8;
    let bytes = input.data.get(from..from + length).ok_or(Stop::Short)?;
    out.extend_from_slice(bytes);
    input.bit += length * 8;
    Ok(())
}

/// A dynamic block's codes (inflate.c 919-1046): the counts, the code length code, the lengths it codes, then the
/// literal/length and distance codes they make.
fn table(input: &mut Input) -> Res<(Code, Code)> {
    input.need(14)?;
    let nlen = input.take(5)? + 257; // RFC 1951 3.2.7: HLIT
    let ndist = input.take(5)? + 1; // HDIST, RFC 1951 3.2.7
    let ncode = input.take(4)? + 4; // HCLEN, RFC 1951 3.2.7
    if nlen > 286 || ndist > 30 {
        return Err(Stop::Refused("too many length or distance symbols"));
    }
    let mut lens = [0u16; 19];
    for &at in &ORDER[..ncode] {
        lens[at] = input.take(3)? as u16; // RFC 1951 3.2.7
    }
    let lengths = Code::new(&lens, Kind::Codes).ok_or(Stop::Refused("invalid code lengths set"))?;
    let mut lens = vec![0u16; nlen + ndist];
    let mut have = 0;
    while have < nlen + ndist {
        let symbol = lengths.decode(input)?.unwrap_or(0);
        if symbol < 16 {
            lens[have] = symbol;
            have += 1;
            continue;
        }
        let (len, copy) = match symbol {
            16 => {
                input.need(2)?;
                if have == 0 {
                    return Err(Stop::Refused("invalid bit length repeat"));
                }
                (lens[have - 1], 3 + input.take(2)?)
            }
            17 => (0, 3 + input.take(3)?), // RFC 1951 3.2.7
            _ => (0, 11 + input.take(7)?), // RFC 1951 3.2.7
        };
        if have + copy > nlen + ndist {
            return Err(Stop::Refused("invalid bit length repeat"));
        }
        lens[have..have + copy].fill(len);
        have += copy;
    }
    if lens[256] == 0 {
        return Err(Stop::Refused("invalid code -- missing end-of-block"));
    }
    let lit = Code::new(&lens[..nlen], Kind::Lens).ok_or(Stop::Refused("invalid literal/lengths set"))?;
    let dist = Code::new(&lens[nlen..], Kind::Dists).ok_or(Stop::Refused("invalid distances set"))?;
    Ok((lit, dist))
}

/// A block's literals and matches to its end-of-block code (inflate.c 1050-1209, inffast.c): a symbol of 286 or 287
/// and a distance of 30 or 31 are codes the fixed tables mark invalid; a distance past what has been written is too
/// far back (the window, 32 KiB, holds all of it until then, and no distance is longer).
fn codes(input: &mut Input, out: &mut Vec<u8>, lit: &Code, dist: &Code) -> Res<()> {
    loop {
        let symbol = match lit.decode(input)? {
            Some(symbol) if symbol < 286 => symbol as usize,
            _ => return Err(Stop::Refused("invalid literal/length code")),
        };
        if symbol < 256 {
            out.push(symbol as u8);
            continue;
        }
        if symbol == 256 {
            return Ok(());
        }
        let length = LBASE[symbol - 257] + input.take(LEXT[symbol - 257])?;
        let code = match dist.decode(input)? {
            Some(code) if code < 30 => code as usize,
            _ => return Err(Stop::Refused("invalid distance code")),
        };
        let distance = DBASE[code] + input.take(DEXT[code])?;
        if distance > out.len() {
            return Err(Stop::Refused("invalid distance too far back"));
        }
        let from = out.len() - distance;
        for i in 0..length {
            out.push(out[from + i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deflate stream written bit by bit: numbers low bits first, Huffman codes high bits first.
    #[derive(Default)]
    struct Bits {
        out: Vec<u8>,
        n: usize,
    }

    impl Bits {
        fn put(&mut self, value: usize, len: usize) -> &mut Self {
            for i in 0..len {
                if self.n % 8 == 0 {
                    self.out.push(0);
                }
                self.out[self.n / 8] |= (((value >> i) & 1) as u8) << (self.n % 8);
                self.n += 1;
            }
            self
        }

        fn code(&mut self, code: usize, len: usize) -> &mut Self {
            for i in (0..len).rev() {
                self.put((code >> i) & 1, 1);
            }
            self
        }
    }

    /// Each symbol's canonical code for a set of lengths.
    fn canonical(lens: &[u16]) -> Vec<usize> {
        let mut count = [0usize; 16];
        lens.iter().for_each(|&l| count[l as usize] += 1);
        count[0] = 0;
        let mut next = [0usize; 16];
        let mut code = 0;
        for len in 1..=MAX_BITS {
            code = (code + count[len - 1]) << 1;
            next[len] = code;
        }
        lens.iter().map(|&l| if l == 0 { 0 } else { let c = next[l as usize]; next[l as usize] += 1; c }).collect()
    }

    /// A last dynamic block of `lit` and `dist` lengths, written with a code length code of 4 bits for each of the
    /// lengths 0 to 15, then the literals of `text` and the end of the block.
    fn dynamic(lit: &[u16], dist: &[u16], text: &[u8]) -> Vec<u8> {
        let codes = canonical(lit);
        dynamic_then(lit, dist, &|b: &mut Bits| {
            for &c in text {
                b.code(codes[c as usize], lit[c as usize] as usize);
            }
            b.code(codes[256], lit[256] as usize);
        })
    }

    /// The same block's header and codes, then what `data` writes.
    fn dynamic_then(lit: &[u16], dist: &[u16], data: &dyn Fn(&mut Bits)) -> Vec<u8> {
        let mut b = Bits::default();
        b.put(1, 1).put(2, 2).put(lit.len() - 257, 5).put(dist.len() - 1, 5).put(15, 4);
        let mut cl = [0u16; 19];
        cl[..16].fill(4); // the lengths 0 to 15 of RFC 1951 3.2.7
        for &at in &ORDER {
            b.put(cl[at] as usize, 3);
        }
        let clc = canonical(&cl);
        for &len in lit.iter().chain(dist) {
            b.code(clc[len as usize], 4);
        }
        data(&mut b);
        b.out
    }

    fn complete_lit() -> Vec<u16> {
        let mut lit = vec![9u16; 265];
        lit[256..265].copy_from_slice(&[2, 3, 4, 5, 6, 7, 8, 9, 9]); // the end of a block, lengths 3 to 10 (RFC 1951 3.2.5)
        lit
    }

    /// zlib's acceptance of a code's lengths (inftrees.c 148-157), and what its tables give the stream.
    #[test]
    fn inflates_and_refuses_as_zlib_does() {
        let dist = vec![4u16; 16];
        assert_eq!(inflate(&dynamic(&complete_lit(), &dist, b"int x;\n")), Ok(b"int x;\n".to_vec()));
        // The review's stream: 15 codes of 4 bits, an incomplete literal/length code.
        let mut lit = vec![0u16; 257];
        lit[..14].fill(4); // with the end of a block, 15 codes of 4 bits: RFC 1951 3.2.2 leaves one free
        lit[256] = 4;
        assert_eq!(inflate(&dynamic(&lit, &dist, b"")), Err(Stop::Refused("invalid literal/lengths set")));
        // A single code of one bit: taken, its missing code refused when read.
        let mut lit = vec![0u16; 257];
        lit[256] = 1;
        assert_eq!(inflate(&dynamic(&lit, &dist, b"")), Ok(Vec::new()));
        assert_eq!(inflate(&dynamic_then(&lit, &dist, &|b: &mut Bits| { b.put(1, 1); })), Err(Stop::Refused("invalid literal/length code")));
        // An incomplete distance code, and an empty one: refused, and taken where no distance is read.
        assert_eq!(inflate(&dynamic(&complete_lit(), &[2, 2, 2], b"a")), Err(Stop::Refused("invalid distances set")));
        assert_eq!(inflate(&dynamic(&complete_lit(), &[0], b"a")), Ok(b"a".to_vec()));
        // No end-of-block code; too many symbols; a bad block type; stored lengths.
        let mut lit = vec![8u16; 257];
        lit[256] = 0;
        assert_eq!(inflate(&dynamic(&lit, &dist, b"")), Err(Stop::Refused("invalid code -- missing end-of-block")));
        assert_eq!(inflate(&[0b111]), Err(Stop::Refused("invalid block type")));
        assert_eq!(inflate(&[0b001, 1, 0, 0xfe, 0xff, b'z']), Ok(b"z".to_vec()));
        assert_eq!(inflate(&[0b001, 1, 0, 0xff, 0xff, b'z']), Err(Stop::Refused("invalid stored block lengths")));
        assert_eq!(inflate(&[0b001, 2, 0, 0xfd, 0xff, b'z']), Err(Stop::Short));
        let mut b = Bits::default();
        b.put(1, 1).put(2, 2).put(30, 5).put(0, 5).put(0, 4).put(0, 24);
        assert_eq!(inflate(&b.out), Err(Stop::Refused("too many length or distance symbols")));
    }

    /// A distance as far back as what has been written is taken, one further refused; fixed blocks' symbols 286 and
    /// 287 and distances 30 and 31 refused.
    #[test]
    fn refuses_what_the_fixed_tables_mark_and_a_distance_too_far_back() {
        let lit = canonical(&fixed_lens());
        let fixed = |body: &dyn Fn(&mut Bits)| {
            let mut b = Bits::default();
            b.put(1, 1).put(1, 2);
            body(&mut b);
            b.code(lit[256], 7);
            b.out
        };
        let back = |distance: usize| {
            fixed(&|b: &mut Bits| {
                b.code(lit[b'a' as usize], 8).code(lit[b'b' as usize], 8);
                b.code(lit[257], 7).code(distance - 1, 5);
            })
        };
        assert_eq!(inflate(&back(2)), Ok(b"ababa".to_vec()));
        assert_eq!(inflate(&back(3)), Err(Stop::Refused("invalid distance too far back")));
        assert_eq!(inflate(&fixed(&|b: &mut Bits| { b.code(lit[286], 8); })), Err(Stop::Refused("invalid literal/length code")));
        let far = fixed(&|b: &mut Bits| {
            b.code(lit[b'a' as usize], 8).code(lit[257], 7).code(30, 5);
        });
        assert_eq!(inflate(&far), Err(Stop::Refused("invalid distance code")));
    }
}
