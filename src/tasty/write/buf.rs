//! The byte layer of the writer: TASTy's numbers, and a tree buffer whose lengths and
//! addresses are written wide and made as short as they can be once everything is written.
//!
//! A tree's length precedes the tree and a reference may point forward, so neither is known
//! when its place is written. Both are reserved as four bytes and filled in later, as scalac's
//! `TreeBuffer` does; `finish` then gives every reserved field the fewest bytes its value needs,
//! which moves what follows it, until no field needs more than it has.

/// The widest field a length or a reference is reserved as: base 128 in four digits holds
/// every address up to 256 MB.
const WIDE: usize = 4;

pub fn nat_size(mut x: u64) -> usize {
    let mut n = 1;
    while x >= 128 {
        x >>= 7;
        n += 1;
    }
    n
}

/// Big endian base 128, the last digit marked by its high bit.
pub fn write_nat(out: &mut Vec<u8>, x: u64) {
    let n = nat_size(x);
    for i in (0..n).rev() {
        let digit = ((x >> (7 * i)) & 0x7f) as u8;
        out.push(if i == 0 { digit | 0x80 } else { digit });
    }
}

/// `x` in exactly `width` digits, leading zero digits first.
fn write_nat_wide(out: &mut [u8], x: u64, width: usize) {
    for i in 0..width {
        let shift = 7 * (width - 1 - i);
        let digit = ((x >> shift) & 0x7f) as u8;
        out[i] = if i == width - 1 { digit | 0x80 } else { digit };
    }
}

/// Two's complement in base 128, big endian, as few digits as keep the sign.
pub fn write_long_int(out: &mut Vec<u8>, x: i64) {
    let mut digits = [0u8; 10];
    let mut len = 0;
    let mut v = x;
    loop {
        digits[len] = (v & 0x7f) as u8;
        len += 1;
        let rest = v >> 7;
        let sign_ok = if rest == 0 { v & 0x40 == 0 } else if rest == -1 { v & 0x40 != 0 } else { false };
        v = rest;
        if sign_ok {
            break;
        }
    }
    for i in (0..len).rev() {
        out.push(if i == 0 { digits[i] | 0x80 } else { digits[i] });
    }
}

#[derive(Clone, Copy)]
enum Field {
    /// The length of the tree that follows the field and ends at `end`.
    Length { end: usize },
    /// The address of a tree, which may not be written yet (`usize::MAX` until it is).
    Ref { target: usize },
}

/// A tree buffer: the bytes in their wide form and the reserved fields.
#[derive(Default)]
pub struct TreeBuf {
    pub bytes: Vec<u8>,
    fields: Vec<(usize, Field)>,
}

/// A reserved field, filled once what it names is known.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slot(usize);

/// Where each address of the wide form lands in the final one.
pub struct Moves {
    /// Per field in order, its place in the wide form and how many bytes the final form saved
    /// up to and including it.
    at: Vec<usize>,
    saved: Vec<usize>,
}

impl Moves {
    /// The addresses `addrs` hold, in ascending order, mapped in place by one pass over the
    /// fields.
    pub fn map_ascending<'a>(&self, addrs: impl Iterator<Item = &'a mut usize>) {
        let mut before = 0;
        for addr in addrs {
            while before < self.at.len() && self.at[before] < *addr {
                before += 1;
            }
            if before > 0 {
                *addr -= self.saved[before - 1];
            }
        }
    }

    pub fn map(&self, addr: usize) -> usize {
        // The fields that start before `addr` moved it back by what they saved.
        let n = self.at.partition_point(|&a| a < addr);
        if n == 0 {
            addr
        } else {
            addr - self.saved[n - 1]
        }
    }
}

impl TreeBuf {
    #[inline]
    pub fn addr(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    pub fn byte(&mut self, b: u8) {
        self.bytes.push(b);
    }

    #[inline]
    pub fn nat(&mut self, x: u64) {
        write_nat(&mut self.bytes, x);
    }

    #[inline]
    pub fn long_int(&mut self, x: i64) {
        write_long_int(&mut self.bytes, x);
    }

    fn reserve(&mut self, f: Field) -> Slot {
        let at = self.bytes.len();
        self.fields.push((at, f));
        self.bytes.extend_from_slice(&[0; WIDE]);
        Slot(self.fields.len() - 1)
    }

    /// Reserves the length of the tree that follows; `end_length` closes it.
    pub fn begin_length(&mut self) -> Slot {
        self.reserve(Field::Length { end: usize::MAX })
    }

    pub fn end_length(&mut self, s: Slot) {
        let end = self.bytes.len();
        self.fields[s.0].1 = Field::Length { end };
    }

    /// A reference to a tree written before.
    pub fn reference(&mut self, target: usize) {
        self.reserve(Field::Ref { target });
    }

    /// A reference to a tree not written yet; `fill` names it once it is.
    pub fn forward_reference(&mut self) -> Slot {
        self.reserve(Field::Ref { target: usize::MAX })
    }

    pub fn fill(&mut self, s: Slot, target: usize) {
        self.fields[s.0].1 = Field::Ref { target };
    }

    /// How much is written, for `truncate`.
    pub fn mark(&self) -> (usize, usize) {
        (self.bytes.len(), self.fields.len())
    }

    /// Drops what was written since `mark`: its bytes and its reserved fields.
    pub fn truncate(&mut self, (bytes, fields): (usize, usize)) {
        self.bytes.truncate(bytes);
        self.fields.truncate(fields);
    }

    /// Whether a slot was reserved before `mark`.
    pub fn slot_before(&self, s: Slot, (_, fields): (usize, usize)) -> bool {
        s.0 < fields
    }

    /// Per field, the fields before the end of the tree a length field measures: the trees
    /// nest, so that a pass with the lengths open finds them; a field that is no length's, or
    /// a length that does not nest, by a search.
    fn length_ends(&self, at: &[usize]) -> Vec<usize> {
        let n = self.fields.len();
        let mut ends = vec![usize::MAX; n];
        let mut open: Vec<(usize, usize)> = Vec::new();
        for (i, &(a, f)) in self.fields.iter().enumerate() {
            while let Some(&(end, j)) = open.last() {
                if end > a {
                    break;
                }
                ends[j] = i;
                open.pop();
            }
            if let Field::Length { end } = f {
                open.push((end, i));
            }
        }
        for (_, j) in open {
            ends[j] = n;
        }
        for (i, &(_, f)) in self.fields.iter().enumerate() {
            if let Field::Length { end } = f {
                let k = ends[i];
                let nests = k <= n && (k == 0 || at[k - 1] < end) && (k == n || at[k] >= end);
                if !nests {
                    ends[i] = at.partition_point(|&a| a < end);
                }
            }
        }
        ends
    }

    /// The final bytes, every field as short as its value allows, and where the wide form's
    /// addresses moved. The widths grow from one byte to the least that holds every value: in a
    /// round, the lengths from the last field to the first, each of them once the fields it
    /// spans have theirs, then the references, which a round repeats while one of them grows.
    pub fn finish(self) -> (Vec<u8>, Moves) {
        let n = self.fields.len();
        let at: Vec<usize> = self.fields.iter().map(|(a, _)| *a).collect();
        // Per field, the address its value names in the wide form, the fields before it, and
        // whether the value is the length up to it.
        let ends = self.length_ends(&at);
        let named: Vec<(usize, usize, bool)> = self
            .fields
            .iter()
            .zip(ends)
            .map(|(&(_, f), before_end)| match f {
                Field::Length { end } => (end, before_end, true),
                Field::Ref { target } => (target, at.partition_point(|&a| a < target), false),
            })
            .collect();
        let mut width = vec![1usize; n];
        // What the fields from the i-th on save.
        let mut after = vec![0usize; n + 1];
        loop {
            after[n] = 0;
            for i in (0..n).rev() {
                let (end, before_end, length) = named[i];
                if length {
                    let inside = after[i + 1] - after[before_end];
                    width[i] = width[i].max(nat_size((end - (at[i] + WIDE) - inside) as u64));
                }
                after[i] = after[i + 1] + (WIDE - width[i]);
            }
            let mut grew = false;
            for i in 0..n {
                let (target, before, length) = named[i];
                if !length {
                    let need = nat_size((target - (after[0] - after[before])) as u64);
                    if need > width[i] {
                        width[i] = need;
                        grew = true;
                    }
                }
            }
            if !grew {
                break;
            }
        }
        let saved: Vec<usize> = (0..n).map(|i| after[0] - after[i + 1]).collect();
        let mut out = Vec::with_capacity(self.bytes.len());
        let mut from = 0;
        for i in 0..n {
            out.extend_from_slice(&self.bytes[from..at[i]]);
            let (addr, before, length) = named[i];
            let v = if length { addr - (at[i] + WIDE) - (after[i + 1] - after[before]) } else { addr - (after[0] - after[before]) };
            let start = out.len();
            out.resize(start + width[i], 0);
            write_nat_wide(&mut out[start..], v as u64, width[i]);
            from = at[i] + WIDE;
        }
        out.extend_from_slice(&self.bytes[from..]);
        (out, Moves { at, saved })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasty::Reader;

    #[test]
    fn numbers_read_back() {
        for x in [0u64, 1, 127, 128, 16383, 16384, 1 << 28, u32::MAX as u64] {
            let mut out = Vec::new();
            write_nat(&mut out, x);
            assert_eq!(Reader::new(&out).long_nat(), x);
        }
        for x in [0i64, 1, -1, 63, 64, -64, -65, 8191, -8192, i32::MAX as i64, i32::MIN as i64, i64::MAX, i64::MIN] {
            let mut out = Vec::new();
            write_long_int(&mut out, x);
            assert_eq!(Reader::new(&out).long_int(), x, "{}", x);
        }
    }

    #[test]
    fn lengths_and_forward_references_shrink() {
        let mut b = TreeBuf::default();
        let outer = b.begin_length();
        let fwd = b.forward_reference();
        for _ in 0..200 {
            b.byte(7);
        }
        let target = b.addr();
        b.byte(9);
        b.fill(fwd, target);
        b.end_length(outer);
        b.reference(target);
        let (out, moves) = b.finish();
        let mut r = Reader::new(&out);
        let end = r.end();
        let t = r.nat() as usize;
        assert_eq!(out[t], 9);
        assert_eq!(moves.map(target), t);
        r.pos = end;
        assert_eq!(r.nat() as usize, t);
        assert!(r.at_end());
    }
}
