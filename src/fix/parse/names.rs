//! ScalametaParser.scala's "identifiers and literals" section: names, paths and their selectors, qualified ids and
//! literals, each a value of its token (an identifier's without its backquotes, a literal's as scalameta reads it).

use super::{Parser, P};
use crate::fix::nodes::{F, K};
use crate::fix::scanner::C;

/// An identifier's value: its text, a backquoted one's without the backquotes (`Ident.value`).
pub(super) fn ident_value(text: &str) -> String {
    text.strip_prefix('`').and_then(|t| t.strip_suffix('`')).unwrap_or(text).to_string()
}

impl<'a> Parser<'a> {
    /// `identName`: a name of `kind` of the current identifier, then past it.
    fn ident_name(&mut self, kind: K, value: String) -> u32 {
        let n = self.mk_value(kind, Vec::new(), value);
        self.at_cur_pos_next(n)
    }
    /// `name`: a name of `kind` at the current identifier.
    fn name_of(&mut self, kind: K) -> P<u32> {
        if !self.at(C::Ident) {
            return self.expected_c(C::Ident);
        }
        let value = ident_value(self.curr_text());
        Ok(self.ident_name(kind, value))
    }
    /// `termName()`.
    pub(super) fn term_name(&mut self) -> P<u32> {
        self.name_of(K::TermName)
    }
    /// `typeName()`.
    pub(super) fn type_name(&mut self) -> P<u32> {
        self.name_of(K::TypeName)
    }
    /// `termName(ident)`: a `Term.Name` of `value` at the current token, then past it.
    pub(super) fn term_name_value(&mut self, value: String) -> u32 {
        self.ident_name(K::TermName, value)
    }
    /// `nameAsType`: a `Type.Name` of a name's value at its position.
    pub(super) fn name_as_type(&mut self, name: u32) -> u32 {
        let value = self.nodes[name as usize].value.clone().unwrap_or_default();
        let t = self.mk_value(K::TypeName, Vec::new(), value);
        self.copy_pos(name, t)
    }
    /// `anonName()`: a `Name.Anonymous` of no width at the current token.
    pub(super) fn anon_name(&mut self) -> u32 {
        let n = self.mk_value(K::NameAnonymous, Vec::new(), String::new());
        self.at_cur_pos_empty(n)
    }
    /// `anonNameAt(pos)`.
    pub(super) fn anon_name_at(&mut self, pos: i32) -> u32 {
        let n = self.mk_value(K::NameAnonymous, Vec::new(), String::new());
        self.at_pos_empty(pos, n)
    }
    /// `nameThis()`.
    pub(super) fn name_this(&mut self) -> u32 {
        let n = self.mk_value(K::NameThis, Vec::new(), "this".to_string());
        self.at_cur_pos_next(n)
    }
    /// `namePlaceholder()`.
    pub(super) fn name_placeholder(&mut self) -> u32 {
        let n = self.mk_value(K::NamePlaceholder, Vec::new(), "_".to_string());
        self.at_cur_pos_next(n)
    }
    /// `anonThis()`: `this`, a `Term.This` of an anonymous qualifier.
    pub(super) fn anon_this(&mut self) -> u32 {
        let qual = self.anon_name();
        let n = self.mk(K::TermThis, vec![(F::Qual, qual)]);
        self.at_cur_pos_next(n)
    }
    /// A name's `Name.Indeterminate` at its position (`copyPos(x)(Name.Indeterminate(x.value))`).
    pub(super) fn indeterminate(&mut self, name: u32) -> u32 {
        let value = self.nodes[name as usize].value.clone().unwrap_or_default();
        let n = self.mk_value(K::NameIndeterminate, Vec::new(), value);
        self.copy_pos(name, n)
    }

    /// `path(thisOK)`.
    pub(super) fn path(&mut self, this_ok: bool) -> P<u32> {
        match self.curr_c() {
            C::KwThis => {
                let q = self.get_anon_qual();
                self.get_this(q, this_ok)
            }
            C::KwSuper => {
                let q = self.get_anon_qual();
                self.get_super(q)
            }
            _ => {
                let name = self.term_name()?;
                if !self.at(C::Dot) {
                    return Ok(name);
                }
                match self.peek_c() {
                    C::KwThis => {
                        self.next();
                        let q = self.get_qual(name);
                        self.get_this(q, this_ok)
                    }
                    C::KwSuper => {
                        self.next();
                        let q = self.get_qual(name);
                        self.get_super(q)
                    }
                    C::Ident => {
                        self.next();
                        self.selectors(name)
                    }
                    _ => Ok(name),
                }
            }
        }
    }
    fn get_anon_qual(&mut self) -> u32 {
        let n = self.anon_name();
        self.next();
        n
    }
    fn get_qual(&mut self, name: u32) -> u32 {
        self.next();
        self.indeterminate(name)
    }
    fn maybe_selectors(&mut self, r: u32) -> P<u32> {
        if self.at(C::Dot) && self.peek(C::Ident) {
            self.next();
            self.selectors(r)
        } else {
            Ok(r)
        }
    }
    fn get_this(&mut self, name: u32, this_ok: bool) -> P<u32> {
        let start = self.beg_index(name);
        let thisp = self.auto_end_pos(start, |p| Ok(p.mk(K::TermThis, vec![(F::Qual, name)])))?;
        if this_ok {
            self.maybe_selectors(thisp)
        } else {
            self.accept(C::Dot)?;
            self.selectors(thisp)
        }
    }
    fn get_super(&mut self, name: u32) -> P<u32> {
        let start = self.beg_index(name);
        let superp = self.auto_end_pos(start, |p| {
            let mixin = p.mixin_qualifier()?;
            Ok(p.mk(K::TermSuper, vec![(F::Thisp, name), (F::Superp, mixin)]))
        })?;
        self.accept(C::Dot)?;
        let sel = self.auto_end_pos(start, |p| {
            let n = p.term_name()?;
            Ok(p.mk(K::TermSelect, vec![(F::Qual, superp), (F::Name, n)]))
        })?;
        self.maybe_selectors(sel)
    }
    /// `selector(t, startPos)`.
    pub(super) fn selector(&mut self, t: u32, start: i32) -> P<u32> {
        self.auto_end_pos(start, |p| {
            let n = p.term_name()?;
            Ok(p.mk(K::TermSelect, vec![(F::Qual, t), (F::Name, n)]))
        })
    }
    /// `selectors(t)`: `t.a.b`, each from `t`'s start.
    pub(super) fn selectors(&mut self, t: u32) -> P<u32> {
        let start = self.beg_index(t);
        let mut t1 = self.selector(t, start)?;
        while self.at(C::Dot) && self.try_ahead(C::Ident) {
            t1 = self.selector(t1, start)?;
        }
        Ok(t1)
    }
    /// `mixinQualifier()`: `[T]` of `super[T]`, else an anonymous name.
    pub(super) fn mixin_qualifier(&mut self) -> P<u32> {
        if self.accept_opt(C::LeftBracket) {
            let t = self.type_name()?;
            let n = self.indeterminate(t);
            self.in_brackets_after_open(n)
        } else {
            Ok(self.anon_name())
        }
    }
    /// `stableId()`.
    pub(super) fn stable_id(&mut self) -> P<u32> {
        self.path(false)
    }
    /// `qualId()`.
    pub(super) fn qual_id(&mut self) -> P<u32> {
        let name = self.term_name()?;
        if self.accept_opt(C::Dot) { self.selectors(name) } else { Ok(name) }
    }

    /// `literal()`: the current literal, of no unary operator, at its token.
    pub(super) fn literal(&mut self) -> P<u32> {
        let pos = self.curr_index();
        let n = self.raw_literal(None)?;
        Ok(self.at_pos1(pos, n))
    }

    /// `syntaxErrorExpected[T]` for the token class `c`.
    pub(super) fn expected_c<T>(&self, c: C) -> P<T> {
        self.refused(format!("`{}` expected but `{}` found", super::stream::token_class_name(c), super::stream::found_name(self.curr_c(), self.curr_text())))
    }
}

/// A constant token's class and value: an integer's number (`Lit.Int`, `Lit.Long`), a decimal's `BigDecimal` text
/// (`big_decimal_text`), a character's and a string's decoded value.
pub(super) fn constant_value(text: &str) -> Option<(K, String)> {
    constant_with_unary(text, None, false).ok().map(|(k, v, _)| (k, v))
}

/// `rawLiteral`'s constant as scalameta's tokenizer classes it (`LegacyTokenData`): its class and value, a numeric
/// unary operator (`Unary.Numeric`) applied to a number's value where it applies (`-` and `+` to every number, `~` to
/// an integer), and whether it was; past its range an integer a `Lit.IntXL`, a decimal as `decimal_constant` says.
pub(super) fn constant_with_unary(text: &str, unary: Option<&str>, minus_before: bool) -> Result<(K, String, bool), String> {
    if text.starts_with('\'') {
        let inner = &text[1..text.len().saturating_sub(1).max(1)];
        let s = crate::parser::unescape(inner, false, &mut Vec::new(), crate::source::Span::default());
        return Ok((K::LitChar, s, false));
    }
    if text.starts_with('"') {
        let s = match text.strip_prefix("\"\"\"").and_then(|t| t.strip_suffix("\"\"\"")) {
            Some(raw) => raw.to_string(),
            None => crate::parser::unescape(&text[1..text.len().saturating_sub(1).max(1)], false, &mut Vec::new(), crate::source::Span::default()),
        };
        return Ok((K::LitString, s, false));
    }
    let clean: String = text.chars().filter(|&c| c != '_').collect();
    let hex = clean.strip_prefix("0x").or(clean.strip_prefix("0X"));
    let bin = clean.strip_prefix("0b").or(clean.strip_prefix("0B"));
    if hex.is_none() && bin.is_none() && (clean.contains(['.', 'e', 'E']) || clean.ends_with(['d', 'D', 'f', 'F'])) {
        return decimal_constant(text, &clean, unary);
    }
    integer_constant(&clean, unary, minus_before).ok_or_else(|| format!("illegal literal {}", text))?
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DecimalToken {
    /// `FLOATLIT`: an `f` suffix.
    Float,
    /// `DOUBLELIT`: a `d` suffix, or a point and no exponent.
    Double,
    /// `DECIMALLIT`: an exponent and no suffix.
    Decimal,
}

/// `BigDecimal.binary(Double.MaxValue, MathContext(32, UP))` and `BigDecimal.binary(Float.MaxValue, MathContext(8, UP))`
/// (`LegacyTokenData`'s maxima): unscaled digits and scale.
const DOUBLE_MAX: (&str, i64) = ("17976931348623157081452742373171", -277);
const FLOAT_MAX: (&str, i64) = ("34028235", -31);

/// The tokenizer's decimal: a `Float` or a `Double` past its maximum refused (`toBigDec`), a decimal past `Double`'s
/// a `Lit.FloatXL` (`doubleOrDecimalVal`), each compared exactly.
fn decimal_constant(text: &str, clean: &str, unary: Option<&str>) -> Result<(K, String, bool), String> {
    let (body, token) = match clean.strip_suffix(['f', 'F']) {
        Some(b) => (b, DecimalToken::Float),
        None => match clean.strip_suffix(['d', 'D']) {
            Some(b) => (b, DecimalToken::Double),
            None if clean.contains(['e', 'E']) => (clean, DecimalToken::Decimal),
            None => (clean, DecimalToken::Double),
        },
    };
    let (digits, exponent) = match body.find(['e', 'E']) {
        Some(k) => (&body[..k], &body[k + 1..]),
        None => (body, ""),
    };
    let value = big_decimal_opt(digits, exponent);
    let (kind, max, what) = if token == DecimalToken::Float { (K::LitFloat, FLOAT_MAX, "Float") } else { (K::LitDouble, DOUBLE_MAX, "Double") };
    let in_range = value.as_ref().map(|v| at_most(v, max));
    match (token, in_range) {
        (DecimalToken::Decimal, Some(true)) => {}
        (DecimalToken::Decimal, _) => return Ok((K::LitFloatXL, text.to_string(), false)),
        (_, None) => return Err(format!("malformed floating-point {} number: overflow", what)),
        (_, Some(false)) => return Err(format!("floating-point value out of range for {}", what)),
        (_, Some(true)) => {}
    }
    let (text, zero) = crate::fix::nodes::big_decimal_text(body);
    Ok(match unary {
        Some("-") => (kind, if zero { text } else { format!("-{}", text) }, true),
        Some("+") => (kind, text, true),
        _ => (kind, text, false),
    })
}

/// `AnyDecimal.toBigDecimalOpt`: the digits' unscaled value (no leading zero) and its scale, none where the exponent
/// or the scale leaves an `Int`.
fn big_decimal_opt(digits: &str, exponent: &str) -> Option<(String, i64)> {
    let exp: i64 = if exponent.is_empty() {
        0
    } else if exponent.len() > 11 { // scalameta's AnyDecimal: Int.MaxValue's 10 digits and a sign
        return None;
    } else {
        let e: i64 = exponent.parse().ok()?;
        if e != e as i32 as i64 {
            return None;
        }
        e
    };
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let scale = frac.len() as i64 - exp;
    if exp != 0 && scale != scale as i32 as i64 {
        return None;
    }
    Some((format!("{}{}", int, frac).trim_start_matches('0').to_string(), scale))
}

/// `value <= max`, both unscaled digits and a scale, compared digit by digit.
fn at_most(value: &(String, i64), max: (&str, i64)) -> bool {
    let normal = |digits: &str, scale: i64| {
        let trimmed = digits.trim_end_matches('0');
        (trimmed.to_string(), scale - (digits.len() - trimmed.len()) as i64)
    };
    let (v, vs) = normal(&value.0, value.1);
    if v.is_empty() {
        return true;
    }
    let (m, ms) = normal(max.0, max.1);
    let (ve, me) = (v.len() as i64 - vs, m.len() as i64 - ms);
    if ve != me {
        return ve < me;
    }
    let width = v.len().max(m.len());
    format!("{:0<width$}", v) <= format!("{:0<width$}", m)
}

/// The tokenizer's integer (`intVal`, `longVal`): a decimal one of more digits than its leading zero refused
/// (`getNumber`'s `hadLeadingZero`), past `Int` a `Lit.IntXL`, its range wider by one bit where it is not decimal and
/// a `-` written right before it counted; a `Long` past its range refused.
fn integer_constant(clean: &str, unary: Option<&str>, minus_before: bool) -> Option<Result<(K, String, bool), String>> {
    let (body, long) = match clean.strip_suffix(['l', 'L']) {
        Some(b) => (b, true),
        None => (clean, false),
    };
    let (digits, radix) = if let Some(h) = body.strip_prefix("0x").or(body.strip_prefix("0X")) {
        (h, 16)
    } else if let Some(b) = body.strip_prefix("0b").or(body.strip_prefix("0B")) {
        (b, 2)
    } else {
        (body, 10)
    };
    if radix == 10 && digits.len() > 1 && digits.starts_with('0') {
        return Some(Err("Non-zero integral values may not have a leading zero.".to_string()));
    }
    let m = Magnitude::parse(digits, radix)?;
    let max_bits = if long { 64 } else { 32 } + if radix == 10 { 0 } else { 1 };
    let bits = if minus_before && !m.is_zero() { m.minus_one().bit_length() } else { m.bit_length() };
    let folded = unary.is_some();
    let low = match unary {
        Some("-") => m.low_u64().wrapping_neg(),
        Some("~") => !m.low_u64(),
        _ => m.low_u64(),
    };
    Some(Ok(match (long, bits < max_bits) {
        (true, false) => return Some(Err("integer number out of range for Long".to_string())),
        (true, true) => (K::LitLong, (low as i64).to_string(), folded),
        (false, true) => (K::LitInt, (low as u32 as i32).to_string(), folded),
        (false, false) => {
            let text = match unary {
                Some("-") if !m.is_zero() => format!("-{}", m.decimal()),
                Some("~") => format!("-{}", m.plus_one().decimal()),
                _ => m.decimal(),
            };
            (K::LitIntXL, text, folded)
        }
    }))
}

/// A literal integer's magnitude (`BigInt`'s), its limbs in base 2^32, the least first.
struct Magnitude(Vec<u32>);

impl Magnitude {
    fn parse(digits: &str, radix: u32) -> Option<Magnitude> {
        if digits.is_empty() {
            return None;
        }
        let mut limbs: Vec<u32> = Vec::new();
        for ch in digits.chars() {
            let mut carry = ch.to_digit(radix)? as u64;
            for limb in limbs.iter_mut() {
                let t = *limb as u64 * radix as u64 + carry;
                *limb = t as u32;
                carry = t >> 32;
            }
            if carry > 0 {
                limbs.push(carry as u32);
            }
        }
        Some(Magnitude(limbs))
    }
    fn is_zero(&self) -> bool {
        self.0.iter().all(|&l| l == 0)
    }
    /// `BigInteger.bitLength`.
    fn bit_length(&self) -> u32 {
        match self.0.iter().rposition(|&l| l != 0) {
            Some(top) => 32 * top as u32 + (32 - self.0[top].leading_zeros()),
            None => 0,
        }
    }
    fn minus_one(&self) -> Magnitude {
        let mut limbs = self.0.clone();
        for limb in limbs.iter_mut() {
            let (v, borrow) = limb.overflowing_sub(1);
            *limb = v;
            if !borrow {
                break;
            }
        }
        Magnitude(limbs)
    }
    fn plus_one(&self) -> Magnitude {
        let mut limbs = self.0.clone();
        for limb in limbs.iter_mut() {
            let (v, carry) = limb.overflowing_add(1);
            *limb = v;
            if !carry {
                return Magnitude(limbs);
            }
        }
        limbs.push(1);
        Magnitude(limbs)
    }
    fn low_u64(&self) -> u64 {
        self.0.first().copied().unwrap_or(0) as u64 | (self.0.get(1).copied().unwrap_or(0) as u64) << 32
    }
    /// Its decimal digits, by division by 10^9 from the most significant limb.
    fn decimal(&self) -> String {
        let mut limbs = self.0.clone();
        let mut chunks: Vec<u32> = Vec::new();
        while limbs.iter().any(|&l| l != 0) {
            let mut rem = 0u64;
            for limb in limbs.iter_mut().rev() {
                let t = rem << 32 | *limb as u64;
                *limb = (t / 1_000_000_000) as u32;
                rem = t % 1_000_000_000;
            }
            chunks.push(rem as u32);
        }
        match chunks.split_last() {
            None => "0".to_string(),
            Some((top, rest)) => {
                let mut out = top.to_string();
                for c in rest.iter().rev() {
                    out.push_str(&format!("{:09}", c));
                }
                out
            }
        }
    }
}
