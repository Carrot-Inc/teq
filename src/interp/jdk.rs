//! Members of JDK classes that a macro calls where the class path offers only the JDK's
//! signatures: `java.time.Instant` and `java.time.Duration` (chimney times its derivations with
//! `Instant.now()` and `Duration.between`). The output calls the JDK's classes; a std class of
//! the name would shadow them for every JVM program. An instance is an object of the JDK class
//! whose two fields are the seconds (`Long`) and the nanoseconds (`Int`); a member of the class
//! that has no entry here stays unsupported.

use super::builtins::{reg, Table};
use super::value::*;
use super::*;
use std::cell::Cell;

type A<'a> = &'a [Value];

fn arg(a: A, i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Null)
}

fn long(it: &mut Interp, a: A, i: usize) -> R<i64> {
    match arg(a, i).as_i64() {
        Some(n) => Ok(n),
        None => it.unsupported("a java.time argument that is no number"),
    }
}

/// The class of the instances of the JDK class whose statics object `a[0]` is, or of which it
/// is an instance.
fn instance_class(it: &Interp, a: A) -> Option<ClassId> {
    let Value::Obj(o) = arg(a, 0) else { return None };
    let info = it.syms().class(o.class);
    Some(if info.kind == crate::symbols::ClassKind::Object { info.companion? } else { o.class })
}

/// `Instant.MIN` and `Instant.MAX` in seconds.
const INSTANT_SECONDS: std::ops::RangeInclusive<i64> = -31_557_014_167_219_200..=31_556_889_864_403_199;

fn make(it: &mut Interp, a: A, secs: i64, nanos: i64) -> R {
    let Some(class) = instance_class(it, a) else { return it.unsupported("a java.time value without its class") };
    let secs = secs.checked_add(nanos.div_euclid(1_000_000_000));
    let Some(secs) = secs else { return overflow(it) };
    let instant = it.typer.interner.intern("Instant");
    if it.syms().class(class).name == instant && !INSTANT_SECONDS.contains(&secs) {
        return it.throw_named("DateTimeException", "Instant exceeds minimum or maximum instant");
    }
    let fields = vec![Value::Long(secs), Value::Int(nanos.rem_euclid(1_000_000_000) as i32)];
    Ok(Value::Obj(Rc::new(Object { class, fields: RefCell::new(fields), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) })))
}

/// The seconds and nanoseconds of an `Instant` or a `Duration`.
fn parts(it: &mut Interp, v: &Value) -> R<(i64, i64)> {
    if let Value::Obj(o) = v {
        let fields = o.fields.borrow();
        if let (Some(Value::Long(s)), Some(Value::Int(n))) = (fields.first(), fields.get(1)) {
            return Ok((*s, *n as i64));
        }
    }
    it.unsupported("a java.time value that no macro made")
}

fn nanos_between(it: &mut Interp, a: &Value, b: &Value) -> R<i128> {
    let (sa, na) = parts(it, a)?;
    let (sb, nb) = parts(it, b)?;
    Ok((sb as i128 - sa as i128) * 1_000_000_000 + (nb - na) as i128)
}

fn overflow<T>(it: &mut Interp) -> R<T> {
    it.throw_named("ArithmeticException", "long overflow")
}

/// As the JDK's: the order of the seconds, then the difference of the nanoseconds.
fn compare(it: &mut Interp, a: A) -> R<i32> {
    let (sx, nx) = parts(it, &arg(a, 0))?;
    let (sy, ny) = parts(it, &arg(a, 1))?;
    Ok(if sx != sy { sx.cmp(&sy) as i32 } else { (nx - ny) as i32 })
}

/// The whole count of `unit` nanoseconds, truncated toward zero, as `Duration.toMillis` and
/// `toNanos` compute it.
fn in_units(it: &mut Interp, a: A, unit: i64) -> R {
    let (s, n) = parts(it, &arg(a, 0))?;
    let (s, n) = if s < 0 { (s + 1, n - 1_000_000_000) } else { (s, n) };
    match s.checked_mul(1_000_000_000 / unit).and_then(|v| v.checked_add(n / unit)) {
        Some(v) => Ok(Value::Long(v)),
        None => overflow(it),
    }
}

fn now() -> (i64, i64) {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    (d.as_secs() as i64, d.subsec_nanos() as i64)
}

/// `1970-01-01T00:00:00Z` with the fraction in groups of three digits, as `Instant.toString`.
fn instant_text(secs: i64, nanos: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    let year = if year > 9999 { format!("+{}", year) } else if year < 0 { format!("-{:04}", -year) } else { format!("{:04}", year) };
    let mut s = format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}", year, month, day, rem / 3600, rem % 3600 / 60, rem % 60);
    if nanos > 0 {
        let digits = if nanos % 1_000_000 == 0 { format!(".{:03}", nanos / 1_000_000) } else if nanos % 1000 == 0 { format!(".{:06}", nanos / 1000) } else { format!(".{:09}", nanos) };
        s.push_str(&digits);
    }
    s.push('Z');
    s
}

/// `PT8H6M12.345S`, as `Duration.toString`.
fn duration_text(secs: i64, nanos: i64) -> String {
    if secs == 0 && nanos == 0 {
        return "PT0S".to_string();
    }
    let effective = if secs < 0 && nanos > 0 { secs + 1 } else { secs };
    let (hours, minutes, s) = (effective / 3600, (effective % 3600) / 60, effective % 60);
    let mut out = String::from("PT");
    if hours != 0 {
        out.push_str(&format!("{}H", hours));
    }
    if minutes != 0 {
        out.push_str(&format!("{}M", minutes));
    }
    if s == 0 && nanos == 0 && out.len() > 2 {
        return out;
    }
    if secs < 0 && nanos > 0 && s == 0 {
        out.push_str("-0");
    } else {
        out.push_str(&s.to_string());
    }
    if nanos > 0 {
        let frac = if secs < 0 { 2 * 1_000_000_000 - nanos } else { nanos + 1_000_000_000 };
        let text = frac.to_string();
        let trimmed = text[1..].trim_end_matches('0');
        out.push('.');
        out.push_str(trimmed);
    }
    out.push('S');
    out
}

pub(super) fn install(it: &mut Table) {
    reg!(it, "java.time.Instant.now", |it, a| {
        let (s, n) = now();
        make(it, a, s, n)
    });
    reg!(it, "java.time.Instant.ofEpochSecond", |it, a| {
        let s = long(it, a, 1)?;
        let n = if a.len() > 2 { long(it, a, 2)? } else { 0 };
        make(it, a, s, n)
    });
    reg!(it, "java.time.Instant.ofEpochMilli", |it, a| {
        let ms = long(it, a, 1)?;
        make(it, a, ms.div_euclid(1000), ms.rem_euclid(1000) * 1_000_000)
    });
    reg!(it, "java.time.Instant.getEpochSecond", |it, a| Ok(Value::Long(parts(it, &arg(a, 0))?.0)));
    reg!(it, "java.time.Instant.getNano", |it, a| Ok(Value::Int(parts(it, &arg(a, 0))?.1 as i32)));
    reg!(it, "java.time.Instant.toEpochMilli", |it, a| {
        let (s, n) = parts(it, &arg(a, 0))?;
        let (s, adjust) = if s < 0 && n > 0 { (s + 1, n / 1_000_000 - 1000) } else { (s, n / 1_000_000) };
        match s.checked_mul(1000).and_then(|v| v.checked_add(adjust)) {
            Some(v) => Ok(Value::Long(v)),
            None => overflow(it),
        }
    });
    reg!(it, "java.time.Instant.isBefore", |it, a| Ok(Value::Bool(compare(it, a)? < 0)));
    reg!(it, "java.time.Instant.isAfter", |it, a| Ok(Value::Bool(compare(it, a)? > 0)));
    reg!(it, "java.time.Instant.compareTo", |it, a| Ok(Value::Int(compare(it, a)?)));
    reg!(it, "java.time.Instant.toString", |it, a| {
        let (s, n) = parts(it, &arg(a, 0))?;
        Ok(Value::string(instant_text(s, n)))
    });
    reg!(it, "java.time.Duration.between", |it, a| {
        let total = nanos_between(it, &arg(a, 1), &arg(a, 2))?;
        make(it, a, total.div_euclid(1_000_000_000) as i64, total.rem_euclid(1_000_000_000) as i64)
    });
    reg!(it, "java.time.Duration.ofSeconds", |it, a| {
        let s = long(it, a, 1)?;
        let n = if a.len() > 2 { long(it, a, 2)? } else { 0 };
        make(it, a, s, n)
    });
    reg!(it, "java.time.Duration.ofMillis", |it, a| {
        let ms = long(it, a, 1)?;
        make(it, a, ms.div_euclid(1000), ms.rem_euclid(1000) * 1_000_000)
    });
    reg!(it, "java.time.Duration.getSeconds", |it, a| Ok(Value::Long(parts(it, &arg(a, 0))?.0)));
    reg!(it, "java.time.Duration.getNano", |it, a| Ok(Value::Int(parts(it, &arg(a, 0))?.1 as i32)));
    reg!(it, "java.time.Duration.toMillis", |it, a| in_units(it, a, 1_000_000));
    reg!(it, "java.time.Duration.toNanos", |it, a| in_units(it, a, 1));
    reg!(it, "java.time.Duration.compareTo", |it, a| Ok(Value::Int(compare(it, a)?)));
    reg!(it, "java.time.Duration.toString", |it, a| {
        let (s, n) = parts(it, &arg(a, 0))?;
        Ok(Value::string(duration_text(s, n)))
    });
    for class in ["Instant", "Duration"] {
        it.insert(format!("java.time.{}.equals", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| {
            let same_class = matches!((arg(a, 0), arg(a, 1)), (Value::Obj(x), Value::Obj(y)) if x.class == y.class);
            Ok(Value::Bool(same_class && parts(it, &arg(a, 0))? == parts(it, &arg(a, 1))?))
        }));
        it.insert(format!("java.time.{}.hashCode", class), Rc::new(|it: &mut Interp<'_, '_>, a: &[Value]| {
            let (s, n) = parts(it, &arg(a, 0))?;
            Ok(Value::Int(((s ^ ((s as u64) >> 32) as i64) as i32).wrapping_add(51 * n as i32)))
        }));
    }
}
