//! scalameta's `Chars` (common2's internal/tokens/Chars.scala) over UTF-16 units, its predicates reading Java's
//! `Character` as JDK 24 answers it (`interp::char_tables`), with trees' `precedence` and `isLeftAssoc` of an
//! operator's name (trees2's internal/trees/package.scala) and the tokens' `isIdentSymbolicInfixOperator` over them.

use crate::interp::char_tables::{LOWER_CASE, TYPES};

/// `java.lang.Character`'s general categories the predicates read.
pub(crate) const UPPERCASE_LETTER: u8 = 1;
pub(crate) const LOWERCASE_LETTER: u8 = 2;
const TITLECASE_LETTER: u8 = 3;
const OTHER_LETTER: u8 = 5;
const LETTER_NUMBER: u8 = 10;
const MATH_SYMBOL: u8 = 25;
const OTHER_SYMBOL: u8 = 28;

/// `scalaLetterTypeMask`: `Ll`, `Lu`, `Lo`, `Lt`, `Nl`.
const SCALA_LETTER_TYPE_MASK: u32 = 1 << LOWERCASE_LETTER | 1 << UPPERCASE_LETTER | 1 << OTHER_LETTER | 1 << TITLECASE_LETTER | 1 << LETTER_NUMBER;

/// `Character.getType` of a UTF-16 unit.
pub(crate) fn get_type(u: u16) -> u8 {
    TYPES[TYPES.partition_point(|&(first, _)| first <= u) - 1].1
}

fn in_runs(runs: &[(u16, u16)], u: u16) -> bool {
    let k = runs.partition_point(|&(first, _)| first <= u);
    k > 0 && u <= runs[k - 1].1
}

/// `isTypeMask(mask)(ch)`.
fn is_type_mask(mask: u32, u: u16) -> bool {
    mask >> get_type(u) & 1 != 0
}

/// `isTypeMask(scalaLetterTypeMask)`.
pub(crate) fn is_scala_letter(u: u16) -> bool {
    is_type_mask(SCALA_LETTER_TYPE_MASK, u)
}

/// `isSpecial`: a math or other symbol.
pub(crate) fn is_special(u: u16) -> bool {
    is_type_mask(1 << MATH_SYMBOL | 1 << OTHER_SYMBOL, u)
}

/// `isOperatorPart`.
pub(crate) fn is_operator_part(u: u16) -> bool {
    matches!(u, 0x7e | 0x21 | 0x40 | 0x23 | 0x25 | 0x5e | 0x2a | 0x2b | 0x2d | 0x3c | 0x3e | 0x3f | 0x3a | 0x3d | 0x26 | 0x7c | 0x2f | 0x5c) || is_special(u)
}

/// `Char.isLower`, `Character.isLowerCase`.
pub(crate) fn is_lower_case(u: u16) -> bool {
    in_runs(&LOWER_CASE, u)
}

/// `escape(value)`: a tab, a backspace, a line feed, a carriage return, a form feed and a backslash as their escapes.
pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{c}' => out.push_str("\\f"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

/// trees' `precedence` of an operator's name: its first unit's class, an assignment's 0.
pub(crate) fn precedence(op: &str) -> i32 {
    let units: Vec<u16> = op.encode_utf16().collect();
    let (Some(&head), Some(&last)) = (units.first(), units.last()) else { return 10 };
    let assign = last == 0x3d;
    let assign_or = |p| if assign { 0 } else { p };
    let assign_if_pred_or = |p, pred: bool| if assign && pred { 0 } else { p };
    match head {
        0x24 | 0x5f => assign_or(1),
        0x7c => assign_or(2),
        0x5e => assign_or(3),
        0x26 => assign_or(4),
        0x3d => 5,
        0x21 => assign_if_pred_or(5, units.len() > 2),
        0x3c | 0x3e => assign_if_pred_or(6, units.len() > 2),
        0x3a => assign_or(7),
        0x2b | 0x2d => assign_or(8),
        0x2a | 0x2f | 0x25 => assign_or(9),
        ch if is_scala_letter(ch) => 1,
        ch => assign_if_pred_or(10, is_operator_part(ch)),
    }
}

/// trees' `isLeftAssoc`: an operator not ending in `:`.
pub(crate) fn is_left_assoc(op: &str) -> bool {
    op.encode_utf16().last() != Some(0x3a)
}

/// The tokens' `isIdentSymbolicInfixOperator` of an identifier's text: backquoted, or its units operator parts,
/// each `_` after one.
pub(crate) fn is_ident_symbolic_infix_operator(text: &str) -> bool {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() || units[0] == 0x60 && units[units.len() - 1] == 0x60 {
        return true;
    }
    let (mut idx, mut non_empty) = (units.len() - 1, false);
    loop {
        if units[idx] == 0x5f {
            if non_empty {
                return true;
            }
            if idx == 0 {
                return false;
            }
            non_empty = false;
        } else {
            if !is_operator_part(units[idx]) {
                return false;
            }
            if idx == 0 {
                return true;
            }
            non_empty = true;
        }
        idx -= 1;
    }
}
