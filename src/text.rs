//! Lone surrogates in string literals. A `str` cannot hold an unpaired UTF-16 surrogate, so a
//! literal's `"\uD800"` is kept as one character of the last 2048 code points of Plane 16's
//! private use area, `U+10F800 + (unit - 0xD800)`; the places that speak UTF-16 units (the
//! JavaScript literal writer, the class file's modified UTF-8, the interpreter's string views)
//! read it back as the one unit it stands for.
//!
//! The form is canonical: a high surrogate followed by a low one is the supplementary character
//! they spell, never two stand-ins, so two strings of the same units have the same bytes. What
//! joins strings or builds one from units keeps it so: `push_str`, `push_unit` and `from_units`
//! pair a high stand-in with a low one that comes to stand after it. A pair that spells a code
//! point of the stand-ins' own range (a high surrogate of `U+DBFE` or `U+DBFF`) stays two
//! stand-ins, since the character it spells would read back as a lone surrogate (`joins`).

const BASE: u32 = 0x10F800;

pub fn lone_surrogate(unit: u32) -> char {
    char::from_u32(BASE + (unit - 0xD800)).unwrap()
}

/// The surrogate unit `c` stands for.
#[inline]
pub fn surrogate_of(c: char) -> Option<u16> {
    let c = c as u32;
    (c >= BASE).then(|| (0xD800 + (c - BASE)) as u16)
}

/// The UTF-16 units of `s`, a stand-in counting as its one surrogate.
pub fn utf16_units(s: &str) -> Units<'_> {
    Units { chars: s.chars(), low: None }
}

pub struct Units<'s> {
    chars: std::str::Chars<'s>,
    low: Option<u16>,
}

impl Iterator for Units<'_> {
    type Item = u16;

    #[inline]
    fn next(&mut self) -> Option<u16> {
        if let Some(u) = self.low.take() {
            return Some(u);
        }
        let c = self.chars.next()?;
        let code = c as u32;
        if code < 0x10000 {
            return Some(code as u16);
        }
        if let Some(u) = surrogate_of(c) {
            return Some(u);
        }
        let v = code - 0x10000;
        self.low = Some(0xDC00 + (v & 0x3FF) as u16);
        Some(0xD800 + (v >> 10) as u16)
    }
}

/// The number of UTF-16 units `c` takes.
#[inline]
pub fn len_utf16(c: char) -> usize {
    if surrogate_of(c).is_some() { 1 } else { c.len_utf16() }
}

/// The character a unit is on its own: itself, or a surrogate's stand-in.
#[inline]
pub fn unit_char(unit: u16) -> char {
    match unit {
        0xD800..=0xDFFF => lone_surrogate(unit as u32),
        _ => char::from_u32(unit as u32).unwrap(),
    }
}

/// A high surrogate's stand-in is `U+10F800..U+10FBFF`, `F4 8F A0..AF xx` in UTF-8, a low one's
/// `U+10FC00..U+10FFFF`, `F4 8F B0..BF xx`.
#[inline]
fn is_stand_in(b: &[u8], high: bool) -> bool {
    let (lo, hi) = if high { (0xA0, 0xAF) } else { (0xB0, 0xBF) };
    b.len() >= 4 && b[0] == 0xF4 && b[1] == 0x8F && (lo..=hi).contains(&b[2])
}

#[inline]
pub fn starts_with_low(s: &str) -> bool {
    is_stand_in(s.as_bytes(), false)
}

#[inline]
pub fn ends_with_high(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 4 && is_stand_in(&b[b.len() - 4..], true)
}

/// Whether a high stand-in joins a low one that follows it.
#[inline]
fn joins(high: char) -> bool {
    surrogate_of(high).is_some_and(|u| u < 0xDBFE)
}

/// Whether a high and a low surrogate spell a character of their own: one below the stand-ins.
#[inline]
pub fn pair_joins(high: u32) -> bool {
    (0xD800..0xDBFE).contains(&high)
}

/// The string of a code point: the character, the stand-in of a surrogate, and the two stand-ins
/// of a code point in the stand-ins' own range, whose character would read back as one.
pub fn code_point(cp: u32) -> Option<String> {
    match cp {
        0xD800..=0xDFFF => Some(lone_surrogate(cp).to_string()),
        0x10F800..=0x10FFFF => {
            let v = cp - 0x10000;
            Some(from_units([(0xD800 + (v >> 10)) as u16, (0xDC00 + (v & 0x3FF)) as u16]))
        }
        _ => char::from_u32(cp).map(|c| c.to_string()),
    }
}

/// The supplementary character of a high and a low stand-in.
fn paired(high: char, low: char) -> char {
    let (h, l) = (surrogate_of(high).unwrap() as u32, surrogate_of(low).unwrap() as u32);
    char::from_u32(0x10000 + ((h - 0xD800) << 10) + (l - 0xDC00)).unwrap()
}

/// Appends `part`, pairing a high stand-in at the end of `out` with a low one at its start.
#[inline]
pub fn push_str(out: &mut String, part: &str) {
    if starts_with_low(part) && ends_with_high(out) && out.chars().next_back().is_some_and(joins) {
        let high = out.pop().unwrap();
        let mut rest = part.chars();
        let low = rest.next().unwrap();
        out.push(paired(high, low));
        out.push_str(rest.as_str());
    } else {
        out.push_str(part);
    }
}

/// Appends one unit, as `push_str` appends a string.
#[inline]
pub fn push_unit(out: &mut String, unit: u16) {
    match unit {
        0xDC00..=0xDFFF if ends_with_high(out) && out.chars().next_back().is_some_and(joins) => {
            let high = out.pop().unwrap();
            out.push(paired(high, lone_surrogate(unit as u32)));
        }
        _ => out.push(unit_char(unit)),
    }
}

/// The string of these units.
pub fn from_units(units: impl IntoIterator<Item = u16>) -> String {
    let mut out = String::new();
    for u in units {
        push_unit(&mut out, u);
    }
    out
}

/// `s` in the canonical form, for a string whose parts were put together without `push_str`.
pub fn canonical(s: String) -> String {
    if !s.as_bytes().contains(&0xF4) {
        return s;
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match surrogate_of(c) {
            Some(u) => push_unit(&mut out, u),
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_pair_again() {
        let (hi, lo) = (lone_surrogate(0xD834).to_string(), lone_surrogate(0xDD1E).to_string());
        let mut s = format!("a{}", hi);
        push_str(&mut s, &format!("{}b", lo));
        assert_eq!(s, "a\u{1D11E}b");
        assert_eq!(from_units([0x61, 0xD834, 0xDD1E, 0xDD1E]), format!("a\u{1D11E}{}", lo));
        assert_eq!(canonical(format!("{}{}{}", lo, hi, lo)), format!("{}\u{1D11E}", lo));
        assert_eq!(utf16_units("\u{1D11E}").collect::<Vec<_>>(), [0xD834, 0xDD1E]);
        let top = from_units([0xDBFE, 0xDC00]);
        assert_eq!(top.chars().count(), 2);
        assert_eq!(utf16_units(&top).collect::<Vec<_>>(), [0xDBFE, 0xDC00]);
        assert_eq!(code_point(0x10F800), Some(top));
    }
}
