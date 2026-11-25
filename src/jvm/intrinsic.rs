//! `@jvm("...")` intrinsics. A template is a sequence of JVM instructions separated by blanks:
//!
//! ```text
//! invokevirtual java/lang/String.length()I          one instruction: the arguments are loaded
//!                                                    in order, as the descriptor types them
//! $0 $1:I invokevirtual java/util/ArrayList.get(I)Ljava/lang/Object;
//! getstatic java/lang/System.out:Ljava/io/PrintStream; $0:L invokevirtual java/io/PrintStream.println(Ljava/lang/Object;)V
//! ```
//!
//! `$n` loads argument `n` at its static type, `$n:T` at the type `T` (`I`, `J`, `D`, `Z`, `C`,
//! `L` for `Object`, or a class descriptor). For a member `$0` is the receiver. The instructions
//! are the invokes, field accesses, `new`, `checkcast`, `instanceof`, `ldc "text"`, `ipush n`,
//! the operand-free instructions listed in `plain_op`, and `rt ... rtcall name(desc)`, which
//! calls a definition of `std/jvm.scala` (the dead-code pass reads the name off the template); a template has no branches, so it needs no
//! frames. What the last instruction leaves is the value of the call.
//!
//! The lowering reads the stack code into nodes (`template.rs`). The templates the typer makes
//! itself (`$hashCode($0)`, `$pf($0, $1)`, ...) are JS text; the backend knows them by that text.

use crate::lir::ErasedType;
use super::classfile::*;

/// Whether a text is the typer's own and no stack code: it opens with `$` and no digit, or
/// with `(`, or it is an enum value's ordinal (`$0.$ordinal`).
pub(super) fn typer_text(text: &str) -> bool {
    text.starts_with('$') && !text[1..].starts_with(|c: char| c.is_ascii_digit()) || text.starts_with('(') || text == "$0.$ordinal"
}

/// Whether a template is one member reference and nothing else, its operands implicit.
pub(super) fn single_form(text: &str, tokens: &[String]) -> bool {
    !text.contains('$') && tokens.len() == 2 && tokens[0].starts_with("invoke")
}

/// Blank-separated tokens; a double-quoted token may hold blanks and the escapes `\n`, `\"`, `\\`.
pub(super) fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some(other) => s.push(other),
                        None => {}
                    },
                    _ => s.push(c),
                }
            }
            out.push(s);
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                s.push(c);
                chars.next();
            }
            out.push(s);
        }
    }
    out
}

/// `owner.name(desc)ret` into its parts.
pub(super) fn split_method(member: &str) -> Option<(&str, &str, &str)> {
    let paren = member.find('(')?;
    let dot = member[..paren].rfind('.')?;
    Some((&member[..dot], &member[dot + 1..paren], &member[paren..]))
}

/// The instructions without operands: opcode, how many values they pop, what they push.
pub(super) fn plain_op(name: &str) -> Option<(u8, usize, Option<ErasedType>)> {
    use ErasedType::*;
    Some(match name {
        "aconst_null" => (op::ACONST_NULL, 0, Some(ErasedType::object())),
        "iconst_0" => (op::ICONST_0, 0, Some(I)),
        "iconst_1" => (op::ICONST_0 + 1, 0, Some(I)),
        "iconst_m1" => (op::ICONST_0 - 1, 0, Some(I)),
        "iadd" => (op::IADD, 2, Some(I)),
        "isub" => (op::ISUB, 2, Some(I)),
        "imul" => (op::IMUL, 2, Some(I)),
        "iand" => (op::IAND, 2, Some(I)),
        "ior" => (op::IOR, 2, Some(I)),
        "ixor" => (op::IXOR, 2, Some(I)),
        "ishl" => (op::ISHL, 2, Some(I)),
        "ishr" => (op::ISHR, 2, Some(I)),
        "iushr" => (op::IUSHR, 2, Some(I)),
        "ineg" => (op::INEG, 1, Some(I)),
        "ladd" => (op::LADD, 2, Some(J)),
        "lsub" => (op::LSUB, 2, Some(J)),
        "lmul" => (op::LMUL, 2, Some(J)),
        "land" => (op::LAND, 2, Some(J)),
        "lor" => (op::LOR, 2, Some(J)),
        "lxor" => (op::LXOR, 2, Some(J)),
        "lneg" => (op::LNEG, 1, Some(J)),
        "dadd" => (op::DADD, 2, Some(D)),
        "dsub" => (op::DSUB, 2, Some(D)),
        "dmul" => (op::DMUL, 2, Some(D)),
        "ddiv" => (op::DDIV, 2, Some(D)),
        "dneg" => (op::DNEG, 1, Some(D)),
        "i2l" => (op::I2L, 1, Some(J)),
        "i2d" => (op::I2D, 1, Some(D)),
        "i2c" => (op::I2C, 1, Some(C)),
        "l2i" => (op::L2I, 1, Some(I)),
        "l2d" => (op::L2D, 1, Some(D)),
        "d2i" => (op::D2I, 1, Some(I)),
        "d2f" => (op::D2F, 1, Some(F)),
        "f2d" => (op::F2D, 1, Some(D)),
        "i2f" => (op::I2F, 1, Some(F)),
        "f2i" => (op::F2I, 1, Some(I)),
        "i2b" => (op::I2B, 1, Some(B)),
        "i2s" => (op::I2S, 1, Some(S)),
        "d2l" => (op::D2L, 1, Some(J)),
        "lcmp" => (op::LCMP, 2, Some(I)),
        "dcmpl" => (op::DCMPL, 2, Some(I)),
        "dcmpg" => (op::DCMPG, 2, Some(I)),
        "athrow" => (op::ATHROW, 1, None),
        _ => return None,
    })
}
