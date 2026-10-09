//! The builtins behind the `@js` templates of the standard library, keyed by the qualified name
//! of the def each one stands for, and the templates the typer makes itself, keyed by their text.

use super::bignum::Big;
use super::char_tables::{DIGIT_ZEROS, LETTERS, NUMERIC_RUNS};
use super::value::*;
use super::*;
use std::cell::Cell;

type A<'v> = &'v [Value];

fn arg(a: A, i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Null)
}

/// `new String(bytes, ..)`: the `newString` overload over `Array[Byte]` of the arguments' shape
/// (a charset or its name last), run on its Scala body, the std's charset decoding.
fn string_from_bytes(it: &mut Interp, a: &[Value]) -> Option<R> {
    let n = a.len().checked_sub(1)?;
    let bytes = matches!(a.get(1), Some(Value::Array(arr)) if arr.borrow().iter().any(|v| matches!(v, Value::Byte(_))));
    let charset_last = n >= 2 && !matches!(a[n], Value::Int(_));
    if !bytes && !charset_last {
        return None;
    }
    let Value::Obj(module) = &a[0] else { return None };
    let name = it.typer.interner.intern("newString");
    let m = *it.syms().class(module.class).members.get(&name)?;
    let alts: Vec<SymId> = it.syms().alternatives(m).map_or_else(|| vec![m], |x| x.to_vec());
    let (array, t_byte, t_string) = (it.typer.b.array, it.typer.b.t_byte, it.typer.b.t_string);
    let name_last = matches!(a[n], Value::Str(_));
    let sym = alts.into_iter().find(|&s| {
        let Some(sig) = it.syms().sym(s).sig.clone() else { return false };
        let Some(params) = sig.clauses.first().map(|c| &c.params) else { return false };
        let over_bytes = params.len() == n
            && matches!(it.typer.types.get(params[0].ty), crate::types::Type::Class(c, args) if c == array && it.typer.types.items(args).first() == Some(&t_byte));
        over_bytes && (n == 1 || (params[n - 1].ty == t_string) == name_last)
    })?;
    Some(it.invoke(a[0].clone(), sym, a[1..].to_vec()))
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

/// The number of a `java.lang` box's member that is both static (`Double.isNaN(d)`, after the
/// module) and an instance method (`d.isNaN()`, the receiver alone).
fn static_or_instance_dbl(it: &mut Interp, a: A) -> R<f64> {
    dbl(it, a, a.len().saturating_sub(1))
}

fn dbl(it: &mut Interp, a: A, i: usize) -> R<f64> {
    match a.get(i).and_then(|v| v.as_f64()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Double", i)),
    }
}

fn boolean(it: &mut Interp, a: A, i: usize) -> R<bool> {
    match a.get(i).and_then(|v| v.as_bool()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Boolean", i)),
    }
}

/// A String or a Char, which JavaScript does not tell apart.
fn text(it: &mut Interp, a: A, i: usize) -> R<Rc<str>> {
    match a.get(i) {
        Some(Value::Str(s)) => {
            index_str(s);
            Ok(s.clone())
        }
        Some(Value::Char(c)) => Ok(Rc::from(char_to_string(*c))),
        Some(Value::Null) => Ok(Rc::from("null")),
        Some(other) => {
            let s = it.to_str(other)?;
            Ok(Rc::from(s))
        }
        None => it.unsupported(format!("argument {} is missing", i)),
    }
}

/// The `ch: Int` of `indexOf(ch)` as the character it codes, `None` for a number that codes none;
/// a `String` or `Char` as its text.
fn code_point_or_text(it: &mut Interp, a: A, i: usize) -> R<Option<Rc<str>>> {
    match a.get(i) {
        Some(Value::Int(c)) => Ok(crate::text::code_point(*c as u32).map(|s| Rc::from(s))),
        _ => text(it, a, i).map(Some),
    }
}

/// `Character.isJavaIdentifierPart` without the Unicode tables: letters and digits, `_` and `$`,
/// the currency signs, the combining marks' blocks and the identifier-ignorable controls.
fn is_java_identifier_part(c: char) -> bool {
    let n = c as u32;
    c.is_alphanumeric()
        || matches!(c, '_' | '$' | '\u{203F}' | '\u{2040}' | '\u{2054}' | '\u{FE33}' | '\u{FE34}' | '\u{FE4D}'..='\u{FE4F}' | '\u{FF3F}')
        || matches!(n, 0xA2..=0xA5 | 0x20A0..=0x20CF)
        || matches!(n, 0x300..=0x36F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
        || matches!(n, 0..=8 | 0xE..=0x1B | 0x7F..=0x9F | 0xAD | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0xFEFF)
}

fn chr(it: &mut Interp, a: A, i: usize) -> R<char> {
    match a.get(i) {
        Some(Value::Char(c)) => Ok(crate::text::unit_char(*c)),
        Some(Value::Str(s)) => Ok(s.chars().next().unwrap_or('\0')),
        Some(v) if v.is_integral() => Ok(char::from_u32(v.as_i64().unwrap() as u32).unwrap_or('\u{fffd}')),
        _ => it.unsupported(format!("argument {} is no Char", i)),
    }
}

/// A Char argument as its UTF-16 unit.
fn unit(it: &mut Interp, a: A, i: usize) -> R<u16> {
    match a.get(i) {
        Some(Value::Char(c)) => Ok(*c),
        Some(Value::Str(s)) => Ok(crate::text::utf16_units(s).next().unwrap_or(0)),
        Some(v) if v.is_integral() => Ok(v.as_i64().unwrap() as u16),
        _ => it.unsupported(format!("argument {} is no Char", i)),
    }
}

fn array(it: &mut Interp, a: A, i: usize) -> R<Rc<ArrayCell>> {
    match a.get(i) {
        Some(Value::Array(x)) => Ok(x.clone()),
        Some(Value::Null) => it.throw_named("NullPointerException", "Cannot read the array because it is null"),
        _ => it.unsupported(format!("argument {} is no Array", i)),
    }
}

fn map(it: &mut Interp, a: A, i: usize) -> R<Rc<MapCell>> {
    match a.get(i) {
        Some(Value::Map(m)) => Ok(m.clone()),
        _ => it.unsupported(format!("argument {} is no RawMap", i)),
    }
}

fn str_of_char(c: char) -> Value {
    Value::string(c.to_string())
}

/// The character functions of `String.prototype` on a Char, whose result is a Char again.
fn map_char(v: &Value, f: impl Fn(char) -> String) -> Value {
    match v {
        Value::Char(0xD800..=0xDFFF) => v.clone(),
        Value::Char(c) => {
            let s = f(char::from_u32(*c as u32).unwrap_or('\u{fffd}'));
            let mut units = s.encode_utf16();
            match (units.next(), units.next()) {
                (Some(u), None) => Value::Char(u),
                _ => Value::Char(*c),
            }
        }
        Value::Str(s) => Value::string(s.chars().map(&f).collect()),
        other => other.clone(),
    }
}

fn to_radix(mut v: i64, radix: u32) -> String {
    if v == 0 {
        return "0".to_string();
    }
    let neg = v < 0;
    let mut digits = Vec::new();
    while v != 0 {
        let d = (v % radix as i64).unsigned_abs() as u32;
        digits.push(std::char::from_digit(d, radix).unwrap());
        v /= radix as i64;
    }
    if neg {
        digits.push('-');
    }
    digits.iter().rev().collect()
}

fn unsigned_radix(v: u64, radix: u32) -> String {
    if v == 0 {
        return "0".to_string();
    }
    let mut v = v;
    let mut digits = Vec::new();
    while v != 0 {
        digits.push(std::char::from_digit((v % radix as u64) as u32, radix).unwrap());
        v /= radix as u64;
    }
    digits.iter().rev().collect()
}

/// `java.lang.Double.parseDouble`'s grammar, which is narrower than a general number parser.
fn parse_java_double(s: &str) -> Option<f64> {
    let t = s.trim_matches(|c: char| (c as u32) <= 0x20);
    let (neg, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let value = match body {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        _ => {
            let body = body.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(body);
            let (mantissa, exp) = match body.find(['e', 'E']) {
                Some(at) => (&body[..at], Some(&body[at + 1..])),
                None => (body, None),
            };
            let (int, frac) = match mantissa.split_once('.') {
                Some((i, f)) => (i, Some(f)),
                None => (mantissa, None),
            };
            let digits_ok = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
            let mantissa_ok = match frac {
                Some(f) => (digits_ok(int) && (f.is_empty() || digits_ok(f))) || (int.is_empty() && digits_ok(f)),
                None => digits_ok(int),
            };
            if !mantissa_ok {
                return None;
            }
            if let Some(e) = exp {
                let e = e.strip_prefix(['+', '-']).unwrap_or(e);
                if !digits_ok(e) {
                    return None;
                }
            }
            body.parse::<f64>().ok()?
        }
    };
    Some(if neg { -value } else { value })
}

fn int_or_null(s: &str) -> Value {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return Value::Null;
    }
    match s.parse::<i64>() {
        Ok(n) if n >= i32::MIN as i64 && n <= i32::MAX as i64 => Value::Int(n as i32),
        _ => Value::Null,
    }
}

fn long_or_null(s: &str) -> Value {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return Value::Null;
    }
    match s.strip_prefix('+').unwrap_or(s).parse::<i64>() {
        Ok(n) => Value::Long(n),
        _ => Value::Null,
    }
}

fn number_format<T>(it: &mut Interp, s: &str) -> R<T> {
    it.throw_named("NumberFormatException", &format!("For input string: \"{}\"", s))
}

fn parse_int_radix(it: &mut Interp, s: &str, radix: u32) -> R {
    if !(2..=36).contains(&radix) {
        return number_format(it, s);
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() || !body.chars().all(|c| c.is_digit(radix)) {
        return number_format(it, s);
    }
    match i64::from_str_radix(s.strip_prefix('+').unwrap_or(s), radix) {
        Ok(n) if n >= i32::MIN as i64 && n <= i32::MAX as i64 => Ok(Value::Int(n as i32)),
        _ => number_format(it, s),
    }
}

fn is_java_whitespace(c: char) -> bool {
    matches!(c, '\t'..='\r' | '\u{1c}'..='\u{1f}' | ' ' | '\u{1680}' | '\u{2000}'..='\u{2006}' | '\u{2008}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{205f}' | '\u{3000}')
}

fn is_space_char(c: char) -> bool {
    matches!(c, ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

/// Java's `String.trim`: what is at or below the space character goes.
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

fn java_strip(s: &str) -> &str {
    s.trim_matches(is_java_whitespace)
}

/// The wider of two numeric kinds, as the result of a generic `max` takes it.
fn wider(a: &Value, b: &Value) -> u8 {
    let rank = |v: &Value| match v {
        Value::Double(_) => 5,
        Value::Float(_) => 4,
        Value::Long(_) => 3,
        Value::Int(_) => 2,
        Value::Short(_) | Value::Byte(_) | Value::Char(_) => 1,
        _ => 0,
    };
    let r = rank(a).max(rank(b));
    match r {
        5 => K_DOUBLE,
        4 => K_FLOAT,
        3 => K_LONG,
        _ => K_INT,
    }
}

fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

fn num_max(a: &Value, b: &Value) -> Value {
    match wider(a, b) {
        K_DOUBLE => Value::Double(java_max(a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0))),
        K_FLOAT => Value::Float(java_max(a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0)) as f32),
        K_LONG => Value::Long(a.as_i64().unwrap_or(0).max(b.as_i64().unwrap_or(0))),
        _ => Value::Int(a.as_i32().unwrap_or(0).max(b.as_i32().unwrap_or(0))),
    }
}

fn num_min(a: &Value, b: &Value) -> Value {
    match wider(a, b) {
        K_DOUBLE => Value::Double(java_min(a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0))),
        K_FLOAT => Value::Float(java_min(a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0)) as f32),
        K_LONG => Value::Long(a.as_i64().unwrap_or(0).min(b.as_i64().unwrap_or(0))),
        _ => Value::Int(a.as_i32().unwrap_or(0).min(b.as_i32().unwrap_or(0))),
    }
}

fn num_abs(a: &Value) -> Value {
    match a {
        Value::Double(d) => Value::Double(d.abs()),
        Value::Float(f) => Value::Float(f.abs()),
        Value::Long(l) => Value::Long(l.wrapping_abs()),
        Value::Int(i) => Value::Int(i.wrapping_abs()),
        other => Value::Int(other.as_i32().unwrap_or(0).wrapping_abs()),
    }
}

fn num_signum(a: &Value) -> Value {
    match a {
        Value::Double(d) => Value::Double(if d.is_nan() || *d == 0.0 { *d } else { d.signum() }),
        Value::Float(f) => Value::Float(if f.is_nan() || *f == 0.0 { *f } else { f.signum() }),
        Value::Long(l) => Value::Long(l.signum()),
        other => Value::Int(other.as_i32().unwrap_or(0).signum()),
    }
}

fn floor_mod_i64(a: i64, b: i64) -> i64 {
    let r = a.wrapping_rem(b);
    if r != 0 && ((r < 0) != (b < 0)) { r.wrapping_add(b) } else { r }
}

fn floor_div_i64(a: i64, b: i64) -> i64 {
    let q = a.wrapping_div(b);
    if (a ^ b) < 0 && q.wrapping_mul(b) != a { q - 1 } else { q }
}

/// `Math.addExact` and its kind over the arguments after the module: `Long` when either is one,
/// as the overloads `(JJ)J` and `(JI)J` are, `Int` otherwise.
fn exact(it: &mut Interp, a: A, op: fn(i64, i64) -> Option<i64>) -> R {
    let (x, y) = (arg(a, 1), arg(a, 2));
    let long = matches!(x, Value::Long(_)) || matches!(y, Value::Long(_));
    let r = op(x.as_i64().unwrap_or(0), y.as_i64().unwrap_or(0));
    match r {
        Some(r) if long => Ok(Value::Long(r)),
        Some(r) if r == r as i32 as i64 => Ok(Value::Int(r as i32)),
        _ => it.throw_named("ArithmeticException", if long { "long overflow" } else { "integer overflow" }),
    }
}

fn num_floor_div(it: &mut Interp, a: &Value, b: &Value) -> R {
    let (x, y) = (a.as_i64().unwrap_or(0), b.as_i64().unwrap_or(0));
    if y == 0 {
        return it.throw_named("ArithmeticException", "/ by zero");
    }
    Ok(if matches!(wider(a, b), K_LONG) { Value::Long(floor_div_i64(x, y)) } else { Value::Int(floor_div_i64(x, y) as i32) })
}

fn num_floor_mod(it: &mut Interp, a: &Value, b: &Value) -> R {
    let (x, y) = (a.as_i64().unwrap_or(0), b.as_i64().unwrap_or(0));
    if y == 0 {
        return it.throw_named("ArithmeticException", "/ by zero");
    }
    Ok(if matches!(wider(a, b), K_LONG) { Value::Long(floor_mod_i64(x, y)) } else { Value::Int(floor_mod_i64(x, y) as i32) })
}

/// JavaScript's `Array.prototype.slice` bounds: negative from the end, clamped.
fn slice_bounds(len: usize, from: i32, until: i32) -> (usize, usize) {
    let fix = |i: i32| if i < 0 { (len as i64 + i as i64).max(0) as usize } else { (i as usize).min(len) };
    let (a, b) = (fix(from), fix(until));
    (a, b.max(a))
}

fn compare_natural(it: &mut Interp, a: &Value, b: &Value) -> R<i32> {
    Ok(match (a, b) {
        (Value::Str(x), Value::Str(y)) => compare_strings(x, y),
        (x, y) if x.is_number() && y.is_number() => {
            if x.is_fractional() || y.is_fractional() {
                compare_doubles(x.as_f64().unwrap(), y.as_f64().unwrap())
            } else {
                x.as_i64().unwrap().cmp(&y.as_i64().unwrap()) as i32
            }
        }
        (Value::Char(x), Value::Char(y)) => *x as i32 - *y as i32,
        (Value::Bool(x), Value::Bool(y)) => (*x as i32) - (*y as i32),
        (Value::Obj(_), _) => {
            let r = it.call_by_name(a.clone(), "compareTo", vec![b.clone()])?;
            r.as_i32().unwrap_or(0)
        }
        _ => 0,
    })
}

/// A stable merge sort with a comparison that may run program code.
fn sort_values(it: &mut Interp, items: &mut Vec<Value>, cmp: &mut dyn FnMut(&mut Interp, &Value, &Value) -> R<i32>) -> R<()> {
    let n = items.len();
    if n < 2 {
        return Ok(());
    }
    let mut src = items.clone();
    let mut dst = items.clone();
    let mut width = 1;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let mid = (lo + width).min(n);
            let hi = (lo + 2 * width).min(n);
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while k < hi {
                let take_left = i < mid && (j >= hi || cmp(it, &src[i], &src[j])? <= 0);
                if take_left {
                    dst[k] = src[i].clone();
                    i += 1;
                } else {
                    dst[k] = src[j].clone();
                    j += 1;
                }
                k += 1;
            }
            lo += 2 * width;
        }
        std::mem::swap(&mut src, &mut dst);
        width *= 2;
    }
    *items = src;
    Ok(())
}

/// A decimal digit's value, -1 for anything else.
fn decimal(u: u16) -> i32 {
    DIGIT_ZEROS.iter().find(|&&zero| u.wrapping_sub(zero) < 10).map_or(-1, |&zero| (u - zero) as i32)
}

/// `Character.digit`: a decimal digit, or a Latin letter, ASCII or fullwidth, as 10 to 35.
fn digit(u: u16, radix: i32) -> i32 {
    if !(2..=36).contains(&radix) {
        return -1;
    }
    let value = match u {
        0x41..=0x5a => (u - 0x41) as i32 + 10,
        0x61..=0x7a => (u - 0x61) as i32 + 10,
        0xff21..=0xff3a => (u - 0xff21) as i32 + 10,
        0xff41..=0xff5a => (u - 0xff41) as i32 + 10,
        _ => decimal(u),
    };
    if value < radix { value } else { -1 }
}

fn numeric_value(u: u16) -> i32 {
    match decimal(u) {
        -1 => NUMERIC_RUNS
            .iter()
            .find(|&&(first, last, ..)| (first..=last).contains(&u))
            .map_or(-1, |&(first, _, value, grows)| if grows { value + (u - first) as i32 } else { value }),
        d => d,
    }
}

fn is_letter(u: u16) -> bool {
    let i = LETTERS.partition_point(|&(_, last)| last < u);
    LETTERS.get(i).is_some_and(|&(first, _)| first <= u)
}


macro_rules! reg {
    ($it:ident, $name:expr, |$i:ident, $a:ident| $body:expr) => {
        $it.insert($name.to_string(), Rc::new(|$i: &mut Interp<'_, '_>, $a: &[Value]| -> R { $body }));
    };
}
pub(super) use reg;
pub(super) type Table = FxMap<String, Builtin>;

/// The table, built once per thread and shared by the interpreters made on it.
pub fn table() -> Rc<Table> {
    thread_local! { static TABLE: RefCell<Option<Rc<Table>>> = const { RefCell::new(None) }; }
    TABLE.with(|t| {
        t.borrow_mut()
            .get_or_insert_with(|| {
                let mut table = Table::default();
                install_core(&mut table);
                install_strings(&mut table);
                install_numbers(&mut table);
                install_collections(&mut table);
                install_java(&mut table);
                super::jdk::install(&mut table);
                install_regex(&mut table);
                install_regex_engine(&mut table);
                install_bignum(&mut table);
                install_typer_templates(&mut table);
                install_reflective(&mut table);
                super::quoted::install(&mut table);
                super::files::install(&mut table);
                super::process::install(&mut table);
                Rc::new(table)
            })
            .clone()
    })
}

fn install_core(it: &mut Table) {
    // One thread: a fence orders nothing.
    reg!(it, "scala.runtime.Statics.releaseFence", |_it, _a| Ok(Value::Unit));
    reg!(it, "scala.printlnImpl", |it, a| {
        if it.pure {
            return it.impure("println");
        }
        let s = it.printed(&arg(a, 0))?;
        it.print(&s);
        it.print("\n");
        Ok(Value::Unit)
    });
    reg!(it, "scala.print", |it, a| {
        if it.pure {
            return it.impure("print");
        }
        let s = it.printed(&arg(a, 0))?;
        it.print(&s);
        Ok(Value::Unit)
    });
    reg!(it, "scala.Any.##", |it, a| Ok(Value::Int(it.hash(&arg(a, 0))?)));
    reg!(it, "scala.Product.productArity", |it, a| Ok(Value::Int(it.product_arity(&arg(a, 0))?)));
    reg!(it, "scala.Product.productElement", |it, a| {
        let i = int(it, a, 1)?;
        it.product_element(&arg(a, 0), i)
    });
    reg!(it, "scala.Product.productPrefix", |it, a| Ok(Value::string(it.product_prefix(&arg(a, 0))?)));
    reg!(it, "scala.Product.productElementName", |it, a| {
        let i = int(it, a, 1)?;
        it.product_element_name(&arg(a, 0), i)
    });
    reg!(it, "scala.matchErrorMessage", |it, a| {
        let v = arg(a, 0);
        let shown = it.to_str(&v)?;
        let class = match it.class_of_value(&v) {
            Value::Class(c) => c.qname.to_string(),
            _ => String::new(),
        };
        Ok(Value::string(format!("{} (of class {})", shown, class)))
    });
    reg!(it, "scala.unsafeCast", |_it, a| Ok(arg(a, 0)));
    reg!(it, "scala.PartialFunction.apply", |it, a| {
        let f = arg(a, 0);
        it.apply_value(f, vec![arg(a, 1)])
    });
    reg!(it, "scala.newPartialFunction", |_it, a| {
        Ok(Value::Fun(Rc::new(Closure {
            kind: ClosureKind::Partial(arg(a, 0), arg(a, 1)),
            env: Frame::new(None),
            this: Value::Unit,
            def_key: 0,
            class: None,
            hash: Cell::new(0),
        })))
    });
    reg!(it, "scala.partialMiss", |_it, _a| {
        Ok(Value::Fun(Rc::new(Closure { kind: ClosureKind::Miss, env: Frame::new(None), this: Value::Unit, def_key: 0, class: None, hash: Cell::new(0) })))
    });
    reg!(it, "scala.isPartialMiss", |_it, a| Ok(Value::Bool(matches!(arg(a, 0), Value::Absent))));
    reg!(it, "scala.comparePrimitives", |it, a| Ok(Value::Int(compare_natural(it, &arg(a, 0), &arg(a, 1))?)));
    reg!(it, "scala.compareDoubles", |it, a| {
        let (x, y) = (dbl(it, a, 0)?, dbl(it, a, 1)?);
        Ok(Value::Int(compare_doubles(x, y)))
    });
    reg!(it, "java.util.freshSeed", |it, _a| {
        if it.pure {
            return it.impure("a random seed");
        }
        let mut x = it.seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        it.seed = x;
        Ok(Value::Long((x >> 16) as i64 & 0xffff_ffff_ffff))
    });
    reg!(it, "scala.concurrent.duration.durationLength", |it, a| Ok(Value::Long(long(it, a, 0)?)));
    reg!(it, "java.lang.Throwable.className", |it, a| match arg(a, 0) {
        Value::Obj(o) => Ok(Value::Str(it.runtime_class_name(o.class))),
        other => Ok(Value::string(it.type_name(&other).to_string())),
    });
    reg!(it, "java.lang.printStackTraceImpl", |it, a| {
        if it.pure {
            return it.impure("printStackTrace");
        }
        let s = it.to_str(&arg(a, 0))?;
        it.flush();
        eprintln!("{}", s);
        Ok(Value::Unit)
    });
    // `new String(..)` from chars, a string or nothing: the overloads with a JavaScript template;
    // the byte ones run their Scala bodies, also on the JVM target, whose templates would route
    // them here. The first argument is the `String` object.
    reg!(it, "java.lang.String.newString", |it, a| {
        if let Some(r) = string_from_bytes(it, a) {
            return r;
        }
        let chars = |v: &Value| -> Option<Vec<u16>> {
            match v {
                Value::Array(arr) => Some(arr.borrow().iter().map(|x| match x { Value::Char(c) => *c, Value::Int(i) => *i as u16, _ => 0 }).collect()),
                _ => None,
            }
        };
        match a.get(1..).unwrap_or(&[]) {
            [] => Ok(Value::str("")),
            [Value::Str(s)] => Ok(Value::Str(s.clone())),
            [cs_arr, rest @ ..] => {
                let Some(cs) = chars(cs_arr) else { return it.unsupported("new String of that argument") };
                let slice = match rest {
                    [Value::Int(off), Value::Int(cnt)] => {
                        let (off, cnt, len) = (*off as i64, *cnt as i64, cs.len() as i64);
                        if off < 0 || cnt < 0 || off + cnt > len {
                            return it.throw_named("StringIndexOutOfBoundsException", &format!("offset {}, count {}, length {}", off, cnt, len));
                        }
                        cs[off as usize..(off + cnt) as usize].to_vec()
                    }
                    _ => cs,
                };
                Ok(Value::string(crate::text::from_units(slice)))
            }
        }
    });
    reg!(it, "java.lang.Class.getName", |_it, a| match arg(a, 0) {
        Value::Class(c) => Ok(Value::Str(c.qname.clone())),
        _ => Ok(Value::Null),
    });
    reg!(it, "java.lang.Class.getMethod", |it, a| {
        let Value::Class(c) = arg(a, 0) else { return Ok(Value::Null) };
        let name = it.to_str(&arg(a, 1))?;
        let n = it.typer.interner.intern(&name);
        let found = c.class.map_or(false, |k| !matches!(it.member(k, n), Member::Missing)) || Interp::internal_methods(&c.qname).contains(&name.as_str());
        if !found {
            return it.throw_named("NoSuchMethodException", &format!("{}.{}()", c.qname, name));
        }
        let Some(method) = it.typer.class_at(&["java", "lang", "reflect", "Method"]) else {
            return it.unsupported("java.lang.reflect.Method is missing from the standard library");
        };
        it.construct_new(method, vec![Value::string(name)], &Frame::new(None))
    });
    // The public methods by name: the members of a class of the program, and the internal
    // methods the interpreter answers on the reflection API's values.
    reg!(it, "java.lang.Class.getMethods", |it, a| {
        let Value::Class(c) = arg(a, 0) else { return Ok(Value::array(Vec::new())) };
        let Some(method) = it.typer.class_at(&["java", "lang", "reflect", "Method"]) else {
            return it.unsupported("java.lang.reflect.Method is missing from the standard library");
        };
        let mut names: Vec<String> = Interp::internal_methods(&c.qname).iter().map(|s| s.to_string()).collect();
        if let Some(k) = c.class {
            it.typer.complete_class(k);
            let members: Vec<crate::types::SymId> = it.syms().class(k).member_order.clone();
            names.extend(members.into_iter().map(|m| it.typer.name_ref(it.syms().sym(m).name).to_string()));
        }
        let mut out = Vec::with_capacity(names.len());
        for n in names {
            out.push(it.construct_new(method, vec![Value::string(n)], &Frame::new(None))?);
        }
        Ok(Value::array(out))
    });
    reg!(it, "java.lang.reflect.invokeByName", |it, a| {
        let name = it.to_str(&arg(a, 1))?;
        let args = it.list_items(&arg(a, 2))?;
        if let Some(r) = it.internal_call(&arg(a, 0), &name, &args) {
            return r;
        }
        let n = it.typer.interner.intern(&name);
        it.call_named(arg(a, 0), n, args)
    });
    reg!(it, "java.lang.Class.getSimpleName", |_it, a| match arg(a, 0) {
        Value::Class(c) => Ok(Value::string(Interp::simple_class_name(&c.qname))),
        _ => Ok(Value::Null),
    });
    reg!(it, "java.lang.Class.isInstance", |it, a| match arg(a, 0) {
        Value::Class(c) => {
            let x = arg(a, 1);
            Ok(Value::Bool(match c.class {
                Some(k) => it.is_instance(&x, k),
                None => match it.class_of_value(&x) {
                    Value::Class(other) => other.qname == c.qname,
                    _ => false,
                },
            }))
        }
        _ => Ok(Value::Bool(false)),
    });
    reg!(it, "java.lang.Class.toString", |_it, a| match arg(a, 0) {
        Value::Class(c) => Ok(Value::string(crate::interp::value::class_text(&c.qname))),
        _ => Ok(Value::Null),
    });
    reg!(it, "java.lang.Class.isPrimitive", |_it, a| match arg(a, 0) {
        Value::Class(c) => Ok(Value::Bool(matches!(&*c.qname, "int" | "long" | "double" | "float" | "short" | "byte" | "char" | "boolean" | "void"))),
        _ => Ok(Value::Bool(false)),
    });
    reg!(it, "java.lang.Class.isArray", |_it, a| match arg(a, 0) {
        Value::Class(c) => Ok(Value::Bool(c.qname.starts_with('['))),
        _ => Ok(Value::Bool(false)),
    });
    // The element class of an array is not kept: `Object` stands for it.
    reg!(it, "java.lang.Class.getComponentType", |it, a| match arg(a, 0) {
        Value::Class(_) => Ok(it.class_value_named("java.lang.Object")),
        _ => Ok(Value::Null),
    });
    reg!(it, "java.lang.Class.isAssignableFrom", |it, a| match (arg(a, 0), arg(a, 1)) {
        (Value::Class(to), Value::Class(from)) if &*to.qname == "java.lang.Object" => Ok(Value::Bool(!PRIMITIVE_CLASS_NAMES.contains(&&*from.qname))),
        (Value::Class(to), Value::Class(from)) => Ok(Value::Bool(match (to.class, from.class) {
            (Some(t), Some(f)) => it.is_subclass(f, t),
            _ => to.qname == from.qname,
        })),
        _ => Ok(Value::Bool(false)),
    });
    // js.*: the values JavaScript has that the interpreter models.
    reg!(it, "js.undefined", |_it, _a| Ok(Value::Unit));
    reg!(it, "js.nullValue", |_it, _a| Ok(Value::Null));
    reg!(it, "js.isUndefined", |_it, a| Ok(Value::Bool(matches!(arg(a, 0), Value::Unit | Value::Absent))));
    reg!(it, "js.isNull", |_it, a| Ok(Value::Bool(matches!(arg(a, 0), Value::Null))));
    reg!(it, "js.typeOf", |_it, a| {
        Ok(Value::str(match arg(a, 0) {
            Value::Unit | Value::Absent => "undefined",
            Value::Bool(_) => "boolean",
            Value::Long(_) => "bigint",
            v if v.is_number() => "number",
            Value::Str(_) | Value::Char(_) => "string",
            Value::Fun(_) => "function",
            _ => "object",
        }))
    });
    reg!(it, "js.cast", |_it, a| Ok(arg(a, 0)));
    reg!(it, "js.stringOf", |it, a| Ok(Value::string(it.to_str(&arg(a, 0))?)));
    reg!(it, "js.array", |it, a| {
        let items = it.elements_of(&arg(a, 0))?;
        Ok(Value::array(items))
    });
    reg!(it, "js.arrayFrom", |it, a| {
        let items = it.elements_of(&arg(a, 0))?;
        Ok(Value::array(items))
    });
    for name in ["js.globalThis", "js.global", "js.obj", "js.get", "js.set", "js.call", "js.apply", "js.construct"] {
        reg!(it, name, |it, _a| it.unsupported("JavaScript interop is not executable by the interpreter"));
    }
}

fn install_strings(it: &mut Table) {
    reg!(it, "scala.String.length", |it, a| Ok(Value::Int(utf16_len(&text(it, a, 0)?) as i32)));
    reg!(it, "scala.String.size", |it, a| Ok(Value::Int(utf16_len(&text(it, a, 0)?) as i32)));
    for name in ["scala.String.apply", "scala.String.charAt"] {
        reg!(it, name, |it, a| {
            let s = text(it, a, 0)?;
            let i = int(it, a, 1)?;
            match if i >= 0 { char_at(&s, i as usize) } else { None } {
                Some(c) => Ok(Value::Char(c)),
                None => it.throw_named("StringIndexOutOfBoundsException", &format!("Index {} out of bounds for length {}", i, utf16_len(&s))),
            }
        });
    }
    reg!(it, "scala.String.codePointAt", |it, a| {
        let s = text(it, a, 0)?;
        let i = int(it, a, 1)?;
        let units: Vec<u16> = crate::text::utf16_units(&s).collect();
        if i < 0 || i as usize >= units.len() {
            return it.throw_named("StringIndexOutOfBoundsException", &format!("Index {} out of bounds for length {}", i, units.len()));
        }
        let (hi, lo) = (units[i as usize] as u32, units.get(i as usize + 1).copied().unwrap_or(0) as u32);
        let cp = if (0xd800..0xdc00).contains(&hi) && (0xdc00..0xe000).contains(&lo) { 0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00) } else { hi };
        Ok(Value::Int(cp as i32))
    });
    reg!(it, "scala.String.codePointCount", |it, a| {
        let s = text(it, a, 0)?;
        let (from, to) = (int(it, a, 1)?, int(it, a, 2)?);
        let len = utf16_len(&s) as i32;
        if from < 0 || to > len || from > to {
            return it.throw_named("IndexOutOfBoundsException", &format!("Range [{}, {}) out of bounds for length {}", from, to, len));
        }
        // The units, a high surrogate before a low one counting once, as two stand-ins do too.
        let units: Vec<u16> = crate::text::utf16_units(&utf16_slice(&s, from as usize, to as usize)).collect();
        let pairs = units.windows(2).filter(|w| (0xD800..0xDC00).contains(&w[0]) && (0xDC00..0xE000).contains(&w[1])).count();
        Ok(Value::Int((units.len() - pairs) as i32))
    });
    reg!(it, "scala.String.substring", |it, a| {
        let s = text(it, a, 0)?;
        let len = utf16_len(&s) as i32;
        let start = int(it, a, 1)?;
        let end = if a.len() > 2 { int(it, a, 2)? } else { len };
        if start < 0 || end > len || start > end {
            return it.throw_named("StringIndexOutOfBoundsException", &format!("Range [{}, {}) out of bounds for length {}", start, end, len));
        }
        Ok(Value::str(&utf16_slice(&s, start as usize, end as usize)))
    });
    reg!(it, "scala.String.drop", |it, a| {
        let s = text(it, a, 0)?;
        let n = int(it, a, 1)?.max(0) as usize;
        Ok(Value::str(&utf16_slice(&s, n.min(utf16_len(&s)), utf16_len(&s))))
    });
    reg!(it, "scala.String.take", |it, a| {
        let s = text(it, a, 0)?;
        let n = int(it, a, 1)?.max(0) as usize;
        Ok(Value::str(&utf16_slice(&s, 0, n.min(utf16_len(&s)))))
    });
    reg!(it, "scala.String.dropRight", |it, a| {
        let s = text(it, a, 0)?;
        let len = utf16_len(&s);
        let n = int(it, a, 1)?.max(0) as usize;
        Ok(Value::str(&utf16_slice(&s, 0, len.saturating_sub(n))))
    });
    reg!(it, "scala.String.takeRight", |it, a| {
        let s = text(it, a, 0)?;
        let len = utf16_len(&s);
        let n = int(it, a, 1)?.max(0) as usize;
        Ok(Value::str(&utf16_slice(&s, len.saturating_sub(n), len)))
    });
    reg!(it, "scala.String.indexOf", |it, a| {
        let s = text(it, a, 0)?;
        let Some(part) = code_point_or_text(it, a, 1)? else { return Ok(Value::Int(-1)) };
        let from = if a.len() > 2 { int(it, a, 2)?.max(0) as usize } else { 0 };
        Ok(Value::Int(index_of(&s, &part, from)))
    });
    reg!(it, "scala.String.lastIndexOf", |it, a| {
        let s = text(it, a, 0)?;
        let Some(part) = code_point_or_text(it, a, 1)? else { return Ok(Value::Int(-1)) };
        if a.len() > 2 {
            return Ok(Value::Int(last_index_of_from(&s, &part, int(it, a, 2)?)));
        }
        Ok(Value::Int(last_index_of(&s, &part)))
    });
    reg!(it, "scala.String.contains", |it, a| {
        let s = text(it, a, 0)?;
        let part = text(it, a, 1)?;
        Ok(Value::Bool(if cuts_pairs(&part) { index_of(&s, &part, 0) >= 0 } else { s.contains(&*part) }))
    });
    reg!(it, "scala.String.startsWith", |it, a| {
        let s = text(it, a, 0)?;
        let part = text(it, a, 1)?;
        Ok(Value::Bool(starts_with(&s, &part)))
    });
    reg!(it, "scala.String.endsWith", |it, a| {
        let s = text(it, a, 0)?;
        let part = text(it, a, 1)?;
        Ok(Value::Bool(ends_with(&s, &part)))
    });
    reg!(it, "scala.String.toUpperCase", |it, a| Ok(Value::string(text(it, a, 0)?.to_uppercase())));
    reg!(it, "scala.String.toLowerCase", |it, a| Ok(Value::string(text(it, a, 0)?.to_lowercase())));
    reg!(it, "scala.String.trim", |it, a| Ok(Value::str(java_trim(&text(it, a, 0)?))));
    reg!(it, "scala.String.strip", |it, a| Ok(Value::str(java_strip(&text(it, a, 0)?))));
    reg!(it, "scala.String.stripTrailing", |it, a| Ok(Value::str(text(it, a, 0)?.trim_end_matches(is_java_whitespace))));
    reg!(it, "scala.String.stripLeading", |it, a| Ok(Value::str(text(it, a, 0)?.trim_start_matches(is_java_whitespace))));
    reg!(it, "scala.String.intern", |it, a| Ok(Value::string(text(it, a, 0)?.to_string())));
    reg!(it, "scala.String.isBlank", |it, a| Ok(Value::Bool(java_strip(&text(it, a, 0)?).is_empty())));
    for name in ["scala.String.++", "scala.String.concat"] {
        reg!(it, name, |it, a| {
            let mut s = text(it, a, 0)?.to_string();
            crate::text::push_str(&mut s, &text(it, a, 1)?);
            Ok(Value::string(s))
        });
    }
    reg!(it, "scala.String.capitalize", |it, a| {
        let s = text(it, a, 0)?;
        let mut chars = s.chars();
        Ok(match chars.next() {
            Some(c) => Value::string(format!("{}{}", c.to_uppercase(), chars.as_str())),
            None => Value::Str(s),
        })
    });
    reg!(it, "scala.String.isEmpty", |it, a| Ok(Value::Bool(text(it, a, 0)?.is_empty())));
    reg!(it, "scala.String.nonEmpty", |it, a| Ok(Value::Bool(!text(it, a, 0)?.is_empty())));
    reg!(it, "scala.String.split", |it, a| {
        let s = text(it, a, 0)?;
        let sep = text(it, a, 1)?;
        let limit = if a.len() > 2 { int(it, a, 2)? } else { 0 };
        let parts = it.split_string(&s, &sep, limit)?;
        Ok(Value::array(parts.into_iter().map(Value::string).collect()))
    });
    reg!(it, "scala.String.replaceAll", |it, a| {
        let s = text(it, a, 0)?;
        let re = text(it, a, 1)?;
        let repl = text(it, a, 2)?;
        Ok(Value::string(it.regex_replace(&re, &s, &Value::Str(repl), true, None)?))
    });
    reg!(it, "scala.String.replaceFirst", |it, a| {
        let s = text(it, a, 0)?;
        let re = text(it, a, 1)?;
        let repl = text(it, a, 2)?;
        Ok(Value::string(it.regex_replace(&re, &s, &Value::Str(repl), false, None)?))
    });
    reg!(it, "scala.String.matches", |it, a| {
        let s = text(it, a, 0)?;
        let re = text(it, a, 1)?;
        Ok(Value::Bool(it.regex_exec(&re, "f", &s)?.is_some()))
    });
    reg!(it, "scala.String.equalsIgnoreCase", |it, a| {
        let s = text(it, a, 0)?;
        let t = text(it, a, 1)?;
        Ok(Value::Bool(s.to_lowercase() == t.to_lowercase()))
    });
    reg!(it, "scala.String.compareToIgnoreCase", |it, a| {
        let s = text(it, a, 0)?;
        let t = text(it, a, 1)?;
        Ok(Value::Int(compare_strings(&s.to_lowercase(), &t.to_lowercase())))
    });
    reg!(it, "scala.String.stripPrefix", |it, a| {
        let s = text(it, a, 0)?;
        let p = text(it, a, 1)?;
        Ok(match s.strip_prefix(&*p) {
            Some(rest) if !cuts_pairs(&p) => Value::str(rest),
            _ if starts_with(&s, &p) => Value::str(&utf16_slice(&s, utf16_len(&p), utf16_len(&s))),
            _ => Value::Str(s),
        })
    });
    reg!(it, "scala.String.stripSuffix", |it, a| {
        let s = text(it, a, 0)?;
        let p = text(it, a, 1)?;
        Ok(match s.strip_suffix(&*p) {
            Some(rest) if !cuts_pairs(&p) => Value::str(rest),
            _ if ends_with(&s, &p) => Value::str(&utf16_slice(&s, 0, utf16_len(&s) - utf16_len(&p))),
            _ => Value::Str(s),
        })
    });
    reg!(it, "scala.String.repeat", |it, a| {
        let s = text(it, a, 0)?;
        let n = int(it, a, 1)?;
        if n < 0 {
            return it.throw_named("IllegalArgumentException", &format!("count is negative: {}", n));
        }
        Ok(Value::string(repeat(&s, n as usize)))
    });
    reg!(it, "scala.String.*", |it, a| {
        let s = text(it, a, 0)?;
        let n = int(it, a, 1)?;
        Ok(Value::string(repeat(&s, n.max(0) as usize)))
    });
    reg!(it, "scala.String.slice", |it, a| {
        let s = text(it, a, 0)?;
        let (from, until) = (int(it, a, 1)?.max(0), int(it, a, 2)?.max(0));
        let (lo, hi) = slice_bounds(utf16_len(&s), from, until);
        Ok(Value::str(&utf16_slice(&s, lo, hi)))
    });
    // `getChars(srcBegin, srcEnd, dst, dstBegin)`: the UTF-16 units, the JDK's bounds checked.
    reg!(it, "scala.String.getChars", |it, a| {
        let s = text(it, a, 0)?;
        let (begin, end) = (int(it, a, 1)?, int(it, a, 2)?);
        let dst = array(it, a, 3)?;
        let at = int(it, a, 4)?;
        let units: Vec<u16> = crate::text::utf16_units(&s).collect();
        let n = units.len() as i32;
        if begin < 0 || begin > end || end > n {
            return it.throw_named("StringIndexOutOfBoundsException", &format!("begin {}, end {}, length {}", begin, end, n));
        }
        let len = dst.borrow().len() as i32;
        if at < 0 || at + (end - begin) > len {
            return it.throw_named("ArrayIndexOutOfBoundsException", &format!("Range [{}, {} + {}) out of bounds for length {}", at, at, end - begin, len));
        }
        let mut items = dst.write();
        for (i, &u) in units[begin as usize..end as usize].iter().enumerate() {
            items[at as usize + i] = Value::Char(u);
        }
        Ok(Value::Unit)
    });
    reg!(it, "scala.String.toCharArray", |it, a| {
        let s = text(it, a, 0)?;
        Ok(Value::array(crate::text::utf16_units(&s).map(Value::Char).collect()))
    });
    reg!(it, "scala.String.formatImpl", |it, a| {
        let s = text(it, a, 0)?;
        let args = array(it, a, 1)?;
        let items = args.borrow().clone();
        Ok(Value::string(it.format(&s, &items)?))
    });
    reg!(it, "scala.String.replace", |it, a| {
        let s = text(it, a, 0)?;
        let target = text(it, a, 1)?;
        let repl = text(it, a, 2)?;
        Ok(Value::string(replace(&s, &target, &repl)))
    });
    reg!(it, "scala.String.toInt", |it, a| {
        let s = text(it, a, 0)?;
        match int_or_null(&s) {
            Value::Null => number_format(it, &s),
            v => Ok(v),
        }
    });
    reg!(it, "scala.String.toLong", |it, a| {
        let s = text(it, a, 0)?;
        match long_or_null(&s) {
            Value::Null => number_format(it, &s),
            v => Ok(v),
        }
    });
    reg!(it, "scala.String.toDouble", |it, a| {
        let s = text(it, a, 0)?;
        match parse_java_double(&s) {
            Some(d) => Ok(Value::Double(d)),
            None => number_format(it, &s),
        }
    });
    reg!(it, "scala.String.toByte", |it, a| {
        let s = text(it, a, 0)?;
        match int_or_null(&s) {
            Value::Int(i) if i >= -128 && i <= 127 => Ok(Value::Byte(i as i8)),
            _ => number_format(it, &s),
        }
    });
    reg!(it, "scala.String.toShort", |it, a| {
        let s = text(it, a, 0)?;
        match int_or_null(&s) {
            Value::Int(i) if i >= -32768 && i <= 32767 => Ok(Value::Short(i as i16)),
            _ => number_format(it, &s),
        }
    });
    reg!(it, "scala.String.reverse", |it, a| {
        let s = text(it, a, 0)?;
        Ok(Value::string(reverse(&s)))
    });
    reg!(it, "scala.String.linesArray", |it, a| {
        let s = text(it, a, 0)?;
        let parts: Vec<Value> = s.split('\n').map(|l| Value::str(l.strip_suffix('\r').unwrap_or(l))).collect();
        Ok(Value::array(parts))
    });
    for name in ["scala.String.compareTo", "scala.String.compare"] {
        reg!(it, name, |it, a| {
            let s = text(it, a, 0)?;
            let t = text(it, a, 1)?;
            Ok(Value::Int(compare_strings(&s, &t)))
        });
    }
    reg!(it, "scala.String.stripMargin", |it, a| {
        let s = text(it, a, 0)?;
        let margin = if a.len() > 1 { chr(it, a, 1)? } else { '|' };
        let out: Vec<String> = s
            .split('\n')
            .map(|line| match line.find(margin) {
                Some(i) if line[..i].trim().is_empty() => line[i + margin.len_utf8()..].to_string(),
                _ => line.to_string(),
            })
            .collect();
        Ok(Value::string(out.join("\n")))
    });
    reg!(it, "scala.intOrNull", |it, a| Ok(int_or_null(&text(it, a, 0)?)));
    reg!(it, "scala.longOrNull", |it, a| Ok(long_or_null(&text(it, a, 0)?)));
    reg!(it, "scala.doubleOrNull", |it, a| {
        let s = text(it, a, 0)?;
        Ok(parse_java_double(&s).map_or(Value::Null, Value::Double))
    });
    reg!(it, "scala.Char.isDigit", |it, a| Ok(Value::Bool(decimal(unit(it, a, 0)?) >= 0)));
    reg!(it, "scala.Char.isLetter", |it, a| Ok(Value::Bool(is_letter(unit(it, a, 0)?))));
    reg!(it, "scala.Char.isLetterOrDigit", |it, a| {
        let u = unit(it, a, 0)?;
        Ok(Value::Bool(is_letter(u) || decimal(u) >= 0))
    });
    reg!(it, "scala.Char.isWhitespace", |it, a| Ok(Value::Bool(is_java_whitespace(chr(it, a, 0)?))));
    reg!(it, "scala.Char.isControl", |it, a| Ok(Value::Bool(chr(it, a, 0)?.is_control())));
    reg!(it, "scala.Char.isSpaceChar", |it, a| Ok(Value::Bool(is_space_char(chr(it, a, 0)?))));
    reg!(it, "scala.Char.isUpper", |it, a| Ok(Value::Bool(chr(it, a, 0)?.is_uppercase())));
    reg!(it, "scala.Char.isLower", |it, a| Ok(Value::Bool(chr(it, a, 0)?.is_lowercase())));
    reg!(it, "scala.Char.toUpper", |_it, a| Ok(map_char(&arg(a, 0), |c| c.to_uppercase().collect())));
    reg!(it, "scala.Char.toLower", |_it, a| Ok(map_char(&arg(a, 0), |c| c.to_lowercase().collect())));
    reg!(it, "scala.Char.asDigit", |it, a| Ok(Value::Int(digit(unit(it, a, 0)?, 36))));
    reg!(it, "scala.StringContext.processEscapes", |it, a| {
        let s = text(it, a, 1)?;
        Ok(Value::string(it.process_escapes(&s)?))
    });
    reg!(it, "scala.StringContext.formatInterpolated", |it, a| {
        let parts = array(it, a, 1)?.borrow().clone();
        let args = array(it, a, 2)?.borrow().clone();
        Ok(Value::string(it.format_interpolated(&parts, &args)?))
    });
}

fn install_numbers(it: &mut Table) {
    reg!(it, "scala.Int.toHexString", |it, a| Ok(Value::string(format!("{:x}", int(it, a, 0)? as u32))));
    reg!(it, "scala.Int.toOctalString", |it, a| Ok(Value::string(format!("{:o}", int(it, a, 0)? as u32))));
    reg!(it, "scala.Int.toBinaryString", |it, a| Ok(Value::string(format!("{:b}", int(it, a, 0)? as u32))));
    reg!(it, "scala.Long.toHexString", |it, a| Ok(Value::string(format!("{:x}", long(it, a, 0)? as u64))));
    reg!(it, "scala.Long.toOctalString", |it, a| Ok(Value::string(format!("{:o}", long(it, a, 0)? as u64))));
    reg!(it, "scala.Long.toBinaryString", |it, a| Ok(Value::string(format!("{:b}", long(it, a, 0)? as u64))));
    reg!(it, "scala.Double.round", |it, a| Ok(Value::Long(java_round(dbl(it, a, 0)?))));
    reg!(it, "scala.Double.sign", |_it, a| Ok(num_signum(&arg(a, 0))));
    reg!(it, "scala.Double.signum", |it, a| {
        let d = dbl(it, a, 0)?;
        Ok(Value::Int(if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 }))
    });
    reg!(it, "scala.Double.isFinite", |it, a| Ok(Value::Bool(dbl(it, a, 0)?.is_finite())));
    reg!(it, "scala.Double.isInfinite", |it, a| Ok(Value::Bool(dbl(it, a, 0)?.is_infinite())));
    reg!(it, "scala.Double.isNaN", |it, a| Ok(Value::Bool(dbl(it, a, 0)?.is_nan())));
    reg!(it, "scala.Double.isWhole", |it, a| {
        let d = dbl(it, a, 0)?;
        Ok(Value::Bool(d.is_finite() && d == d.trunc()))
    });
    reg!(it, "scala.Double.toDegrees", |it, a| Ok(Value::Double(dbl(it, a, 0)?.to_degrees())));
    reg!(it, "scala.Double.toRadians", |it, a| Ok(Value::Double(dbl(it, a, 0)?.to_radians())));
    reg!(it, "scala.Double.floor", |it, a| Ok(Value::Double(dbl(it, a, 0)?.floor())));
    reg!(it, "scala.Double.ceil", |it, a| Ok(Value::Double(dbl(it, a, 0)?.ceil())));
    reg!(it, "scala.parseIntRadix", |it, a| {
        let s = text(it, a, 0)?;
        let radix = int(it, a, 1)?;
        parse_int_radix(it, &s, radix as u32)
    });
    reg!(it, "scala.Float.PositiveInfinity", |_it, _a| Ok(Value::Float(f32::INFINITY)));
    reg!(it, "scala.Float.NegativeInfinity", |_it, _a| Ok(Value::Float(f32::NEG_INFINITY)));
    reg!(it, "scala.Float.NaN", |_it, _a| Ok(Value::Float(f32::NAN)));
    reg!(it, "scala.Double.PositiveInfinity", |_it, _a| Ok(Value::Double(f64::INFINITY)));
    reg!(it, "scala.Double.NegativeInfinity", |_it, _a| Ok(Value::Double(f64::NEG_INFINITY)));
    reg!(it, "scala.Double.NaN", |_it, _a| Ok(Value::Double(f64::NAN)));

    // scala.math and the std's object Math take the same operations, the object with its
    // module as the first argument.
    let unary: &[(&str, fn(f64) -> f64)] = &[
        ("sqrt", f64::sqrt),
        ("floor", f64::floor),
        ("ceil", f64::ceil),
        ("log", f64::ln),
        ("log10", f64::log10),
        ("exp", f64::exp),
        ("sin", f64::sin),
        ("cos", f64::cos),
        ("tan", f64::tan),
        ("cbrt", f64::cbrt),
        ("asin", f64::asin),
        ("acos", f64::acos),
        ("atan", f64::atan),
        ("sinh", f64::sinh),
        ("cosh", f64::cosh),
        ("tanh", f64::tanh),
        ("log1p", f64::ln_1p),
        ("expm1", f64::exp_m1),
        ("rint", rint),
    ];
    for &(name, f) in unary {
        let f = Rc::new(f);
        let g = f.clone();
        it.insert(format!("scala.math.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(f(dbl(it, a, 0)?)))));
        let h = g.clone();
        it.insert(format!("scala.Math.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(g(dbl(it, a, 1)?)))));
        it.insert(format!("java.lang.Math.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(h(dbl(it, a, 1)?)))));
    }
    let binary: &[(&str, fn(f64, f64) -> f64)] = &[("pow", f64::powf), ("atan2", f64::atan2), ("hypot", f64::hypot)];
    for &(name, f) in binary {
        let f = Rc::new(f);
        let g = f.clone();
        it.insert(format!("scala.math.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(f(dbl(it, a, 0)?, dbl(it, a, 1)?)))));
        let h = g.clone();
        it.insert(format!("scala.Math.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(g(dbl(it, a, 1)?, dbl(it, a, 2)?)))));
        it.insert(format!("java.lang.Math.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(h(dbl(it, a, 1)?, dbl(it, a, 2)?)))));
    }
    for (prefix, off) in [("scala.math.", 0usize), ("scala.Math.", 1), ("java.lang.Math.", 1)] {
        it.insert(format!("{}max", prefix), Rc::new(move |_it: &mut Interp<'_, '_>, a: &[Value]| Ok(num_max(&arg(a, off), &arg(a, off + 1)))));
        it.insert(format!("{}min", prefix), Rc::new(move |_it: &mut Interp<'_, '_>, a: &[Value]| Ok(num_min(&arg(a, off), &arg(a, off + 1)))));
        it.insert(format!("{}abs", prefix), Rc::new(move |_it: &mut Interp<'_, '_>, a: &[Value]| Ok(num_abs(&arg(a, off)))));
        it.insert(format!("{}signum", prefix), Rc::new(move |_it: &mut Interp<'_, '_>, a: &[Value]| Ok(num_signum(&arg(a, off)))));
        it.insert(format!("{}floorDiv", prefix), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| num_floor_div(it, &arg(a, off), &arg(a, off + 1))));
        it.insert(format!("{}floorMod", prefix), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| {
            let (x, y) = (arg(a, off), arg(a, off + 1));
            let m = num_floor_mod(it, &x, &y)?;
            // Java's `floorMod(long, int)` is an `int`; the std's generic one takes the wider kind.
            match (prefix, &y, m) {
                ("java.lang.Math.", Value::Int(_), Value::Long(l)) => Ok(Value::Int(l as i32)),
                (_, _, m) => Ok(m),
            }
        }));
        it.insert(format!("{}round", prefix), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Long(java_round(dbl(it, a, off)?)))));
        it.insert(format!("{}random", prefix), Rc::new(move |it: &mut Interp<'_, '_>, _a: &[Value]| {
            if it.pure {
                return it.impure("a random number");
            }
            let mut x = it.seed;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            it.seed = x;
            Ok(Value::Double((x >> 11) as f64 / (1u64 << 53) as f64))
        }));
    }
    reg!(it, "java.lang.Math.nextUp", |it, a| Ok(Value::Double(dbl(it, a, 1)?.next_up())));
    reg!(it, "java.lang.Math.nextDown", |it, a| Ok(Value::Double(dbl(it, a, 1)?.next_down())));
    reg!(it, "java.lang.Math.PI", |_it, _a| Ok(Value::Double(std::f64::consts::PI)));
    reg!(it, "java.lang.Math.E", |_it, _a| Ok(Value::Double(std::f64::consts::E)));
    reg!(it, "java.lang.Math.addExact", |it, a| exact(it, a, i64::checked_add));
    reg!(it, "java.lang.Math.subtractExact", |it, a| exact(it, a, i64::checked_sub));
    reg!(it, "java.lang.Math.multiplyExact", |it, a| exact(it, a, i64::checked_mul));
    reg!(it, "java.lang.Math.negateExact", |it, a| {
        let zero = if matches!(arg(a, 1), Value::Long(_)) { Value::Long(0) } else { Value::Int(0) };
        exact(it, &[Value::Null, zero, arg(a, 1)], i64::checked_sub)
    });
    reg!(it, "java.lang.Math.toIntExact", |it, a| {
        let l = long(it, a, 1)?;
        if l < i32::MIN as i64 || l > i32::MAX as i64 {
            return it.throw_named("ArithmeticException", "integer overflow");
        }
        Ok(Value::Int(l as i32))
    });
}

fn install_collections(it: &mut Table) {
    reg!(it, "scala.emptyArray", |_it, _a| Ok(Value::array(Vec::new())));
    reg!(it, "scala.Array.empty", |_it, _a| Ok(Value::array(Vec::new())));
    reg!(it, "scala.Array.apply", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?;
        let v = arr.borrow().get(i.max(0) as usize).cloned();
        match v {
            Some(v) if i >= 0 => Ok(v),
            _ => it.throw_named("IndexOutOfBoundsException", &format!("Index {} out of bounds for length {}", i, arr.borrow().len())),
        }
    });
    reg!(it, "scala.Array.update", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?;
        let len = arr.borrow().len();
        if i < 0 || i as usize >= len {
            return it.throw_named("IndexOutOfBoundsException", &format!("Index {} out of bounds for length {}", i, len));
        }
        arr.write()[i as usize] = arg(a, 2);
        Ok(Value::Unit)
    });
    for name in ["scala.Array.length", "scala.Array.size"] {
        reg!(it, name, |it, a| Ok(Value::Int(array(it, a, 0)?.borrow().len() as i32)));
    }
    for name in ["scala.Array.clone", "scala.copyArray"] {
        reg!(it, name, |it, a| Ok(Value::array(array(it, a, 0)?.borrow().clone())));
    }
    reg!(it, "scala.Array.map", |it, a| {
        let items = array(it, a, 0)?.borrow().clone();
        let f = arg(a, 1);
        let mut out = Vec::with_capacity(items.len());
        for x in items {
            out.push(it.apply_value(f.clone(), vec![x])?);
        }
        Ok(Value::array(out))
    });
    reg!(it, "scala.Array.filter", |it, a| {
        let items = array(it, a, 0)?.borrow().clone();
        let p = arg(a, 1);
        let mut out = Vec::new();
        for x in items {
            if it.apply_value(p.clone(), vec![x.clone()])?.as_bool().unwrap_or(false) {
                out.push(x);
            }
        }
        Ok(Value::array(out))
    });
    reg!(it, "scala.Array.push", |it, a| {
        let arr = array(it, a, 0)?;
        arr.write().push(arg(a, 1));
        let n = arr.borrow().len();
        Ok(Value::Int(n as i32))
    });
    reg!(it, "scala.newArray", |it, a| {
        let n = int(it, a, 0)?.max(0);
        Ok(Value::array(vec![arg(a, 1); n as usize]))
    });
    reg!(it, "scala.Array.fill", |it, a| {
        let n = int(it, a, 1)?.max(0);
        let thunk = arg(a, 2);
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            out.push(it.apply_value(thunk.clone(), Vec::new())?);
        }
        Ok(Value::array(out))
    });
    reg!(it, "scala.Array.tabulate", |it, a| {
        let n = int(it, a, 1)?.max(0);
        let f = arg(a, 2);
        let mut out = Vec::with_capacity(n as usize);
        for i in 0..n {
            out.push(it.apply_value(f.clone(), vec![Value::Int(i)])?);
        }
        Ok(Value::array(out))
    });
    reg!(it, "scala.arraySlice", |it, a| {
        let arr = array(it, a, 0)?;
        let (from, until) = (int(it, a, 1)?, int(it, a, 2)?);
        let items = arr.borrow();
        let (lo, hi) = slice_bounds(items.len(), from, until);
        Ok(Value::array(items[lo..hi].to_vec()))
    });
    reg!(it, "scala.reverseArray", |it, a| {
        array(it, a, 0)?.write().reverse();
        Ok(Value::Unit)
    });
    reg!(it, "scala.arrayOfOne", |_it, a| Ok(Value::array(vec![arg(a, 0)])));
    reg!(it, "scala.sortArray", |it, a| {
        let arr = array(it, a, 0)?;
        let cmp = arg(a, 1);
        let mut items = arr.borrow().clone();
        sort_values(it, &mut items, &mut |it, x, y| {
            let r = it.apply_value(cmp.clone(), vec![x.clone(), y.clone()])?;
            Ok(r.as_i32().unwrap_or(0))
        })?;
        *arr.write() = items;
        Ok(Value::Unit)
    });
    reg!(it, "scala.util.hashing.MurmurHash3.mix", |_it, a| Ok(Value::Int(mix(arg(a, 1).as_i32().unwrap_or(0), arg(a, 2).as_i32().unwrap_or(0)))));
    reg!(it, "scala.util.hashing.MurmurHash3.mixLast", |_it, a| Ok(Value::Int(mix_last(arg(a, 1).as_i32().unwrap_or(0), arg(a, 2).as_i32().unwrap_or(0)))));
    reg!(it, "scala.util.hashing.MurmurHash3.finalizeHash", |_it, a| Ok(Value::Int(finalize_hash(arg(a, 1).as_i32().unwrap_or(0), arg(a, 2).as_i32().unwrap_or(0)))));
    reg!(it, "scala.util.hashing.MurmurHash3.avalanche", |_it, a| Ok(Value::Int(avalanche(arg(a, 1).as_i32().unwrap_or(0)))));
    reg!(it, "scala.seqHash", |it, a| Ok(Value::Int(seq_hash(it, &arg(a, 0))?)));
    reg!(it, "scala.setHash", |it, a| {
        let items = it.elements_of(&arg(a, 0))?;
        let (mut x, mut y, mut z, mut n) = (0i32, 0i32, 1i32, 0i32);
        for item in &items {
            let k = it.hash(item)?;
            x = x.wrapping_add(k);
            y ^= k;
            z = z.wrapping_mul(k | 1);
            n += 1;
        }
        Ok(Value::Int(unordered_hash(x, y, z, n, str_hash("Set"))))
    });
    // The platform layer's own array and hash-map primitives (`std/javalib/util.scala`).
    reg!(it, "java.util.newArray", |_it, _a| Ok(Value::array(Vec::new())));
    reg!(it, "java.util.arrayPush", |it, a| {
        array(it, a, 0)?.write().push(arg(a, 1));
        Ok(Value::Unit)
    });
    reg!(it, "java.util.arrayInsert", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?.max(0) as usize;
        let mut items = arr.write();
        let at = i.min(items.len());
        items.insert(at, arg(a, 2));
        Ok(Value::Unit)
    });
    reg!(it, "java.util.arrayRemove", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?;
        let mut items = arr.write();
        if i >= 0 && (i as usize) < items.len() {
            items.remove(i as usize);
        }
        Ok(Value::Unit)
    });
    reg!(it, "java.util.arrayClear", |it, a| {
        array(it, a, 0)?.write().clear();
        Ok(Value::Unit)
    });
    reg!(it, "java.util.newStore", |_it, _a| Ok(Value::map(HMap::default())));
    reg!(it, "java.util.Store.storeHas", |it, a| {
        let m = map(it, a, 0)?;
        Ok(Value::Bool(it.map_has(&m, &arg(a, 1))?))
    });
    reg!(it, "java.util.Store.storeGet", |it, a| {
        let m = map(it, a, 0)?;
        it.map_get(&m, &arg(a, 1))
    });
    reg!(it, "java.util.Store.storeSet", |it, a| {
        let m = map(it, a, 0)?;
        it.map_set(&m, arg(a, 1), arg(a, 2))?;
        Ok(Value::Unit)
    });
    reg!(it, "java.util.Store.storeDelete", |it, a| {
        let m = map(it, a, 0)?;
        Ok(Value::Bool(it.map_delete(&m, &arg(a, 1))?))
    });
    reg!(it, "java.util.Store.storeKeys", |it, a| Ok(Value::array(Interp::map_keys(&map(it, a, 0)?))));
    reg!(it, "java.util.Store.storeValues", |it, a| Ok(Value::array(Interp::map_values(&map(it, a, 0)?))));
    reg!(it, "java.util.Store.storeSize", |it, a| Ok(Value::Int(Interp::map_size(&map(it, a, 0)?) as i32)));
    reg!(it, "java.util.Store.storeClear", |it, a| {
        Interp::map_clear(&map(it, a, 0)?);
        Ok(Value::Unit)
    });
    reg!(it, "scala.collection.mutable.bufferInsert", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?.max(0) as usize;
        let mut items = arr.write();
        let at = i.min(items.len());
        items.insert(at, arg(a, 2));
        Ok(Value::Unit)
    });
    reg!(it, "scala.collection.mutable.bufferRemove", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?;
        let mut items = arr.write();
        if i >= 0 && (i as usize) < items.len() {
            items.remove(i as usize);
        }
        Ok(Value::Unit)
    });
    reg!(it, "scala.collection.mutable.bufferClear", |it, a| {
        array(it, a, 0)?.write().clear();
        Ok(Value::Unit)
    });
    reg!(it, "scala.collection.mutable.bufferInsertAll", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?.max(0) as usize;
        let values = array(it, a, 2)?.borrow().clone();
        let mut items = arr.write();
        let at = i.min(items.len());
        items.splice(at..at, values);
        Ok(Value::Unit)
    });
    reg!(it, "scala.collection.mutable.bufferRemoveRange", |it, a| {
        let arr = array(it, a, 0)?;
        let i = int(it, a, 1)?.max(0) as usize;
        let count = int(it, a, 2)?.max(0) as usize;
        let mut items = arr.write();
        let at = i.min(items.len());
        let end = (at + count).min(items.len());
        items.drain(at..end);
        Ok(Value::Unit)
    });
    // RawMap.
    reg!(it, "scala.newRawMap", |_it, _a| Ok(Value::map(HMap::default())));
    reg!(it, "scala.RawMap.rawHas", |it, a| {
        let m = map(it, a, 0)?;
        Ok(Value::Bool(it.map_has(&m, &arg(a, 1))?))
    });
    reg!(it, "scala.RawMap.rawGet", |it, a| {
        let m = map(it, a, 0)?;
        it.map_get(&m, &arg(a, 1))
    });
    reg!(it, "scala.RawMap.rawSet", |it, a| {
        let m = map(it, a, 0)?;
        it.map_set(&m, arg(a, 1), arg(a, 2))?;
        Ok(Value::Unit)
    });
    reg!(it, "scala.RawMap.rawDelete", |it, a| {
        let m = map(it, a, 0)?;
        Ok(Value::Bool(it.map_delete(&m, &arg(a, 1))?))
    });
    reg!(it, "scala.RawMap.rawKeys", |it, a| Ok(Value::array(Interp::map_keys(&map(it, a, 0)?))));
    reg!(it, "scala.RawMap.rawValues", |it, a| Ok(Value::array(Interp::map_values(&map(it, a, 0)?))));
    reg!(it, "scala.RawMap.rawSize", |it, a| Ok(Value::Int(Interp::map_size(&map(it, a, 0)?) as i32)));
    reg!(it, "scala.RawMap.rawClear", |it, a| {
        Interp::map_clear(&map(it, a, 0)?);
        Ok(Value::Unit)
    });
    reg!(it, "scala.RawMap.rawForeach", |it, a| {
        let m = map(it, a, 0)?;
        let f = arg(a, 1);
        for (k, v) in Interp::map_entries(&m) {
            it.apply_value(f.clone(), vec![k, v])?;
        }
        Ok(Value::Unit)
    });
    reg!(it, "scala.keyEquals", |it, a| Ok(Value::Bool(it.equal(&arg(a, 0), &arg(a, 1))?)));
    reg!(it, "scala.rawMapEquals", |it, a| {
        let (x, y) = (map(it, a, 0)?, map(it, a, 1)?);
        Ok(Value::Bool(it.map_equals(&x, &y)?))
    });
    reg!(it, "scala.rawMapHash", |it, a| {
        let m = map(it, a, 0)?;
        Ok(Value::Int(it.map_hash(&m)?))
    });
}

/// The pairs of a `.properties` file as `java.util.Properties.load` reads them: `\r\n`, `\n`
/// and `\r` end a natural line, a logical line continues on the next when it ends in an odd
/// number of backslashes, the next line's leading whitespace dropped; a line whose first
/// character is `#` or `!` is a comment; the key ends at the first unescaped `=`, `:` or
/// whitespace, an `=` or `:` after whitespace still separates; `\uXXXX` (a UTF-16 unit, a
/// surrogate pair over two escapes), `\t`, `\n`, `\r`, `\f` and `\<c>` for `<c>` are the
/// escapes, in keys and values; a key defined twice keeps its last value.
fn parse_properties(text: &str) -> Vec<(String, String)> {
    fn unescape(s: &str) -> String {
        let mut units: Vec<u16> = Vec::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                units.extend(c.encode_utf16(&mut [0; 2]).iter());
                continue;
            }
            match chars.next() {
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    match u16::from_str_radix(&hex, 16) {
                        Ok(u) if hex.len() == 4 => units.push(u),
                        _ => {
                            units.extend("\\u".encode_utf16());
                            units.extend(hex.encode_utf16());
                        }
                    }
                }
                Some('t') => units.push('\t' as u16),
                Some('n') => units.push('\n' as u16),
                Some('r') => units.push('\r' as u16),
                Some('f') => units.push(0xc),
                Some(other) => units.extend(other.encode_utf16(&mut [0; 2]).iter()),
                None => {}
            }
        }
        crate::text::from_units(units)
    }
    let is_blank = |c: char| matches!(c, ' ' | '\t' | '\u{c}');
    let mut out: Vec<(String, String)> = Vec::new();
    let mut put = |full: String| {
        let chars: Vec<char> = full.chars().collect();
        let mut i = 0;
        let mut key_end = chars.len();
        while i < chars.len() {
            match chars[i] {
                '\\' => i += 2,
                c if c == '=' || c == ':' || is_blank(c) => {
                    key_end = i;
                    break;
                }
                _ => i += 1,
            }
        }
        let key_end = key_end.min(chars.len());
        let key: String = chars[..key_end].iter().collect();
        let mut j = key_end;
        while j < chars.len() && is_blank(chars[j]) {
            j += 1;
        }
        if j < chars.len() && (chars[j] == '=' || chars[j] == ':') {
            j += 1;
            while j < chars.len() && is_blank(chars[j]) {
                j += 1;
            }
        }
        let value: String = chars[j.min(chars.len())..].iter().collect();
        let (key, value) = (unescape(&key), unescape(&value));
        match out.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = value,
            None => out.push((key, value)),
        }
    };
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut logical = String::new();
    let mut continued = false;
    for raw in normalized.split('\n') {
        let line = raw.trim_start_matches(is_blank);
        if !continued && (line.is_empty() || line.starts_with('#') || line.starts_with('!')) {
            continue;
        }
        let trailing = line.chars().rev().take_while(|&c| c == '\\').count();
        if trailing % 2 == 1 {
            logical.push_str(&line[..line.len() - 1]);
            continued = true;
            continue;
        }
        logical.push_str(line);
        continued = false;
        put(std::mem::take(&mut logical));
    }
    // A continuation the file ends in: the line so far is a whole one.
    if continued && !logical.is_empty() {
        put(logical);
    }
    out
}

fn install_java(it: &mut Table) {
    // Boxed numbers are the numbers themselves.
    for class in ["Integer", "Long", "Double", "Float", "Number"] {
        it.insert(format!("java.lang.{}.intValue", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Int(int(it, a, 0)?))));
        it.insert(format!("java.lang.{}.longValue", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Long(long(it, a, 0)?))));
        it.insert(format!("java.lang.{}.doubleValue", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Double(dbl(it, a, 0)?))));
        it.insert(format!("java.lang.{}.floatValue", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Float(dbl(it, a, 0)? as f32))));
        it.insert(format!("java.lang.{}.byteValue", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Byte(int(it, a, 0)? as i8))));
        it.insert(format!("java.lang.{}.shortValue", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Short(int(it, a, 0)? as i16))));
        it.insert(format!("java.lang.{}.compareTo", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Int(compare_natural(it, &arg(a, 0), &arg(a, 1))?))));
        it.insert(format!("java.lang.{}.valueOf", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| match arg(a, 1) {
            Value::Str(s) => parse_int_radix(it, &s, 10),
            v => Ok(v),
        }));
        it.insert(format!("java.lang.{}.compare", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| Ok(Value::Int(compare_natural(it, &arg(a, 1), &arg(a, 2))?))));
    }
    reg!(it, "java.lang.Integer.MAX_VALUE", |_it, _a| Ok(Value::Int(i32::MAX)));
    reg!(it, "java.lang.Integer.MIN_VALUE", |_it, _a| Ok(Value::Int(i32::MIN)));
    reg!(it, "java.lang.Integer.SIZE", |_it, _a| Ok(Value::Int(32)));
    reg!(it, "java.lang.Integer.parseInt", |it, a| {
        let s = text(it, a, 1)?;
        let radix = if a.len() > 2 { int(it, a, 2)? } else { 10 };
        parse_int_radix(it, &s, radix as u32)
    });
    reg!(it, "java.lang.Integer.toString", |it, a| {
        let i = int(it, a, 1)?;
        let radix = if a.len() > 2 { int(it, a, 2)? } else { 10 };
        Ok(Value::string(to_radix(i as i64, radix.clamp(2, 36) as u32)))
    });
    reg!(it, "java.lang.Integer.toHexString", |it, a| Ok(Value::string(format!("{:x}", int(it, a, 1)? as u32))));
    reg!(it, "java.lang.Integer.toBinaryString", |it, a| Ok(Value::string(format!("{:b}", int(it, a, 1)? as u32))));
    reg!(it, "java.lang.Integer.toOctalString", |it, a| Ok(Value::string(format!("{:o}", int(it, a, 1)? as u32))));
    reg!(it, "java.lang.Integer.hashCode", |it, a| Ok(Value::Int(int(it, a, 1)?)));
    reg!(it, "java.lang.Integer.compareUnsigned", |it, a| Ok(Value::Int((int(it, a, 1)? as u32).cmp(&(int(it, a, 2)? as u32)) as i32)));
    reg!(it, "java.lang.Integer.divideUnsigned", |it, a| {
        let (x, y) = (int(it, a, 1)? as u32, int(it, a, 2)? as u32);
        if y == 0 {
            return it.throw_named("ArithmeticException", "/ by zero");
        }
        Ok(Value::Int((x / y) as i32))
    });
    reg!(it, "java.lang.Integer.remainderUnsigned", |it, a| {
        let (x, y) = (int(it, a, 1)? as u32, int(it, a, 2)? as u32);
        if y == 0 {
            return it.throw_named("ArithmeticException", "/ by zero");
        }
        Ok(Value::Int((x % y) as i32))
    });
    reg!(it, "java.lang.Integer.bitCount", |it, a| Ok(Value::Int(int(it, a, 1)?.count_ones() as i32)));
    reg!(it, "java.lang.Integer.numberOfLeadingZeros", |it, a| Ok(Value::Int(int(it, a, 1)?.leading_zeros() as i32)));
    reg!(it, "java.lang.Integer.numberOfTrailingZeros", |it, a| Ok(Value::Int(int(it, a, 1)?.trailing_zeros() as i32)));
    reg!(it, "java.lang.Integer.highestOneBit", |it, a| {
        let i = int(it, a, 1)? as u32;
        Ok(Value::Int(if i == 0 { 0 } else { (1u32 << (31 - i.leading_zeros())) as i32 }))
    });
    reg!(it, "java.lang.Integer.lowestOneBit", |it, a| {
        let i = int(it, a, 1)?;
        Ok(Value::Int(i & i.wrapping_neg()))
    });
    reg!(it, "java.lang.Integer.rotateLeft", |it, a| Ok(Value::Int(int(it, a, 1)?.rotate_left(int(it, a, 2)? as u32 & 31))));
    reg!(it, "java.lang.Integer.rotateRight", |it, a| Ok(Value::Int(int(it, a, 1)?.rotate_right(int(it, a, 2)? as u32 & 31))));
    reg!(it, "java.lang.Integer.reverse", |it, a| Ok(Value::Int(int(it, a, 1)?.reverse_bits())));
    reg!(it, "java.lang.Integer.reverseBytes", |it, a| Ok(Value::Int(int(it, a, 1)?.swap_bytes())));
    reg!(it, "java.lang.Integer.signum", |it, a| Ok(Value::Int(int(it, a, 1)?.signum())));
    reg!(it, "java.lang.Integer.max", |it, a| Ok(Value::Int(int(it, a, 1)?.max(int(it, a, 2)?))));
    reg!(it, "java.lang.Integer.min", |it, a| Ok(Value::Int(int(it, a, 1)?.min(int(it, a, 2)?))));
    reg!(it, "java.lang.Integer.sum", |it, a| Ok(Value::Int(int(it, a, 1)?.wrapping_add(int(it, a, 2)?))));
    reg!(it, "java.lang.Integer.toUnsignedLong", |it, a| Ok(Value::Long(int(it, a, 1)? as u32 as i64)));
    reg!(it, "java.lang.Integer.toUnsignedString", |it, a| Ok(Value::string((int(it, a, 1)? as u32).to_string())));
    reg!(it, "java.lang.Long.MAX_VALUE", |_it, _a| Ok(Value::Long(i64::MAX)));
    reg!(it, "java.lang.Long.MIN_VALUE", |_it, _a| Ok(Value::Long(i64::MIN)));
    reg!(it, "java.lang.Long.SIZE", |_it, _a| Ok(Value::Int(64)));
    reg!(it, "java.lang.Long.valueOf", |it, a| match arg(a, 1) {
        Value::Str(s) => match long_or_null(java_trim(&s)) {
            Value::Null => number_format(it, &s),
            v => Ok(v),
        },
        v => Ok(Value::Long(v.as_i64().unwrap_or(0))),
    });
    reg!(it, "java.lang.Long.parseLong", |it, a| {
        let s = text(it, a, 1)?;
        match long_or_null(java_trim(&s)) {
            Value::Null => number_format(it, &s),
            v => Ok(v),
        }
    });
    reg!(it, "java.lang.Long.toString", |it, a| {
        let l = long(it, a, 1)?;
        let radix = if a.len() > 2 { int(it, a, 2)? } else { 10 };
        Ok(Value::string(to_radix(l, radix.clamp(2, 36) as u32)))
    });
    reg!(it, "java.lang.Long.toHexString", |it, a| Ok(Value::string(unsigned_radix(long(it, a, 1)? as u64, 16))));
    reg!(it, "java.lang.Long.toBinaryString", |it, a| Ok(Value::string(unsigned_radix(long(it, a, 1)? as u64, 2))));
    reg!(it, "java.lang.Long.hashCode", |it, a| Ok(Value::Int(long_hash_code(long(it, a, 1)?))));
    reg!(it, "java.lang.Long.bitCount", |it, a| Ok(Value::Int(long(it, a, 1)?.count_ones() as i32)));
    reg!(it, "java.lang.Long.numberOfLeadingZeros", |it, a| Ok(Value::Int(long(it, a, 1)?.leading_zeros() as i32)));
    reg!(it, "java.lang.Long.numberOfTrailingZeros", |it, a| Ok(Value::Int(long(it, a, 1)?.trailing_zeros() as i32)));
    reg!(it, "java.lang.Long.rotateLeft", |it, a| Ok(Value::Long(long(it, a, 1)?.rotate_left(int(it, a, 2)? as u32 & 63))));
    reg!(it, "java.lang.Long.signum", |it, a| Ok(Value::Int(long(it, a, 1)?.signum() as i32)));
    reg!(it, "java.lang.Long.max", |it, a| Ok(Value::Long(long(it, a, 1)?.max(long(it, a, 2)?))));
    reg!(it, "java.lang.Long.min", |it, a| Ok(Value::Long(long(it, a, 1)?.min(long(it, a, 2)?))));
    reg!(it, "java.lang.Double.MAX_VALUE", |_it, _a| Ok(Value::Double(f64::MAX)));
    reg!(it, "java.lang.Double.MIN_VALUE", |_it, _a| Ok(Value::Double(f64::from_bits(1))));
    reg!(it, "java.lang.Double.POSITIVE_INFINITY", |_it, _a| Ok(Value::Double(f64::INFINITY)));
    reg!(it, "java.lang.Double.NEGATIVE_INFINITY", |_it, _a| Ok(Value::Double(f64::NEG_INFINITY)));
    reg!(it, "java.lang.Double.NaN", |_it, _a| Ok(Value::Double(f64::NAN)));
    reg!(it, "java.lang.Double.valueOf", |it, a| match arg(a, 1) {
        Value::Str(s) => match parse_java_double(&s) {
            Some(d) => Ok(Value::Double(d)),
            None => number_format(it, &s),
        },
        v => Ok(Value::Double(v.as_f64().unwrap_or(0.0))),
    });
    reg!(it, "java.lang.Double.parseDouble", |it, a| {
        let s = text(it, a, 1)?;
        match parse_java_double(&s) {
            Some(d) => Ok(Value::Double(d)),
            None => number_format(it, &s),
        }
    });
    reg!(it, "java.lang.Double.toString", |it, a| Ok(Value::string(java_double(dbl(it, a, 1)?))));
    reg!(it, "java.lang.Double.hashCode", |it, a| Ok(Value::Int(double_hash_code(dbl(it, a, 1)?))));
    reg!(it, "java.lang.Double.isNaN", |it, a| Ok(Value::Bool(static_or_instance_dbl(it, a)?.is_nan())));
    reg!(it, "java.lang.Double.isInfinite", |it, a| Ok(Value::Bool(static_or_instance_dbl(it, a)?.is_infinite())));
    reg!(it, "java.lang.Double.isFinite", |it, a| Ok(Value::Bool(dbl(it, a, 1)?.is_finite())));
    reg!(it, "java.lang.Double.doubleToLongBits", |it, a| {
        let d = dbl(it, a, 1)?;
        Ok(Value::Long(if d.is_nan() { 0x7ff8000000000000 } else { d.to_bits() as i64 }))
    });
    reg!(it, "java.lang.Double.doubleToRawLongBits", |it, a| Ok(Value::Long(dbl(it, a, 1)?.to_bits() as i64)));
    reg!(it, "java.lang.Double.longBitsToDouble", |it, a| Ok(Value::Double(f64::from_bits(long(it, a, 1)? as u64))));
    reg!(it, "java.lang.Float.valueOf", |it, a| Ok(Value::Float(dbl(it, a, 1)? as f32)));
    reg!(it, "java.lang.Float.isNaN", |it, a| Ok(Value::Bool(static_or_instance_dbl(it, a)?.is_nan())));
    reg!(it, "java.lang.Float.isInfinite", |it, a| Ok(Value::Bool(static_or_instance_dbl(it, a)?.is_infinite())));
    reg!(it, "java.lang.Float.hashCode", |it, a| Ok(Value::Int(float_hash_code(dbl(it, a, 1)? as f32))));
    reg!(it, "java.lang.Float.floatToIntBits", |it, a| {
        let f = dbl(it, a, 1)? as f32;
        Ok(Value::Int(if f.is_nan() { 0x7fc00000 } else { f.to_bits() as i32 }))
    });
    reg!(it, "java.lang.Float.intBitsToFloat", |it, a| Ok(Value::Float(f32::from_bits(int(it, a, 1)? as u32))));
    reg!(it, "java.lang.Float.toString", |it, a| Ok(Value::string(java_float(dbl(it, a, 1)? as f32))));
    reg!(it, "java.lang.Boolean.booleanValue", |it, a| Ok(Value::Bool(boolean(it, a, 0)?)));
    reg!(it, "java.lang.Boolean.compareTo", |it, a| Ok(Value::Int(boolean(it, a, 0)? as i32 - boolean(it, a, 1)? as i32)));
    reg!(it, "java.lang.Boolean.compare", |it, a| Ok(Value::Int(boolean(it, a, 1)? as i32 - boolean(it, a, 2)? as i32)));
    reg!(it, "java.lang.Boolean.hashCode", |it, a| Ok(Value::Int(if boolean(it, a, 1)? { 1231 } else { 1237 })));
    reg!(it, "java.lang.Boolean.toString", |it, a| Ok(Value::string(boolean(it, a, 1)?.to_string())));
    reg!(it, "java.lang.Boolean.parseBoolean", |_it, a| {
        let s = arg(a, 1);
        Ok(Value::Bool(matches!(&s, Value::Str(s) if s.eq_ignore_ascii_case("true"))))
    });
    reg!(it, "java.lang.Boolean.valueOf", |_it, a| Ok(arg(a, 1)));
    reg!(it, "java.lang.Short.valueOf", |_it, a| Ok(arg(a, 1)));
    reg!(it, "java.lang.Short.compareTo", |it, a| Ok(Value::Int(int(it, a, 0)? - int(it, a, 1)?)));
    reg!(it, "java.lang.Byte.compareTo", |it, a| Ok(Value::Int(int(it, a, 0)? - int(it, a, 1)?)));
    reg!(it, "java.lang.Comparable.compareTo", |it, a| match (arg(a, 0), arg(a, 1)) {
        (Value::Short(x), Value::Short(y)) => Ok(Value::Int(x as i32 - y as i32)),
        (Value::Byte(x), Value::Byte(y)) => Ok(Value::Int(x as i32 - y as i32)),
        (x, y) => Ok(Value::Int(compare_natural(it, &x, &y)?)),
    });
    reg!(it, "java.lang.Byte.valueOf", |_it, a| Ok(arg(a, 1)));
    reg!(it, "java.lang.Character.charValue", |it, a| Ok(Value::Char(unit(it, a, 0)?)));
    reg!(it, "java.lang.Character.compareTo", |it, a| Ok(Value::Int(unit(it, a, 0)? as i32 - unit(it, a, 1)? as i32)));
    reg!(it, "java.lang.Character.MAX_VALUE", |_it, _a| Ok(Value::Char(0xffff)));
    reg!(it, "java.lang.Character.MIN_VALUE", |_it, _a| Ok(Value::Char(0)));
    reg!(it, "java.lang.Character.MIN_RADIX", |_it, _a| Ok(Value::Int(2)));
    reg!(it, "java.lang.Character.MAX_RADIX", |_it, _a| Ok(Value::Int(36)));
    reg!(it, "java.lang.Character.hashCode", |it, a| Ok(Value::Int(unit(it, a, 1)? as i32)));
    reg!(it, "java.lang.Character.compare", |it, a| Ok(Value::Int(unit(it, a, 1)? as i32 - unit(it, a, 2)? as i32)));
    reg!(it, "java.lang.Character.isDigit", |it, a| Ok(Value::Bool(decimal(unit(it, a, 1)?) >= 0)));
    reg!(it, "java.lang.Character.isLetter", |it, a| Ok(Value::Bool(is_letter(unit(it, a, 1)?))));
    reg!(it, "java.lang.Character.isLetterOrDigit", |it, a| {
        let u = unit(it, a, 1)?;
        Ok(Value::Bool(is_letter(u) || decimal(u) >= 0))
    });
    reg!(it, "java.lang.Character.isJavaIdentifierPart", |it, a| Ok(Value::Bool(is_java_identifier_part(chr(it, a, 1)?))));
    reg!(it, "java.lang.Character.isUnicodeIdentifierStart", |it, a| Ok(Value::Bool(chr(it, a, 1)?.is_alphabetic())));
    reg!(it, "java.lang.Character.isUnicodeIdentifierPart", |it, a| {
        let c = chr(it, a, 1)?;
        Ok(Value::Bool(c != '$' && !matches!(c as u32, 0xA2..=0xA5 | 0x20A0..=0x20CF) && is_java_identifier_part(c)))
    });
    reg!(it, "java.lang.Character.isWhitespace", |it, a| Ok(Value::Bool(is_java_whitespace(chr(it, a, 1)?))));
    reg!(it, "java.lang.Character.isUpperCase", |it, a| Ok(Value::Bool(chr(it, a, 1)?.is_uppercase())));
    reg!(it, "java.lang.Character.isLowerCase", |it, a| Ok(Value::Bool(chr(it, a, 1)?.is_lowercase())));
    reg!(it, "java.lang.Character.toUpperCase", |_it, a| Ok(map_char(&arg(a, 1), |c| c.to_uppercase().collect())));
    reg!(it, "java.lang.Character.toLowerCase", |_it, a| Ok(map_char(&arg(a, 1), |c| c.to_lowercase().collect())));
    reg!(it, "java.lang.Character.digit", |it, a| Ok(Value::Int(digit(unit(it, a, 1)?, int(it, a, 2)?))));
    reg!(it, "java.lang.Character.toString", |it, a| Ok(str_of_char(chr(it, a, 1)?)));
    reg!(it, "java.lang.Character.getNumericValue", |it, a| Ok(Value::Int(numeric_value(unit(it, a, 1)?))));
    reg!(it, "java.lang.Character.isSurrogate", |it, a| Ok(Value::Bool((unit(it, a, 1)? & 0xf800) == 0xd800)));
    reg!(it, "java.lang.Character.isHighSurrogate", |it, a| Ok(Value::Bool((unit(it, a, 1)? & 0xfc00) == 0xd800)));
    reg!(it, "java.lang.Character.isLowSurrogate", |it, a| Ok(Value::Bool((unit(it, a, 1)? & 0xfc00) == 0xdc00)));
    reg!(it, "java.lang.Character.valueOf", |_it, a| Ok(arg(a, 1)));
    reg!(it, "java.lang.System.out", |_it, _a| Ok(Value::str("out")));
    reg!(it, "java.lang.System.err", |_it, _a| Ok(Value::str("err")));
    reg!(it, "java.lang.System.arraycopy", |it, a| {
        let (src, dest) = (array(it, a, 1)?, array(it, a, 3)?);
        let (src_pos, dest_pos, length) = (int(it, a, 2)?, int(it, a, 4)?, int(it, a, 5)?);
        let (n, m) = (src.borrow().len() as i32, dest.borrow().len() as i32);
        if src_pos < 0 || dest_pos < 0 || length < 0 || src_pos + length > n || dest_pos + length > m {
            return it.throw_named("ArrayIndexOutOfBoundsException", &format!("arraycopy: last source index {} out of bounds for length {}", src_pos + length, n));
        }
        let items: Vec<Value> = src.borrow()[src_pos as usize..(src_pos + length) as usize].to_vec();
        let mut d = dest.write();
        for (i, v) in items.into_iter().enumerate() {
            d[dest_pos as usize + i] = v;
        }
        Ok(Value::Unit)
    });
    reg!(it, "java.lang.System.identityHashCode", |it, a| Ok(Value::Int(it.identity_hash_of(&arg(a, 1)))));
    // A macro may read the clock, as under scalac (chimney times its derivations); a folded
    // constant may not.
    reg!(it, "java.lang.System.currentTimeMillis", |it, _a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the clock");
        }
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        Ok(Value::Long(ms))
    });
    reg!(it, "java.lang.System.nanoTime", |it, _a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the clock");
        }
        thread_local! { static START: Cell<Option<std::time::Instant>> = const { Cell::new(None) }; }
        let start = START.with(|s| {
            if s.get().is_none() {
                s.set(Some(std::time::Instant::now()));
            }
            s.get().unwrap()
        });
        Ok(Value::Long(start.elapsed().as_nanos() as i64))
    });
    // The host's, as the JVM's: `\r\n` on Windows.
    reg!(it, "java.lang.System.lineSeparator", |_it, _a| Ok(Value::str(if cfg!(windows) { "\r\n" } else { "\n" })));
    // A resource bundle read from the class path's `.properties` file, for the messages a test
    // framework's macro formats at compile time; its pairs are the object's fields, which
    // `getString` reads (`class::jdk_member`).
    reg!(it, "java.util.ResourceBundle.getBundle", |it, a| {
        let name = text(it, a, 1)?;
        let path = format!("{}.properties", name.replace('.', "/"));
        let Some(bytes) = it.typer.loaded.as_ref().and_then(|l| l.cp.resource(&path)) else {
            return it.unsupported(format!("no resource bundle {} on the class path", name));
        };
        let Some(c) = it.typer.class_at(&["java", "util", "ResourceBundle"]) else {
            return it.unsupported("java.util.ResourceBundle is not on the class path");
        };
        let mut fields = Vec::new();
        for (key, value) in parse_properties(&String::from_utf8_lossy(&bytes)) {
            fields.push(Value::string(key));
            fields.push(Value::string(value));
        }
        let obj = Rc::new(Object { class: c, fields: std::cell::RefCell::new(fields), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
        Ok(Value::Obj(obj))
    });
    // The program's end with its status, as the JVM halts: nothing after the call runs, a
    // `finally` included; `teq interp` exits with it once the output is flushed. A compile-time
    // run cannot end the compiler.
    reg!(it, "java.lang.System.exit", |it, a| {
        if it.pure {
            return it.impure("System.exit");
        }
        let status = int(it, a, 1)?;
        Err(Control::Fail(Failure::Exit(status)))
    });
    // A macro runs where scalac runs it, on the JVM of the build, and a program under `teq interp`
    // in its process: the properties that follow from the process (its directory, the user's
    // home, the separators, the temporary directory) as that JVM has them; any other reads as
    // unset. A folded constant reads none: the call stays the program's, which reads them when it
    // runs.
    reg!(it, "java.lang.System.getProperty", |it, a| {
        let fallback = a.get(2).cloned().unwrap_or(Value::Null);
        let Some(Value::Str(key)) = a.get(1) else { return it.throw_named("NullPointerException", "key") };
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("a system property");
        }
        let value = match &**key {
            "user.dir" => std::env::current_dir().ok().map(|d| d.to_string_lossy().into_owned()),
            "user.home" => if cfg!(windows) { std::env::var("USERPROFILE").ok() } else { std::env::var("HOME").ok() },
            // The JDK's: `/tmp` on Linux whatever TMPDIR says, the system's elsewhere.
            "java.io.tmpdir" => Some(if cfg!(target_os = "linux") { "/tmp".to_string() } else { std::env::temp_dir().to_string_lossy().into_owned() }),
            "os.name" => Some(super::process::os_name()),
            "os.arch" => Some(match std::env::consts::ARCH {
                "x86_64" if !cfg!(target_os = "macos") => "amd64",
                "x86" => "x86",
                arch => arch,
            }
            .to_string()),
            "file.separator" => Some(std::path::MAIN_SEPARATOR.to_string()),
            "path.separator" => Some(if cfg!(windows) { ";" } else { ":" }.to_string()),
            "line.separator" => Some(if cfg!(windows) { "\r\n" } else { "\n" }.to_string()),
            _ => None,
        };
        Ok(value.map_or(fallback, |v| Value::Str(v.into())))
    });
    // A macro runs where scalac runs it, on the JVM of the build, and a program under `teq interp`
    // in its process: the variable or null. A folded constant reads none: the call stays the
    // program's.
    reg!(it, "java.lang.System.getenv", |it, a| {
        let key = match a.last() {
            Some(Value::Str(k)) => k.clone(),
            Some(system @ Value::Obj(_)) if a.len() == 1 => return it.call_by_name(system.clone(), "environment", Vec::new()),
            _ => return it.throw_named("NullPointerException", "key"),
        };
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the environment");
        }
        Ok(std::env::var(&*key).map_or(Value::Null, |v| Value::Str(v.into())))
    });
    reg!(it, "java.lang.System.envEntries", |it, _a| {
        if it.pure && it.macro_ctx.is_none() {
            return it.impure("the environment");
        }
        let mut vars: Vec<(String, String)> = std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
        vars.sort();
        Ok(Value::array(vars.into_iter().flat_map(|(k, v)| [Value::Str(k.into()), Value::Str(v.into())]).collect()))
    });
    reg!(it, "java.io.PrintStream.println", |it, a| {
        let stream = text(it, a, 0)?;
        let mut s = if a.len() > 1 { it.to_str(&arg(a, 1))? } else { String::new() };
        s.push('\n');
        it.print_to(&stream, &s)?;
        Ok(Value::Unit)
    });
    reg!(it, "java.io.PrintStream.print", |it, a| {
        let stream = text(it, a, 0)?;
        let s = it.to_str(&arg(a, 1))?;
        it.print_to(&stream, &s)?;
        Ok(Value::Unit)
    });
    reg!(it, "java.io.PrintStream.flush", |_it, _a| Ok(Value::Unit));
    reg!(it, "java.util.Objects.equals", |it, a| Ok(Value::Bool(it.equal(&arg(a, 1), &arg(a, 2))?)));
    reg!(it, "java.util.Objects.hashCode", |it, a| match arg(a, 1) {
        Value::Null => Ok(Value::Int(0)),
        v => Ok(Value::Int(it.hash_code(&v)?)),
    });
    reg!(it, "java.util.Objects.requireNonNull", |it, a| match arg(a, 1) {
        Value::Null => {
            let msg = if a.len() > 2 { text(it, a, 2)?.to_string() } else { String::new() };
            it.throw_named("NullPointerException", &msg)
        }
        v => Ok(v),
    });
    reg!(it, "java.util.Objects.toString", |it, a| Ok(Value::string(it.to_str(&arg(a, 1))?)));
    reg!(it, "java.util.Objects.isNull", |_it, a| Ok(Value::Bool(matches!(arg(a, 1), Value::Null))));
    reg!(it, "java.util.Objects.nonNull", |_it, a| Ok(Value::Bool(!matches!(arg(a, 1), Value::Null))));
    reg!(it, "java.util.Arrays.copyOf", |it, a| {
        let arr = array(it, a, 1)?;
        let n = int(it, a, 2)?.max(0) as usize;
        let items = arr.borrow();
        // The reference overload: the primitive ones are the std's, padded with their zero.
        let fill = Value::Null;
        let mut out: Vec<Value> = items.iter().take(n).cloned().collect();
        while out.len() < n {
            out.push(fill.clone());
        }
        Ok(Value::array(out))
    });
    reg!(it, "java.util.Arrays.copyOfRange", |it, a| {
        let arr = array(it, a, 1)?;
        let (from, to) = (int(it, a, 2)?, int(it, a, 3)?);
        if from > to {
            return it.throw_named("IllegalArgumentException", &format!("{} > {}", from, to));
        }
        let items = arr.borrow();
        if from < 0 || from as usize > items.len() {
            let msg = format!("Array index out of range: {}", from);
            drop(items);
            return it.throw_named("ArrayIndexOutOfBoundsException", &msg);
        }
        // The reference overload: the primitive ones are the std's, padded with their zero.
        let fill = Value::Null;
        let mut out: Vec<Value> = items.iter().skip(from as usize).take((to - from) as usize).cloned().collect();
        while out.len() < (to - from) as usize {
            out.push(fill.clone());
        }
        Ok(Value::array(out))
    });
    reg!(it, "java.util.Arrays.fill", |it, a| {
        let arr = array(it, a, 1)?;
        let mut items = arr.write();
        let (from, to, value) = if a.len() > 3 { (int(it, a, 2)?.max(0) as usize, int(it, a, 3)?.max(0) as usize, arg(a, 4)) } else { (0, items.len(), arg(a, 2)) };
        for i in from..to.min(items.len()) {
            items[i] = value.clone();
        }
        Ok(Value::Unit)
    });
    // `sort(a)`, `sort(a, comparator)`, `sort(a, from, to)` and `sort(a, from, to, comparator)`.
    reg!(it, "java.util.Arrays.sort", |it, a| {
        let arr = array(it, a, 1)?;
        let ranged = a.len() > 3;
        let comparator = if ranged { arg(a, 4) } else { arg(a, 2) };
        let len = arr.borrow().len();
        let (from, to) = if ranged { (int(it, a, 2)?.max(0) as usize, (int(it, a, 3)?.max(0) as usize).min(len)) } else { (0, len) };
        let mut items: Vec<Value> = arr.borrow()[from..to.max(from)].to_vec();
        match comparator {
            Value::Null => sort_values(it, &mut items, &mut |it, x, y| compare_natural(it, x, y))?,
            cmp => sort_values(it, &mut items, &mut |it, x, y| {
                let r = it.call_by_name(cmp.clone(), "compare", vec![x.clone(), y.clone()])?;
                Ok(r.as_i32().unwrap_or(0))
            })?,
        }
        arr.write()[from..to.max(from)].clone_from_slice(&items);
        Ok(Value::Unit)
    });
    reg!(it, "java.util.Arrays.equals", |it, a| {
        let (x, y) = (arg(a, 1), arg(a, 2));
        match (&x, &y) {
            (Value::Array(p), Value::Array(q)) => {
                if Rc::ptr_eq(p, q) {
                    return Ok(Value::Bool(true));
                }
                let (u, v) = (p.borrow().clone(), q.borrow().clone());
                if u.len() != v.len() {
                    return Ok(Value::Bool(false));
                }
                for (s, t) in u.iter().zip(&v) {
                    if !it.equal(s, t)? {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            _ => Ok(Value::Bool(x.same(&y))),
        }
    });
    reg!(it, "java.util.Arrays.hashCode", |it, a| match arg(a, 1) {
        Value::Array(p) => {
            let items = p.borrow().clone();
            let mut h: i32 = 1;
            for x in &items {
                let e = if let Value::Null = x { 0 } else { it.hash_code(x)? };
                h = h.wrapping_mul(31).wrapping_add(e);
            }
            Ok(Value::Int(h))
        }
        _ => Ok(Value::Int(0)),
    });
    reg!(it, "java.util.Arrays.toString", |it, a| match arg(a, 1) {
        Value::Array(p) => {
            let items = p.borrow().clone();
            let mut parts = Vec::with_capacity(items.len());
            for x in &items {
                parts.push(it.to_str(x)?);
            }
            Ok(Value::string(format!("[{}]", parts.join(", "))))
        }
        _ => Ok(Value::str("null")),
    });
}

fn big(it: &mut Interp, a: A, i: usize) -> R<Big> {
    match a.get(i) {
        Some(Value::Str(s)) => match Big::parse(s, 10) {
            Some(b) => Ok(b),
            None => it.unsupported(format!("argument {} is no big integer: {}", i, s)),
        },
        Some(v) if v.is_integral() => Ok(Big::from_i64(v.as_i64().unwrap())),
        _ => it.unsupported(format!("argument {} is no big integer", i)),
    }
}

fn big_value(b: Big) -> Value {
    Value::string(b.to_string_radix(10))
}

/// `std/javalib/math.scala`'s JavaScript `BigInt` operations.
fn install_bignum(it: &mut Table) {
    reg!(it, "java.math.bigOf", |it, a| {
        let s = text(it, a, 0)?;
        match Big::parse(&s, 10) {
            Some(b) => Ok(big_value(b)),
            None => it.unsupported(format!("no big integer: {}", s)),
        }
    });
    reg!(it, "java.math.bigOfLong", |it, a| Ok(big_value(Big::from_i64(long(it, a, 0)?))));
    reg!(it, "java.math.bigToLong", |it, a| Ok(Value::Long(big(it, a, 0)?.to_i64_wrapping())));
    reg!(it, "java.math.bigToDouble", |it, a| Ok(Value::Double(big(it, a, 0)?.to_f64())));
    reg!(it, "java.math.bigAdd", |it, a| Ok(big_value(big(it, a, 0)?.add(&big(it, a, 1)?))));
    reg!(it, "java.math.bigSub", |it, a| Ok(big_value(big(it, a, 0)?.sub(&big(it, a, 1)?))));
    reg!(it, "java.math.bigMul", |it, a| Ok(big_value(big(it, a, 0)?.mul(&big(it, a, 1)?))));
    reg!(it, "java.math.bigQuot", |it, a| match big(it, a, 0)?.divrem(&big(it, a, 1)?) {
        Some((q, _)) => Ok(big_value(q)),
        None => it.unsupported("BigInt division by zero".to_string()),
    });
    reg!(it, "java.math.bigRem", |it, a| match big(it, a, 0)?.divrem(&big(it, a, 1)?) {
        Some((_, r)) => Ok(big_value(r)),
        None => it.unsupported("BigInt division by zero".to_string()),
    });
    reg!(it, "java.math.bigNeg", |it, a| Ok(big_value(big(it, a, 0)?.neg())));
    reg!(it, "java.math.bigPow", |it, a| Ok(big_value(big(it, a, 0)?.pow(int(it, a, 1)?.max(0) as u32))));
    reg!(it, "java.math.bigShl", |it, a| Ok(big_value(big(it, a, 0)?.shl(int(it, a, 1)?))));
    reg!(it, "java.math.bigShr", |it, a| Ok(big_value(big(it, a, 0)?.shr(int(it, a, 1)?))));
    reg!(it, "java.math.bigAnd", |it, a| Ok(big_value(big(it, a, 0)?.bitwise(&big(it, a, 1)?, |x, y| x & y))));
    reg!(it, "java.math.bigOr", |it, a| Ok(big_value(big(it, a, 0)?.bitwise(&big(it, a, 1)?, |x, y| x | y))));
    reg!(it, "java.math.bigXor", |it, a| Ok(big_value(big(it, a, 0)?.bitwise(&big(it, a, 1)?, |x, y| x ^ y))));
    reg!(it, "java.math.bigNot", |it, a| Ok(big_value(big(it, a, 0)?.not())));
    reg!(it, "java.math.bigCompare", |it, a| Ok(Value::Int(big(it, a, 0)?.cmp(&big(it, a, 1)?) as i32)));
    reg!(it, "java.math.bigSame", |it, a| Ok(Value::Bool(big(it, a, 0)? == big(it, a, 1)?)));
    reg!(it, "java.math.bigText", |it, a| Ok(Value::string(big(it, a, 0)?.to_string_radix(int(it, a, 1)?.clamp(2, 36) as u32))));
    reg!(it, "java.math.bigHash", |it, a| Ok(Value::Int(big(it, a, 0)?.jdk_hash())));
    reg!(it, "java.math.bigLowestSetBit", |it, a| Ok(Value::Int(big(it, a, 0)?.lowest_set_bit())));
    reg!(it, "java.math.doubleOfText", |it, a| {
        let s = text(it, a, 0)?;
        Ok(Value::Double(s.parse().unwrap_or(f64::NAN)))
    });
}

fn install_regex(it: &mut Table) {
    reg!(it, "scala.util.matching.Regex.quoteReplacement", |it, a| {
        if matches!(arg(a, 1), Value::Null) {
            return it.throw_named("NullPointerException", "");
        }
        let s = text(it, a, 1)?;
        let mut out = String::new();
        for c in s.chars() {
            if c == '\\' || c == '$' {
                out.push('\\');
            }
            out.push(c);
        }
        Ok(Value::string(out))
    });
    reg!(it, "scala.util.matching.Regex.exec", |it, a| {
        let (re, mode, source) = (text(it, a, 1)?, text(it, a, 2)?, text(it, a, 3)?);
        Ok(it.regex_exec(&re, &mode, &source)?.map_or(Value::Null, Value::Match))
    });
    reg!(it, "scala.util.matching.Regex.all", |it, a| {
        let (re, source) = (text(it, a, 1)?, text(it, a, 2)?);
        let all = it.regex_all(&re, &source)?;
        Ok(Value::array(all.into_iter().map(Value::Match).collect()))
    });
    reg!(it, "scala.util.matching.Regex.replace", |it, a| {
        let (re, target) = (text(it, a, 1)?, text(it, a, 2)?);
        let all = boolean(it, a, 4)?;
        let wrap = arg(a, 5);
        Ok(Value::string(it.regex_replace(&re, &target, &arg(a, 3), all, Some(wrap))?))
    });
    reg!(it, "scala.util.matching.Regex.splitBy", |it, a| {
        let (re, source) = (text(it, a, 1)?, text(it, a, 2)?);
        let limit = int(it, a, 3)?;
        let parts = it.regex_split(&re, &source, limit)?;
        Ok(Value::array(parts.into_iter().map(Value::string).collect()))
    });
    reg!(it, "scala.util.matching.Regex.groupOf", |it, a| match (arg(a, 1), arg(a, 2)) {
        (Value::Match(m), id) if id.is_integral() => Ok(it.match_group(&m, id.as_i32().unwrap() as usize)),
        (Value::Match(m), Value::Str(name)) => match m.names.iter().find(|(n, _)| **n == *name) {
            Some(&(_, i)) => Ok(it.match_group(&m, i)),
            None => it.throw_named("IllegalArgumentException", &format!("No group with name <{}>", name)),
        },
        _ => Ok(Value::Null),
    });
    reg!(it, "scala.util.matching.Regex.groupCountOf", |_it, a| match arg(a, 1) {
        Value::Match(m) => Ok(Value::Int(m.groups.len() as i32 - 1)),
        _ => Ok(Value::Int(0)),
    });
    reg!(it, "scala.util.matching.Regex.startOf", |_it, a| match arg(a, 1) {
        Value::Match(m) => Ok(Value::Int(m.groups[0].map_or(0, |(s, _)| s as i32))),
        _ => Ok(Value::Int(0)),
    });
}

/// Scala.js's reflective instantiation (`std/scalajs/reflect.scala`): the registrations run when a
/// lookup first asks, as a JavaScript module runs them when it loads.
fn install_reflective(it: &mut Table) {
    reg!(it, "$reflectClass($0, $1, $2)", |it, a| it.register_reflective(0, a));
    reg!(it, "$reflectModule($0, $1, $2)", |it, a| it.register_reflective(1, a));
    reg!(it, "java.lang.giveCause", |it, a| {
        let cause_param = it.typer.b.throwable.and_then(|t| it.syms().class(t).ctor_syms.first().and_then(|c| c.get(1).copied()));
        if let Some(s) = cause_param {
            it.set_field(arg(a, 0), s, arg(a, 1))?;
        }
        Ok(Value::Unit)
    });
    reg!(it, "java.lang.causeGiven", |_it, _a| Ok(Value::Bool(false)));
    reg!(it, "scala.scalajs.reflect.withCause", |it, a| {
        let e = arg(a, 0);
        let cause_param = it.typer.b.throwable.and_then(|t| it.syms().class(t).ctor_syms.first().and_then(|c| c.get(1).copied()));
        if let Some(s) = cause_param {
            it.set_field(e.clone(), s, arg(a, 1))?;
        }
        Ok(e)
    });
    reg!(it, "$classNamed(\"java.lang.Void\")", |it, _a| Ok(it.class_value_named("java.lang.Void")));
    // A cast of a reference to a primitive type (`Typer::unboxes`), and a reflective call's
    // argument for a primitive parameter: `??` takes `undefined`, the unit, for a null too.
    reg!(it, crate::typer::prims::UNBOX, |_it, a| Ok(match arg(a, 0) {
        Value::Null | Value::Unit => arg(a, 1),
        v => v,
    }));
    reg!(it, "scala.scalajs.reflect.registeredClass", |it, a| it.reflective_entry(0, a));
    reg!(it, "scala.scalajs.reflect.registeredModule", |it, a| it.reflective_entry(1, a));
    reg!(it, "scala.scalajs.reflect.isUndefined", |_it, a| Ok(Value::Bool(matches!(arg(a, 0), Value::Unit))));
    reg!(it, "scala.scalajs.reflect.wrapper", |_it, a| Ok(entry_slot(&arg(a, 0), 2)));
    reg!(it, "scala.scalajs.reflect.setWrapper", |_it, a| {
        if let Value::Array(e) = arg(a, 0) {
            e.write()[2] = arg(a, 1);
        }
        Ok(Value::Unit)
    });
    reg!(it, "scala.scalajs.reflect.entryClass", |_it, a| Ok(entry_slot(&arg(a, 0), 0)));
    reg!(it, "scala.scalajs.reflect.moduleLoader", |_it, a| Ok(entry_slot(&arg(a, 0), 1)));
    reg!(it, "scala.scalajs.reflect.ctorCount", |_it, a| Ok(match entry_slot(&arg(a, 0), 1) {
        Value::Array(c) => Value::Int(c.borrow().len() as i32),
        _ => Value::Int(0),
    }));
    reg!(it, "scala.scalajs.reflect.ctorParams", |it, a| {
        let i = int(it, a, 1)? as usize;
        match entry_slot(&entry_slot(&entry_slot(&arg(a, 0), 1), i), 0) {
            Value::Fun(f) => it.call_closure(&f, Vec::new()),
            _ => Ok(Value::Unit),
        }
    });
    reg!(it, "scala.scalajs.reflect.ctorFun", |it, a| {
        let i = int(it, a, 1)? as usize;
        Ok(entry_slot(&entry_slot(&entry_slot(&arg(a, 0), 1), i), 1))
    });
}

const PRIMITIVE_CLASS_NAMES: [&str; 9] = ["int", "long", "double", "float", "short", "byte", "char", "boolean", "void"];

fn entry_slot(v: &Value, i: usize) -> Value {
    match v {
        Value::Array(items) => items.borrow().get(i).cloned().unwrap_or(Value::Unit),
        _ => Value::Unit,
    }
}

impl Interp<'_, '_> {
    fn register_reflective(&mut self, kind: usize, a: A) -> R {
        let name = text(self, a, 0)?;
        let entry = Value::array(vec![arg(a, 1), arg(a, 2), Value::Unit]);
        if let Some(maps) = self.reflective.as_mut() {
            maps[kind].insert(name, entry);
        }
        Ok(Value::Unit)
    }

    fn reflective_entry(&mut self, kind: usize, a: A) -> R {
        if self.reflective.is_none() {
            self.reflective = Some(Box::new([FxMap::default(), FxMap::default()]));
            for (_, e) in self.typer.reflective_registrations() {
                self.eval(e, &Frame::new(None), &Ctx::plain())?;
            }
        }
        let Value::Str(name) = arg(a, 0) else { return Ok(Value::Unit) };
        Ok(self.reflective.as_ref().unwrap()[kind].get(&name).cloned().unwrap_or(Value::Unit))
    }
}

/// The engine under the std's `java.util.regex` (`std/javalib/util.scala`): the lean
/// `Regex`'s primitives, and the positions of a match's groups.
fn install_regex_engine(it: &mut Table) {
    for name in ["exec", "all", "replace"] {
        let b = it[&format!("scala.util.matching.Regex.{}", name)].clone();
        it.insert(format!("java.util.regex.Engine.{}", name), b);
    }
    reg!(it, "java.util.regex.Engine.group", |it, a| match arg(a, 1) {
        Value::Match(m) => {
            let i = int(it, a, 2)?.max(0) as usize;
            Ok(it.match_group(&m, i))
        }
        _ => Ok(Value::Null),
    });
    reg!(it, "java.util.regex.Engine.hasGroup", |_it, a| match (arg(a, 1), arg(a, 2)) {
        (Value::Match(m), Value::Str(name)) => Ok(Value::Bool(m.names.iter().any(|(n, _)| **n == *name))),
        _ => Ok(Value::Bool(false)),
    });
    reg!(it, "java.util.regex.Engine.named", |it, a| match (arg(a, 1), arg(a, 2)) {
        (Value::Match(m), Value::Str(name)) => match m.names.iter().find(|(n, _)| **n == *name) {
            Some(&(_, i)) => Ok(it.match_group(&m, i)),
            None => Ok(Value::Null),
        },
        _ => Ok(Value::Null),
    });
    // Whether the pattern compiles: null, or the parser's message (`Regex::compile`).
    reg!(it, "java.util.regex.Engine.check", |it, a| {
        let src = it.to_str(&arg(a, 1))?;
        Ok(match regex::Regex::compile(&src) {
            Ok(_) => Value::Null,
            Err(msg) => Value::string(msg),
        })
    });
    let quote = it["scala.util.matching.Regex.quoteReplacement"].clone();
    it.insert("java.util.regex.Engine.quote".to_string(), quote);
    reg!(it, "java.util.regex.Engine.groupCount", |_it, a| match arg(a, 1) {
        Value::Match(m) => Ok(Value::Int(m.groups.len() as i32 - 1)),
        _ => Ok(Value::Int(0)),
    });
    for (name, end) in [("start", false), ("end", true)] {
        it.insert(format!("java.util.regex.Engine.{}", name), Rc::new(move |it: &mut Interp<'_, '_>, a: &[Value]| {
            let i = int(it, a, 2)?;
            Ok(Value::Int(match arg(a, 1) {
                Value::Match(m) => match m.groups.get(i.max(0) as usize).copied().flatten() {
                    Some((s, e)) => (if end { e } else { s }) as i32,
                    None => -1,
                },
                _ => -1,
            }))
        }));
    }
}

fn install_typer_templates(it: &mut Table) {
    reg!(it, crate::typer::WITHHELD_TEMPLATE, |it, _a| it.withheld_reached());
    reg!(it, "$hashCode($0)", |it, a| Ok(Value::Int(it.hash_code(&arg(a, 0))?)));
    reg!(it, "$identityHash($0)", |it, a| Ok(Value::Int(it.identity_hash_of(&arg(a, 0)))));
    reg!(it, "$anyStr($0)", |it, a| Ok(Value::string(it.any_to_string(&arg(a, 0)))));
    reg!(it, "$equals($0, $1)", |it, a| Ok(Value::Bool(it.equals_method(&arg(a, 0), &arg(a, 1))?)));
    reg!(it, "$doubleEquals($0, $1)", |_it, a| {
        let (x, y) = (arg(a, 0), arg(a, 1));
        Ok(Value::Bool(match (x.as_f64(), y.as_f64()) {
            (Some(p), Some(q)) => p.to_bits() == q.to_bits() || (p.is_nan() && q.is_nan()),
            _ => x.same(&y),
        }))
    });
    reg!(it, "$getClass($0)", |it, a| Ok(it.class_of_value(&arg(a, 0))));
    reg!(it, "$productPrefix($0)", |it, a| Ok(Value::string(it.product_prefix(&arg(a, 0))?)));
    reg!(it, "$productArity($0)", |it, a| Ok(Value::Int(it.product_arity(&arg(a, 0))?)));
    reg!(it, "$productElement($0, $1)", |it, a| {
        let i = int(it, a, 1)?;
        it.product_element(&arg(a, 0), i)
    });
    reg!(it, "scala.reflect.memberByName", |it, a| {
        let name = text(it, a, 1)?;
        it.reflective_member(arg(a, 0), &name, Vec::new())
    });
    reg!(it, "scala.reflect.callByName", |it, a| {
        let name = text(it, a, 1)?;
        let args = array(it, a, 2)?.borrow().clone();
        it.reflective_member(arg(a, 0), &name, args)
    });
    reg!(it, "$productElementName($0, $1)", |it, a| {
        let i = int(it, a, 1)?;
        it.product_element_name(&arg(a, 0), i)
    });
    reg!(it, "$0.$ordinal", |_it, a| match arg(a, 0) {
        Value::Obj(o) => Ok(Value::Int(o.ordinal.get())),
        _ => Ok(Value::Int(0)),
    });
    reg!(it, "$enumValueOf($0, $1, $2)", |it, a| {
        let values = array(it, a, 0)?.borrow().clone();
        let name = text(it, a, 1)?;
        let enum_name = text(it, a, 2)?;
        for v in values {
            if let Value::Obj(o) = &v {
                if o.name.as_deref() == Some(&*name) {
                    return Ok(v);
                }
            }
        }
        it.throw_named("IllegalArgumentException", &format!("enum {} has no case with name: {}", enum_name, name))
    });
    reg!(it, "$enumFromOrdinal($0, $1, $2)", |it, a| {
        let values = array(it, a, 0)?.borrow().clone();
        let ordinal = int(it, a, 1)?;
        let enum_name = text(it, a, 2)?;
        match values.get(ordinal.max(0) as usize) {
            Some(v) if ordinal >= 0 => Ok(v.clone()),
            _ => it.throw_named("NoSuchElementException", &format!("enum {} has no case with ordinal: {}", enum_name, ordinal)),
        }
    });
    reg!(it, "$pf($0, $1)", |_it, a| {
        Ok(Value::Fun(Rc::new(Closure {
            kind: ClosureKind::Partial(arg(a, 0), arg(a, 1)),
            env: Frame::new(None),
            this: Value::Unit,
            def_key: 0,
            class: None,
            hash: Cell::new(0),
        })))
    });
    for text in ["($0)[$1]", "(($0)[$1] = $2)", "($0)[$1](...$2)"] {
        reg!(it, text, |it, _a| it.unsupported("js.Dynamic is not executable by the interpreter"));
    }
}

impl<'a, 't> Interp<'a, 't> {
    fn print_to(&mut self, stream: &str, s: &str) -> R<()> {
        if self.pure {
            return self.impure("output");
        }
        if stream == "err" {
            self.flush();
            eprint!("{}", s);
        } else {
            self.print(s);
        }
        Ok(())
    }

    fn compile_regex(&mut self, src: &str) -> R<Rc<regex::Regex>> {
        if let Some(re) = self.regex_cache.get(src) {
            return Ok(re.clone());
        }
        match regex::Regex::compile(src) {
            Ok(re) => {
                let re = Rc::new(re);
                if self.regex_cache.len() > 2000 {
                    self.regex_cache.clear();
                }
                self.regex_cache.insert(src.to_string(), re.clone());
                Ok(re)
            }
            Err(msg) => self.throw_named("IllegalArgumentException", &format!("PatternSyntaxException: {}", msg)),
        }
    }

    fn match_value(re: &regex::Regex, found: &regex::Found, bytes: &Rc<[u32]>, source: &Rc<str>) -> Rc<MatchValue> {
        Rc::new(MatchValue { groups: found.groups.clone(), names: re.names.clone(), source: source.clone(), bytes: bytes.clone() })
    }

    fn char_table(s: &str) -> (Vec<char>, Rc<[u32]>) {
        let (chars, bytes) = regex::unit_chars(s);
        (chars, Rc::from(bytes))
    }

    /// mode: "" first match, "y" match at the start, "f" match of the whole input.
    pub(super) fn regex_exec(&mut self, re: &str, mode: &str, source: &str) -> R<Option<Rc<MatchValue>>> {
        let re = self.compile_regex(re)?;
        let (chars, offsets) = Self::char_table(source);
        let (sticky, whole) = (mode.contains('y') || mode.contains('f'), mode.contains('f'));
        let found = match re.find(&chars, 0, sticky, whole) {
            Ok(f) => f,
            Err(msg) => return self.unsupported(msg),
        };
        let source: Rc<str> = Rc::from(source);
        Ok(found.map(|f| Self::match_value(&re, &f, &offsets, &source)))
    }

    pub(super) fn regex_all(&mut self, re: &str, source: &str) -> R<Vec<Rc<MatchValue>>> {
        let re = self.compile_regex(re)?;
        let (chars, offsets) = Self::char_table(source);
        let all = match re.find_all(&chars) {
            Ok(f) => f,
            Err(msg) => return self.unsupported(msg),
        };
        let source: Rc<str> = Rc::from(source);
        Ok(all.iter().map(|f| Self::match_value(&re, f, &offsets, &source)).collect())
    }

    /// The replacement text of `Matcher.appendReplacement`: `$n`, `${name}` and backslash escapes.
    fn expand_replacement(&mut self, repl: &str, m: &MatchValue) -> R<String> {
        if !repl.contains('$') && !repl.contains('\\') {
            return Ok(repl.to_string());
        }
        let chars: Vec<char> = repl.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' {
                i += 1;
                match chars.get(i) {
                    Some(&e) => out.push(e),
                    None => return self.throw_named("IllegalArgumentException", "character to be escaped is missing"),
                }
            } else if c == '$' {
                i += 1;
                if chars.get(i) == Some(&'{') {
                    let end = chars[i..].iter().position(|&x| x == '}').map(|p| i + p);
                    let Some(end) = end else { return self.throw_named("IllegalArgumentException", "named capturing group is missing trailing '}'") };
                    let name: String = chars[i + 1..end].iter().collect();
                    match m.names.iter().find(|(n, _)| *n == name) {
                        Some(&(_, idx)) => out.push_str(&self.match_group_text(m, idx).unwrap_or_default()),
                        None => return self.throw_named("IllegalArgumentException", &format!("No group with name {{{}}}", name)),
                    }
                    i = end;
                } else {
                    let Some(mut n) = chars.get(i).and_then(|d| d.to_digit(10)).map(|d| d as usize) else {
                        return self.throw_named("IllegalArgumentException", "Illegal group reference");
                    };
                    if n >= m.groups.len() {
                        return self.throw_named("IndexOutOfBoundsException", &format!("No group {}", n));
                    }
                    while let Some(d) = chars.get(i + 1).and_then(|d| d.to_digit(10)) {
                        if n * 10 + d as usize >= m.groups.len() {
                            break;
                        }
                        n = n * 10 + d as usize;
                        i += 1;
                    }
                    out.push_str(&self.match_group_text(m, n).unwrap_or_default());
                }
            } else {
                out.push(c);
            }
            i += 1;
        }
        Ok(out)
    }

    /// `replacement` is a replacement string or a function from the wrapped match to one.
    pub(super) fn regex_replace(&mut self, re: &str, target: &str, replacement: &Value, all: bool, wrap: Option<Value>) -> R<String> {
        let compiled = self.compile_regex(re)?;
        let (chars, offsets) = Self::char_table(target);
        let source: Rc<str> = Rc::from(target);
        let matches = if all {
            match compiled.find_all(&chars) {
                Ok(f) => f,
                Err(msg) => return self.unsupported(msg),
            }
        } else {
            match compiled.find(&chars, 0, false, false) {
                Ok(f) => f.into_iter().collect(),
                Err(msg) => return self.unsupported(msg),
            }
        };
        let mut out = String::new();
        let mut last = 0;
        for found in &matches {
            let m = Self::match_value(&compiled, found, &offsets, &source);
            let (a, b) = m.groups[0].unwrap();
            out.push_str(&m.slice(last, a));
            let text = match replacement {
                Value::Fun(_) | Value::Obj(_) => {
                    let wrapped = match &wrap {
                        Some(w) => self.apply_value(w.clone(), vec![Value::Match(m.clone())])?,
                        None => Value::Match(m.clone()),
                    };
                    let r = self.apply_value(replacement.clone(), vec![wrapped])?;
                    self.to_str(&r)?
                }
                other => self.to_str(other)?,
            };
            out.push_str(&self.expand_replacement(&text, &m)?);
            last = b;
        }
        out.push_str(&unit_slice(target, &offsets, last, offsets.len() - 1));
        Ok(crate::text::canonical(out))
    }

    /// `String.split` with a regular expression: a leading empty string stays and trailing ones
    /// go, unless the limit is negative.
    pub(super) fn regex_split(&mut self, re: &str, source: &str, limit: i32) -> R<Vec<String>> {
        let compiled = self.compile_regex(re)?;
        if source.is_empty() {
            return Ok(vec![String::new()]);
        }
        let (chars, offsets) = Self::char_table(source);
        let all = match compiled.find_all(&chars) {
            Ok(f) => f,
            Err(msg) => return self.unsupported(msg),
        };
        let mut parts: Vec<String> = Vec::new();
        let mut last = 0;
        for found in &all {
            if limit > 0 && parts.len() as i32 == limit - 1 {
                break;
            }
            let (a, b) = found.groups[0].unwrap();
            if b == 0 {
                continue;
            }
            parts.push(unit_slice(source, &offsets, last, a).into_owned());
            last = b;
        }
        if parts.is_empty() {
            return Ok(vec![source.to_string()]);
        }
        parts.push(unit_slice(source, &offsets, last, offsets.len() - 1).into_owned());
        if limit == 0 {
            while parts.last().map_or(false, |p| p.is_empty()) {
                parts.pop();
            }
        }
        Ok(parts)
    }

    /// `split("\\s+")` without the matcher: a run of whitespace (`\s` of `java.util.regex`) ends
    /// a part, a leading run leaves a leading empty part, trailing empty parts go.
    fn split_whitespace_runs(s: &str) -> Vec<String> {
        if s.is_empty() {
            return vec![String::new()];
        }
        let space = |c: char| matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r');
        let mut parts: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut in_run = false;
        for c in s.chars() {
            if space(c) {
                if !in_run {
                    parts.push(std::mem::take(&mut current));
                    in_run = true;
                }
            } else {
                in_run = false;
                current.push(c);
            }
        }
        parts.push(current);
        while parts.len() > 1 && parts.last().map_or(false, |p| p.is_empty()) {
            parts.pop();
        }
        if parts.len() == 1 && parts[0].is_empty() {
            return vec![String::new()];
        }
        parts
    }

    /// `String.split(separator)`: a single character is taken literally, a longer separator is a
    /// regular expression.
    pub(super) fn split_string(&mut self, s: &str, sep: &str, limit: i32) -> R<Vec<String>> {
        let regex_chars = |t: &str| t.chars().any(|c| "\\^$.|?*+()[]{}".contains(c));
        if sep == "\\s+" && limit == 0 {
            return Ok(Self::split_whitespace_runs(s));
        }
        if sep.chars().count() > 1 && regex_chars(sep) || sep.is_empty() {
            return self.regex_split(sep, s, limit);
        }
        if !s.contains(sep) {
            return Ok(vec![s.to_string()]);
        }
        let mut parts: Vec<String> = s.split(sep).map(str::to_string).collect();
        if limit > 0 && parts.len() as i32 > limit {
            let tail = parts.split_off(limit as usize - 1).join(sep);
            parts.push(tail);
        }
        if limit == 0 {
            while parts.last().map_or(false, |p| p.is_empty()) {
                parts.pop();
            }
        }
        Ok(parts)
    }
}

/// `seqHash`: MurmurHash3's `seqHash`, with the range shortcut scala-library takes for an
/// arithmetic sequence of hashes.
pub(super) fn seq_hash(it: &mut Interp, seq: &Value) -> R<i32> {
    let items = it.elements_of(seq)?;
    let seed = str_hash("Seq");
    let (mut h, mut n, mut prev, mut first, mut step, mut state) = (seed, 0i32, 0i32, 0i32, 0i32, 0);
    for x in &items {
        let k = it.hash(x)?;
        h = mix(h, k);
        match state {
            0 => {
                first = k;
                state = 1;
            }
            1 => {
                step = k.wrapping_sub(prev);
                state = 2;
            }
            2 if step != k.wrapping_sub(prev) => state = 3,
            _ => {}
        }
        prev = k;
        n += 1;
    }
    Ok(if state == 2 { avalanche(mix(mix(mix(seed, first), step), prev)) } else { finalize_hash(h, n) })
}
