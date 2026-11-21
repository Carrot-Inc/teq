//! Java's text of a `Double` and a `Float`: `Double.toString` and `Float.toString` as the JDK writes them (19 and
//! later, whose algorithm it specifies), the one implementation the typer's constants, the interpreter, the JVM
//! target's analysis and the NIR printer write numbers with. tests/support/java-numbers.txt is the JDK's text of
//! the values tests/support/JavaNumbers.java generates, which the tests below hold this to.

/// `Double.toString`: the shortest digits that read back as the value, plain between 1e-3 and
/// 1e7 and in scientific notation outside.
pub fn double_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    layout(double_digits(d), d.abs())
}

/// The digits `Double.toString` prints, in Rust's `[-]d[.ddd]e[-]x`: the shortest that read back
/// and of those the closest to the value, as Rust's shortest are, but the even one of two equally
/// close (`2.0233473011568512E15`, Rust `..513`), and two digits where one would do and a second
/// is closer (`4.9E-324`, Rust `5e-324`). Only 16 digits or more are close enough together to tie,
/// and only a small subnormal is imprecise enough for the second digit.
pub fn double_digits(d: f64) -> String {
    let sci = format!("{:e}", d);
    if d.abs() < f64::MIN_POSITIVE && !sci.contains('.') {
        return format!("{:.1e}", d);
    }
    let n = mantissa_digits(&sci);
    if n < 16 {
        return sci;
    }
    let even = format!("{:.*e}", n - 1, d);
    if even != sci && even.parse::<f64>() == Ok(d) { even } else { sci }
}

/// `double_digits` for `Float.toString`, where 7 digits can tie.
fn float_digits(f: f32) -> String {
    let sci = format!("{:e}", f);
    if f.abs() < f32::MIN_POSITIVE && !sci.contains('.') {
        return format!("{:.1e}", f);
    }
    let n = mantissa_digits(&sci);
    if n < 7 {
        return sci;
    }
    let even = format!("{:.*e}", n - 1, f);
    if even != sci && even.parse::<f32>() == Ok(f) { even } else { sci }
}

fn mantissa_digits(sci: &str) -> usize {
    sci.bytes().take_while(|&b| b != b'e').filter(u8::is_ascii_digit).count()
}

pub fn float_string(f: f32) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    layout(float_digits(f), f.abs() as f64)
}

/// `sci` is `[-]d[.ddd]e[-]x` with the shortest digits, as Rust formats it.
fn layout(sci: String, abs: f64) -> String {
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if (1e-3..1e7).contains(&abs) {
        // The decimal point stands after `exp + 1` digits.
        let point = exp + 1;
        if point <= 0 {
            out.push_str("0.");
            for _ in 0..-point {
                out.push('0');
            }
            out.push_str(&digits);
        } else if point as usize >= digits.len() {
            out.push_str(&digits);
            for _ in digits.len()..point as usize {
                out.push('0');
            }
            out.push_str(".0");
        } else {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        }
    } else {
        out.push_str(&digits[..1]);
        out.push('.');
        if digits.len() > 1 {
            out.push_str(&digits[1..]);
        } else {
            out.push('0');
        }
        out.push('E');
        out.push_str(&exp.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::sha256::Sha256;

    /// splitmix64 from 0, as tests/support/JavaNumbers.java draws.
    struct SplitMix(u64);

    impl SplitMix {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
    }

    /// Every power of two with its two neighbours, of a format of `fraction` bits after the point and
    /// an exponent of `exponent` bits: the subnormal ones, a bit each, then the normal ones.
    fn powers(fraction: u32, exponent: u32) -> Vec<u64> {
        let normal = (1..(1u64 << exponent) - 1).map(|e| e << fraction);
        (0..fraction).map(|k| 1u64 << k).chain(normal).flat_map(|bits| [bits - 1, bits, bits + 1]).collect()
    }

    /// The bits of a set of tests/support/JavaNumbers.java, in its order.
    fn values(set: &str) -> Vec<u64> {
        let mut random = SplitMix(0);
        match set {
            "float-powers" => powers(f32::MANTISSA_DIGITS - 1, 8),
            "double-powers" => powers(f64::MANTISSA_DIGITS - 1, 11),
            "float-random" => (0..100_000).map(|_| random.next() >> 32).collect(),
            "double-random" => (0..100_000).map(|_| random.next()).collect(),
            _ => panic!("no set {set}"),
        }
    }

    fn line(single: bool, bits: u64) -> String {
        if single {
            format!("{:08x} {}", bits, float_string(f32::from_bits(bits as u32)))
        } else {
            format!("{:016x} {}", bits, double_string(f64::from_bits(bits)))
        }
    }

    #[test]
    fn writes_numbers_as_the_jdk_does() {
        let (mut sets, mut ties) = (0, 0);
        for oracle in include_str!("../tests/support/java-numbers.txt").lines().filter(|l| !l.starts_with('#')) {
            match oracle.split(' ').collect::<Vec<_>>()[..] {
                ["set", set, count, digest] => {
                    let lines: String = values(set).into_iter().map(|bits| line(set.starts_with("float"), bits) + "\n").collect();
                    let mut sha = Sha256::new();
                    sha.update(lines.as_bytes());
                    if (lines.lines().count().to_string(), sha.hex()) != (count.to_string(), digest.to_string()) {
                        let ours = std::env::temp_dir().join(format!("teq-java-numbers-{set}.txt"));
                        let _ = std::fs::write(&ours, &lines);
                        panic!("{set}: {} differs from `java tests/support/JavaNumbers.java --lines {set}`", ours.display());
                    }
                    sets += 1;
                }
                [width, bits, text] => {
                    let single = width == "float";
                    assert_eq!(line(single, u64::from_str_radix(bits, 16).unwrap()), format!("{bits} {text}"));
                    ties += 1;
                }
                _ => panic!("a line of no shape: {oracle}"),
            }
        }
        assert_eq!((sets, ties), (4, 47));
    }
}
