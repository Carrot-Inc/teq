//! `java.util.Formatter` for `String.format` and the `f` interpolator, and the escape
//! translation of `StringContext.processEscapes`. Fractions are formatted from the digits of
//! `Double.toString`, rounded half up, as the JVM's formatter does.

use super::value::*;
use super::*;

/// A decimal as sign, digits without leading zeros and the position of the point: the value is
/// `0.DIGITS * 10^point`.
struct Dec {
    neg: bool,
    digits: String,
    point: i32,
    zero: bool,
}

/// The digits of `Double.toString`, which `java.util.Formatter` rounds from.
fn dec_of_f64(x: f64) -> Dec {
    let neg = x.is_sign_negative();
    let sci = java_digits(x.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = digits.trim_end_matches('0');
    if digits.is_empty() {
        return Dec { neg, digits: "0".to_string(), point: 1, zero: true };
    }
    Dec { neg, digits: digits.to_string(), point: exp + 1, zero: false }
}

fn dec_of_parts(neg: bool, int: &str, frac: &str) -> Dec {
    let mut digits = format!("{}{}", int, frac);
    let mut point = int.len() as i32;
    let lead = digits.bytes().take_while(|&b| b == b'0').count();
    digits.drain(..lead);
    point -= lead as i32;
    if digits.is_empty() {
        return Dec { neg, digits: "0".to_string(), point: 1, zero: true };
    }
    Dec { neg, digits, point, zero: false }
}

fn dec_of_i64(x: i64) -> Dec {
    dec_of_parts(x < 0, &x.unsigned_abs().to_string(), "")
}

/// The first `count` digits, rounded half up.
fn round_digits(digits: &str, count: i32) -> String {
    if count < 0 {
        return "0".to_string();
    }
    let count = count as usize;
    if count >= digits.len() {
        return format!("{}{}", digits, "0".repeat(count - digits.len()));
    }
    let mut kept: Vec<u8> = digits.as_bytes()[..count].to_vec();
    if digits.as_bytes()[count] >= b'5' {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    if kept.is_empty() {
        return "0".to_string();
    }
    String::from_utf8(kept).unwrap()
}

fn group(int: &str) -> String {
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn fmt_fixed(d: &Dec, precision: i32, grouping: bool) -> String {
    let text = round_digits(&d.digits, d.point + precision);
    let text = if (text.len() as i32) < precision + 1 { format!("{}{}", "0".repeat((precision + 1) as usize - text.len()), text) } else { text };
    let split = text.len() - precision as usize;
    let mut int = text[..split].to_string();
    if grouping {
        int = group(&int);
    }
    if precision > 0 { format!("{}.{}", int, &text[split..]) } else { int }
}

fn fmt_exp(d: &Dec, precision: i32) -> String {
    let mut exp = if d.zero { 0 } else { d.point - 1 };
    let mut text = round_digits(&d.digits, precision + 1);
    if text.len() as i32 > precision + 1 {
        text.pop();
        exp += 1;
    }
    let mantissa = if precision > 0 { format!("{}.{}", &text[..1], &text[1..]) } else { text };
    format!("{}e{}{:02}", mantissa, if exp < 0 { '-' } else { '+' }, exp.abs())
}

impl<'a, 't> Interp<'a, 't> {
    fn format_error<T>(&mut self, name: &'static str, msg: String) -> R<T> {
        let _ = name;
        self.throw_named("IllegalArgumentException", &msg)
    }

    fn fmt_one(&mut self, flags: &str, width: Option<usize>, precision: Option<i32>, conv: char, arg: &Value) -> R<String> {
        let has = |f: char| flags.contains(f);
        let mut sign = String::new();
        let mut numeric = false;
        let lower = conv.to_ascii_lowercase();
        let mut flags = flags.to_string();
        let body: String = match lower {
            'd' | 'x' | 'o' => {
                let v = match arg {
                    v if v.is_integral() => v.as_i64().unwrap(),
                    Value::Char(c) if lower != 'd' => *c as i64,
                    other => {
                        let shown = self.to_str(other)?;
                        return self.format_error("IllegalFormatConversionException", format!("{} != {}", conv, shown));
                    }
                };
                numeric = true;
                if lower == 'd' {
                    if v < 0 {
                        sign.push('-');
                    }
                    let text = v.unsigned_abs().to_string();
                    if has(',') { group(&text) } else { text }
                } else {
                    let unsigned = match arg {
                        Value::Int(i) => *i as u32 as u64,
                        Value::Byte(b) => *b as u8 as u64,
                        Value::Short(s) => *s as u16 as u64,
                        _ => v as u64,
                    };
                    let text = if lower == 'x' { format!("{:x}", unsigned) } else { format!("{:o}", unsigned) };
                    if has('#') { format!("{}{}", if lower == 'x' { "0x" } else { "0" }, text) } else { text }
                }
            }
            'f' | 'e' | 'g' => {
                let d = match arg {
                    Value::Double(x) if !x.is_finite() => {
                        if *x < 0.0 {
                            sign.push('-');
                        }
                        flags = flags.replace('0', "");
                        return Ok(self.pad(&sign, if x.is_nan() { "NaN" } else { "Infinity" }, &flags, width, false, conv));
                    }
                    Value::Float(x) if !x.is_finite() => {
                        if *x < 0.0 {
                            sign.push('-');
                        }
                        flags = flags.replace('0', "");
                        return Ok(self.pad(&sign, if x.is_nan() { "NaN" } else { "Infinity" }, &flags, width, false, conv));
                    }
                    Value::Double(x) => dec_of_f64(*x),
                    Value::Float(x) => dec_of_f64(*x as f64),
                    v if v.is_integral() => {
                        let shown = self.to_str(v)?;
                        return self.format_error("IllegalFormatConversionException", format!("{} != java.lang.Integer ({})", conv, shown));
                    }
                    other => {
                        let shown = self.to_str(other)?;
                        return self.format_error("IllegalFormatConversionException", format!("{} != {}", conv, shown));
                    }
                };
                numeric = true;
                if d.neg && !d.zero || (d.neg && d.zero && lower == 'f') {
                    sign.push('-');
                }
                match lower {
                    'f' => fmt_fixed(&d, precision.unwrap_or(6), has(',')),
                    'e' => fmt_exp(&d, precision.unwrap_or(6)),
                    _ => {
                        let digits = precision.map_or(6, |p| p.max(1));
                        let mut exp = if d.zero { 0 } else { d.point - 1 };
                        if round_digits(&d.digits, digits).len() as i32 > digits {
                            exp += 1;
                        }
                        if !d.zero && (exp < -4 || exp >= digits) {
                            fmt_exp(&d, digits - 1)
                        } else {
                            fmt_fixed(&d, digits - 1 - exp, has(','))
                        }
                    }
                }
            }
            's' => {
                let s = self.to_str(arg)?;
                match precision {
                    Some(p) => utf16_slice(&s, 0, p.max(0) as usize).into_owned(),
                    None => s,
                }
            }
            'c' => match arg {
                Value::Char(c) => char_to_string(*c),
                v if v.is_integral() => crate::text::code_point(v.as_i64().unwrap() as u32).unwrap_or_default(),
                other => self.to_str(other)?,
            },
            'b' => match arg {
                Value::Null => "false".to_string(),
                Value::Bool(b) => b.to_string(),
                _ => "true".to_string(),
            },
            'h' => format!("{:x}", self.hash_code(arg)?),
            _ => return self.format_error("UnknownFormatConversionException", format!("Conversion = '{}'", conv)),
        };
        if numeric && sign.is_empty() {
            if has('+') {
                sign.push('+');
            } else if has(' ') {
                sign.push(' ');
            }
        }
        let mut body = body;
        if numeric && sign == "-" && has('(') {
            sign = "(".to_string();
            body.push(')');
        }
        Ok(self.pad(&sign, &body, &flags, width, numeric, conv))
    }

    fn pad(&self, sign: &str, body: &str, flags: &str, width: Option<usize>, numeric: bool, conv: char) -> String {
        let mut text = format!("{}{}", sign, body);
        if let Some(w) = width {
            let len = utf16_len(&text);
            if len < w {
                let fill = w - len;
                if flags.contains('-') {
                    text.push_str(&" ".repeat(fill));
                } else if flags.contains('0') && numeric {
                    text = format!("{}{}{}", sign, "0".repeat(fill), body);
                } else {
                    text = format!("{}{}", " ".repeat(fill), text);
                }
            }
        }
        if conv.is_ascii_uppercase() { text.to_uppercase() } else { text }
    }

    /// `String.format(fmt, args)`.
    pub(super) fn format(&mut self, fmt: &str, args: &[Value]) -> R<String> {
        let mut out = String::new();
        let bytes = fmt.as_bytes();
        let mut i = 0;
        let mut next = 0usize;
        while i < bytes.len() {
            if bytes[i] != b'%' {
                let start = i;
                while i < bytes.len() && bytes[i] != b'%' {
                    i += 1;
                }
                crate::text::push_str(&mut out, &fmt[start..i]);
                continue;
            }
            i += 1;
            let spec_start = i;
            let mut index: Option<usize> = None;
            // `n$` names an argument.
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i && j < bytes.len() && bytes[j] == b'$' {
                index = fmt[i..j].parse().ok();
                i = j + 1;
            }
            let mut flags = String::new();
            while i < bytes.len() && b"-#+ 0,(".contains(&bytes[i]) {
                flags.push(bytes[i] as char);
                i += 1;
            }
            let ws = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let width: Option<usize> = if i > ws { fmt[ws..i].parse().ok() } else { None };
            let mut precision: Option<i32> = None;
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                let ps = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                precision = fmt[ps..i].parse().ok();
            }
            if i >= bytes.len() {
                return self.format_error("UnknownFormatConversionException", format!("Conversion = '{}'", &fmt[spec_start - 1..]));
            }
            let conv = bytes[i] as char;
            i += 1;
            match conv {
                '%' => out.push('%'),
                'n' => out.push('\n'),
                _ => {
                    let k = match index {
                        Some(n) => n.saturating_sub(1),
                        None => {
                            next += 1;
                            next - 1
                        }
                    };
                    let Some(arg) = args.get(k).cloned() else {
                        return self.throw_named("IllegalArgumentException", &format!("Format specifier '{}'", &fmt[spec_start - 1..i]));
                    };
                    let piece = self.fmt_one(&flags, width, precision, conv, &arg)?;
                    crate::text::push_str(&mut out, &piece);
                }
            }
        }
        Ok(out)
    }

    /// `f"..."`: an argument is formatted by the specifier that follows it, or as `%s` without one.
    pub(super) fn format_interpolated(&mut self, parts: &[Value], args: &[Value]) -> R<String> {
        let mut fmt = String::new();
        for (i, part) in parts.iter().enumerate() {
            let text = part.as_str().unwrap_or("").to_string();
            let text = self.process_escapes(&text)?;
            if i > 0 {
                let specified = starts_with_spec(&text) && !text.starts_with("%n");
                if !specified {
                    fmt.push_str("%s");
                }
            }
            fmt.push_str(&text);
        }
        self.format(&fmt, args)
    }

    /// `StringContext.processEscapes`: the escapes of a string literal.
    pub(super) fn process_escapes(&mut self, s: &str) -> R<String> {
        if !s.contains('\\') {
            return Ok(s.to_string());
        }
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c != '\\' {
                match crate::text::surrogate_of(c) {
                    Some(u) => crate::text::push_unit(&mut out, u),
                    None => out.push(c),
                }
                i += 1;
                continue;
            }
            i += 1;
            let Some(&e) = chars.get(i) else {
                return self.throw_named("IllegalArgumentException", &format!("invalid escape at terminal index {} in \"{}\"", i - 1, s));
            };
            i += 1;
            match e {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '\'' => out.push('\''),
                'u' => {
                    while chars.get(i) == Some(&'u') {
                        i += 1;
                    }
                    let hex: String = chars[i..(i + 4).min(chars.len())].iter().collect();
                    match u32::from_str_radix(&hex, 16).ok().filter(|_| hex.len() == 4) {
                        Some(code) => {
                            i += 4;
                            crate::text::push_unit(&mut out, code as u16);
                        }
                        None => return self.throw_named("IllegalArgumentException", &format!("invalid escape '\\u{}' in \"{}\"", hex, s)),
                    }
                }
                _ => return self.throw_named("IllegalArgumentException", &format!("invalid escape '\\{}' in \"{}\"", e, s)),
            }
        }
        Ok(out)
    }
}

fn starts_with_spec(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'%') {
        return false;
    }
    let mut i = 1;
    while i < bytes.len() && b"-#+ 0,(".contains(&bytes[i]) {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    i < bytes.len() && bytes[i].is_ascii_alphabetic()
}

#[allow(dead_code)]
pub(super) fn dec_int(x: i64) -> String {
    dec_of_i64(x).digits
}
