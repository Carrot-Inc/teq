//! A number literal's digits read as scalac's `scala.util.FromDigits` reads them (`intFromDigits`, `longFromDigits`):
//! the readers `Typer.typedNumber` applies to an unsuffixed integer literal by its expected type, and the parser to a
//! suffixed `Long`.

/// Why digits do not read as a number: scalac's `FromDigits.MalformedNumber` and `NumberTooLarge`, by their messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigitsError {
    Malformed,
    TooLarge,
}

impl DigitsError {
    pub fn message(self) -> &'static str {
        match self {
            DigitsError::Malformed => "malformed number literal",
            DigitsError::TooLarge => "number too large",
        }
    }
}

/// `FromDigits.integerFromDigits`, line for line: `digits` with an optional sign and no separators, in `radix`, up to
/// `limit` (`Int.MaxValue` or `Long.MaxValue`). Outside radix 10 the digits read as unsigned up to twice the limit and
/// one (`0xffffffff` is the `Int` -1), and a negative number reaches `-limit - 1`. The arithmetic wraps as the JVM's
/// `Long` does, where scalac's does.
pub fn integer_from_digits(digits: &str, radix: i64, limit: i64) -> Result<i64, DigitsError> {
    let b = digits.as_bytes();
    let mut value: i64 = 0;
    let divider = if radix == 10 { 1 } else { 2 };
    let mut i = 0;
    let mut negated = false;
    let len = b.len();
    if 0 < len && (b[0] == b'-' || b[0] == b'+') {
        negated = b[0] == b'-';
        i += 1;
    }
    if i == len {
        return Err(DigitsError::Malformed);
    }
    while i < len {
        let ch = b[i];
        let d: i64 = if ch <= b'9' {
            ch as i64 - b'0' as i64
        } else if ch.is_ascii_lowercase() {
            ch as i64 - b'a' as i64 + 10
        } else if ch.is_ascii_uppercase() {
            ch as i64 - b'A' as i64 + 10
        } else {
            -1
        };
        if d < 0 || radix <= d {
            return Err(DigitsError::Malformed);
        }
        if value < 0
            || limit / (radix / divider) < value
            || limit - (d / divider) < value.wrapping_mul(radix / divider)
                && !(negated && limit == value.wrapping_mul(radix).wrapping_sub(1).wrapping_add(d))
        {
            return Err(DigitsError::TooLarge);
        }
        value = value.wrapping_mul(radix).wrapping_add(d);
        i += 1;
    }
    Ok(if negated { value.wrapping_neg() } else { value })
}

/// `FromDigits.intFromDigits`.
pub fn int_from_digits(digits: &str, radix: i64) -> Result<i32, DigitsError> {
    integer_from_digits(digits, radix, i32::MAX as i64).map(|v| v as i32)
}

/// `FromDigits.longFromDigits`.
pub fn long_from_digits(digits: &str, radix: i64) -> Result<i64, DigitsError> {
    integer_from_digits(digits, radix, i64::MAX)
}

/// The value of a decimal literal of at most `Int.MaxValue`, signed, from the parser's carrier: `intFromDigits` and
/// `longFromDigits` read such digits alike (no limit is near), so no text is read again. `None` for any other literal
/// (hexadecimal, binary, larger, or held as text), which its reader takes from its digits.
pub fn small_decimal(n: crate::ast::NumberLit) -> Option<i64> {
    match n.content {
        crate::ast::NumberContent::Value(m) if n.radix == 10 && m <= i32::MAX as u64 => Some(if n.neg { -(m as i64) } else { m as i64 }),
        _ => None,
    }
}

/// A literal's digits as `Parsers.literal` hands them to the readers: the separators out, a `0x` or `0b` prefix out
/// (its radix returned), and the sign of a `-` the parser folded in.
pub fn literal_digits(text: &str, neg: bool) -> (String, i64) {
    let clean: String = text.chars().filter(|&c| c != '_').collect();
    let (radix, body) = if let Some(h) = clean.strip_prefix("0x").or_else(|| clean.strip_prefix("0X")) {
        (16, h)
    } else if let Some(b) = clean.strip_prefix("0b").or_else(|| clean.strip_prefix("0B")) {
        (2, b)
    } else {
        (10, clean.as_str())
    };
    (if neg { format!("-{}", body) } else { body.to_string() }, radix)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The values scalac 3.8.4 gives these literals (the claim table's rows): an Int's and a Long's range by radix and
    // sign, the unsigned hexadecimal and binary readings, and the refusals.
    #[test]
    fn reads_as_from_digits() {
        let int = |t: &str, neg: bool| {
            let (d, r) = literal_digits(t, neg);
            int_from_digits(&d, r)
        };
        let long = |t: &str, neg: bool| {
            let (d, r) = literal_digits(t, neg);
            long_from_digits(&d, r)
        };
        assert_eq!(int("2147483647", false), Ok(i32::MAX));
        assert_eq!(int("2147483648", true), Ok(i32::MIN));
        assert_eq!(int("2147483648", false), Err(DigitsError::TooLarge));
        assert_eq!(int("2147483649", true), Err(DigitsError::TooLarge));
        assert_eq!(int("0xffffffff", false), Ok(-1));
        assert_eq!(int("0xff", false), Ok(255));
        assert_eq!(int("0x1ffffffff", false), Err(DigitsError::TooLarge));
        assert_eq!(int("0xffffffff", true), Ok(1));
        assert_eq!(int("0x80000000", true), Ok(i32::MIN));
        assert_eq!(int("0b101", false), Ok(5));
        assert_eq!(int("0b1010_1010", false), Ok(170));
        assert_eq!(int(&format!("0b{}", "1".repeat(32)), false), Ok(-1));
        assert_eq!(int(&format!("0b{}", "1".repeat(33)), false), Err(DigitsError::TooLarge));
        assert_eq!(int("010", false), Ok(10));
        assert_eq!(int("1__0", false), Ok(10));
        assert_eq!(long("0xffffffff", false), Ok(4294967295));
        assert_eq!(long("0xffffffff", true), Ok(-4294967295));
        assert_eq!(long(&format!("0b{}", "1".repeat(32)), false), Ok(4294967295));
        assert_eq!(long("9223372036854775807", false), Ok(i64::MAX));
        assert_eq!(long("9223372036854775808", false), Err(DigitsError::TooLarge));
        assert_eq!(long("9223372036854775808", true), Ok(i64::MIN));
        assert_eq!(long("9223372036854775809", true), Err(DigitsError::TooLarge));
        assert_eq!(long("0xffffffffffffffff", false), Ok(-1));
        assert_eq!(long("0x1ffffffffffffffff", false), Err(DigitsError::TooLarge));
    }

    // A decimal literal's carrier value, where it is taken, is what both readers give its digits: small values, the
    // powers of ten, the bytes' and shorts' edges and `Int.MaxValue`'s, either sign, with separators.
    #[test]
    fn small_decimal_reads_as_the_digits() {
        let values: Vec<u64> = vec![
            0, 1, 7, 9, 10, 42, 99, 100, 255, 256, 999, 1000, 4096, 65535, 65536, 99999, 100000, 1000000, 16777216, 99999999,
            100000000, 123456789, 999999999, 1000000000, i32::MAX as u64 - 1, i32::MAX as u64, i32::MAX as u64 + 1,
            i32::MAX as u64 + 2, u32::MAX as u64,
        ];
        for m in values {
            for neg in [false, true] {
                for text in [m.to_string(), format!("{}_0", m), format!("0{}", m)] {
                    let value: u64 = text.bytes().filter(|&c| c != b'_').fold(0, |v, c| v * 10 + (c - b'0') as u64);
                    let n = crate::ast::NumberLit { digits: crate::source::Span::default(), radix: 10, neg, content: crate::ast::NumberContent::Value(value) };
                    let (d, r) = literal_digits(&text, neg);
                    match small_decimal(n) {
                        Some(v) => {
                            assert_eq!(int_from_digits(&d, r).map(|i| i as i64), Ok(v), "{} {}", text, neg);
                            assert_eq!(long_from_digits(&d, r), Ok(v), "{} {}", text, neg);
                        }
                        None => assert!(value > i32::MAX as u64, "{} {}", text, neg),
                    }
                }
            }
        }
        let hex = crate::ast::NumberLit { digits: crate::source::Span::default(), radix: 16, neg: false, content: crate::ast::NumberContent::Value(255) };
        assert_eq!(small_decimal(hex), None);
    }
}
