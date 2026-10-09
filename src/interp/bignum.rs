//! The arbitrary-precision integers behind `std/javalib/math.scala`'s `@js` templates, which are
//! JavaScript `BigInt` operations there. A value travels as the `Str` of its decimal digits.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Big {
    neg: bool,
    /// The magnitude, least significant limb first, without leading zero limbs.
    mag: Vec<u32>,
}

fn trim(mut mag: Vec<u32>) -> Vec<u32> {
    while mag.last() == Some(&0) {
        mag.pop();
    }
    mag
}

fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry = 0u64;
    for i in 0..a.len().max(b.len()) {
        let s = *a.get(i).unwrap_or(&0) as u64 + *b.get(i).unwrap_or(&0) as u64 + carry;
        out.push(s as u32);
        carry = s >> 32;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    out
}

/// `a - b` for `a >= b`.
fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0i64;
    for i in 0..a.len() {
        let mut d = a[i] as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
        borrow = 0;
        if d < 0 {
            d += 1 << 32;
            borrow = 1;
        }
        out.push(d as u32);
    }
    trim(out)
}

fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u32; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0u64;
        for (j, &y) in b.iter().enumerate() {
            let t = out[i + j] as u64 + x as u64 * y as u64 + carry;
            out[i + j] = t as u32;
            carry = t >> 32;
        }
        out[i + b.len()] = carry as u32;
    }
    trim(out)
}

fn bit_len(a: &[u32]) -> usize {
    match a.last() {
        Some(&top) => (a.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
        None => 0,
    }
}

fn shl_mag(a: &[u32], n: usize) -> Vec<u32> {
    if a.is_empty() {
        return Vec::new();
    }
    let (words, bits) = (n / 32, n % 32);
    let mut out = vec![0u32; words];
    let mut carry = 0u32;
    for &x in a {
        if bits == 0 {
            out.push(x);
        } else {
            out.push((x << bits) | carry);
            carry = x >> (32 - bits);
        }
    }
    if carry > 0 {
        out.push(carry);
    }
    trim(out)
}

fn shr_mag(a: &[u32], n: usize) -> Vec<u32> {
    let (words, bits) = (n / 32, n % 32);
    if words >= a.len() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(a.len() - words);
    for i in words..a.len() {
        let lo = a[i] >> bits;
        let hi = if bits == 0 { 0 } else { a.get(i + 1).map_or(0, |&h| h << (32 - bits)) };
        out.push(lo | hi);
    }
    trim(out)
}

/// Quotient and remainder of the magnitudes, `b` not zero.
fn divrem_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if cmp_mag(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    if b.len() == 1 {
        let d = b[0] as u64;
        let mut q = vec![0u32; a.len()];
        let mut r = 0u64;
        for i in (0..a.len()).rev() {
            let cur = (r << 32) | a[i] as u64;
            q[i] = (cur / d) as u32;
            r = cur % d;
        }
        return (trim(q), trim(vec![r as u32]));
    }
    let n = bit_len(a);
    let mut q = vec![0u32; a.len()];
    let mut r: Vec<u32> = Vec::new();
    for i in (0..n).rev() {
        r = shl_mag(&r, 1);
        if (a[i / 32] >> (i % 32)) & 1 == 1 {
            if r.is_empty() {
                r.push(1);
            } else {
                r[0] |= 1;
            }
        }
        if cmp_mag(&r, b) != Ordering::Less {
            r = sub_mag(&r, b);
            q[i / 32] |= 1 << (i % 32);
        }
    }
    (trim(q), r)
}

impl Big {
    pub fn zero() -> Big {
        Big { neg: false, mag: Vec::new() }
    }

    fn make(neg: bool, mag: Vec<u32>) -> Big {
        let mag = trim(mag);
        Big { neg: neg && !mag.is_empty(), mag }
    }

    pub fn from_i64(v: i64) -> Big {
        let m = v.unsigned_abs();
        Big::make(v < 0, vec![m as u32, (m >> 32) as u32])
    }

    pub fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    pub fn parse(text: &str, radix: u32) -> Option<Big> {
        let t = text.trim();
        let (neg, digits) = match t.as_bytes().first() {
            Some(b'-') => (true, &t[1..]),
            Some(b'+') => (false, &t[1..]),
            _ => (false, t),
        };
        if digits.is_empty() {
            return if t.is_empty() { Some(Big::zero()) } else { None };
        }
        let mut mag: Vec<u32> = Vec::new();
        for c in digits.chars() {
            let d = c.to_digit(radix)?;
            mag = mul_mag(&mag, &[radix]);
            mag = add_mag(&mag, &[d]);
        }
        Some(Big::make(neg, mag))
    }

    pub fn to_string_radix(&self, radix: u32) -> String {
        if self.mag.is_empty() {
            return "0".to_string();
        }
        let mut digits = Vec::new();
        let mut m = self.mag.clone();
        while !m.is_empty() {
            let (q, r) = divrem_mag(&m, &[radix]);
            let d = r.first().copied().unwrap_or(0);
            digits.push(std::char::from_digit(d, radix).unwrap());
            m = q;
        }
        if self.neg {
            digits.push('-');
        }
        digits.iter().rev().collect()
    }

    /// The low 64 bits of the two's complement, as `BigInt.asIntN(64, x)`.
    pub fn to_i64_wrapping(&self) -> i64 {
        let lo = *self.mag.first().unwrap_or(&0) as u64 | (*self.mag.get(1).unwrap_or(&0) as u64) << 32;
        if self.neg {
            (lo as i64).wrapping_neg()
        } else {
            lo as i64
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.to_string_radix(10).parse().unwrap_or(f64::NAN)
    }

    pub fn neg(&self) -> Big {
        Big::make(!self.neg, self.mag.clone())
    }

    pub fn add(&self, o: &Big) -> Big {
        if self.neg == o.neg {
            return Big::make(self.neg, add_mag(&self.mag, &o.mag));
        }
        match cmp_mag(&self.mag, &o.mag) {
            Ordering::Less => Big::make(o.neg, sub_mag(&o.mag, &self.mag)),
            _ => Big::make(self.neg, sub_mag(&self.mag, &o.mag)),
        }
    }

    pub fn sub(&self, o: &Big) -> Big {
        self.add(&o.neg())
    }

    pub fn mul(&self, o: &Big) -> Big {
        Big::make(self.neg != o.neg, mul_mag(&self.mag, &o.mag))
    }

    /// Truncating quotient and the remainder with the dividend's sign; `None` for a zero divisor.
    pub fn divrem(&self, o: &Big) -> Option<(Big, Big)> {
        if o.is_zero() {
            return None;
        }
        let (q, r) = divrem_mag(&self.mag, &o.mag);
        Some((Big::make(self.neg != o.neg, q), Big::make(self.neg, r)))
    }

    pub fn pow(&self, mut n: u32) -> Big {
        let mut result = Big::from_i64(1);
        let mut base = self.clone();
        while n > 0 {
            if n & 1 == 1 {
                result = result.mul(&base);
            }
            base = base.mul(&base);
            n >>= 1;
        }
        result
    }

    pub fn shl(&self, n: i32) -> Big {
        if n < 0 {
            return self.shr(-n);
        }
        Big::make(self.neg, shl_mag(&self.mag, n as usize))
    }

    /// The floor of `x / 2^n`, as JavaScript's `>>` of a `BigInt`.
    pub fn shr(&self, n: i32) -> Big {
        if n < 0 {
            return self.shl(-n);
        }
        let q = shr_mag(&self.mag, n as usize);
        let exact = shl_mag(&q, n as usize) == self.mag;
        if self.neg && !exact {
            Big::make(true, add_mag(&q, &[1]))
        } else {
            Big::make(self.neg, q)
        }
    }

    /// The two's complement in `words` limbs.
    fn twos(&self, words: usize) -> Vec<u32> {
        let mut out = self.mag.clone();
        out.resize(words, 0);
        if self.neg {
            for w in out.iter_mut() {
                *w = !*w;
            }
            let mut i = 0;
            while i < words {
                let (s, c) = out[i].overflowing_add(1);
                out[i] = s;
                if !c {
                    break;
                }
                i += 1;
            }
        }
        out
    }

    fn of_twos(mut words: Vec<u32>) -> Big {
        let neg = words.last().map_or(false, |&w| w >> 31 == 1);
        if neg {
            for w in words.iter_mut() {
                *w = !*w;
            }
            let mag = add_mag(&trim(words), &[1]);
            return Big::make(true, mag);
        }
        Big::make(false, words)
    }

    pub fn bitwise(&self, o: &Big, f: impl Fn(u32, u32) -> u32) -> Big {
        let words = self.mag.len().max(o.mag.len()) + 1;
        let (a, b) = (self.twos(words), o.twos(words));
        Big::of_twos(a.iter().zip(b.iter()).map(|(&x, &y)| f(x, y)).collect())
    }

    pub fn not(&self) -> Big {
        self.neg().sub(&Big::from_i64(1))
    }

    pub fn cmp(&self, o: &Big) -> Ordering {
        match (self.neg, o.neg) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_mag(&self.mag, &o.mag),
            (true, true) => cmp_mag(&o.mag, &self.mag),
        }
    }

    /// The JDK's `BigInteger.hashCode`: the magnitude's words, most significant first, times the sign.
    pub fn jdk_hash(&self) -> i32 {
        let mut h: i32 = 0;
        for &w in self.mag.iter().rev() {
            h = h.wrapping_mul(31).wrapping_add(w as i32);
        }
        if self.neg {
            h.wrapping_neg()
        } else {
            h
        }
    }

    pub fn lowest_set_bit(&self) -> i32 {
        for (i, &w) in self.mag.iter().enumerate() {
            if w != 0 {
                return (i * 32) as i32 + w.trailing_zeros() as i32;
            }
        }
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> Big {
        Big::parse(s, 10).unwrap()
    }

    #[test]
    fn arithmetic() {
        let a = b("123456789012345678901234567890");
        let c = b("-987654321");
        assert_eq!(a.mul(&c).to_string_radix(10), "-121932631124828532112482853211126352690");
        let (q, r) = a.divrem(&c).unwrap();
        assert_eq!(q.to_string_radix(10), "-124999998873437499901");
        assert_eq!(r.to_string_radix(10), "574845669");
        assert_eq!(b("-1000").shr(3).to_string_radix(10), "-125");
        assert_eq!(b("-6").bitwise(&b("13"), |x, y| x & y).to_string_radix(10), "8");
        assert_eq!(b("-6").bitwise(&b("13"), |x, y| x | y).to_string_radix(10), "-1");
        assert_eq!(b("-6").bitwise(&b("13"), |x, y| x ^ y).to_string_radix(10), "-9");
        assert_eq!(b("5").not().to_string_radix(10), "-6");
        assert_eq!(b("-255").to_string_radix(16), "-ff");
        assert_eq!(b("-1").to_i64_wrapping(), -1);
        assert_eq!(b("18446744073709551617").to_i64_wrapping(), 1);
    }
}
