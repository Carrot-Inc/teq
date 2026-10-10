//! `--wtostring-interpolated`, scalac's `-Wtostring-interpolated`: the lint of a value a standard
//! interpolator turns into a string by its `toString`, dotty's
//! `StringInterpolatorOpt.lintToString` for `s` and `raw` and `FormatChecker`'s for an `f`
//! conversion of kind `%s` (written, or the default one). A `String` and a primitive value are
//! exempt; `()` is `interpolated Unit value`; anything else `interpolation uses toString`, E209
//! (`BadFormatInterpolation`); of a union, the first part that warns. At the argument for `s`
//! and `raw`, at the conversion for `f`: its letter, or before the part a default one leads.

use super::Worker;
use crate::source::Span;
use crate::types::{Type, TypeId};

/// `FormatChecker.formatPattern`, `%(?:(\d+)\$)?([-#+ 0,(<]+)?(\d+)?(\.\d+)?([tT]?[%a-zA-Z])?`,
/// over the part's bytes: the first conversion's start, its index argument, and its conversion
/// letters' start and text (none where they are missing).
fn first_conversion(part: &str) -> Option<(usize, Option<usize>, Option<(usize, String)>)> {
    let b = part.as_bytes();
    let start = part.find('%')?;
    let mut i = start + 1;
    let digits = |i: usize| b[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut index = None;
    let n = digits(i);
    if n > 0 && b.get(i + n) == Some(&b'$') {
        index = part[i..i + n].parse().ok();
        i += n + 1;
    }
    i += b[i..].iter().take_while(|c| b"-#+ 0,(<".contains(c)).count();
    i += digits(i);
    if b.get(i) == Some(&b'.') && digits(i + 1) > 0 {
        i += 1 + digits(i + 1);
    }
    let cc_start = i;
    if matches!(b.get(i), Some(b't' | b'T')) && b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'%') {
        i += 2;
    } else if b.get(i).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'%') {
        i += 1;
    }
    let cc = (i > cc_start).then(|| (cc_start, part[cc_start..i].to_string()));
    Some((start, index, cc))
}

/// A call of the standard interpolator `kind` (`s`, `raw`, `f`) on a `StringContext` of
/// literal parts, whose arguments, typed at the application depth `depth`, are linted one after
/// another as `StringInterpolatorOpt` lints the interpolation scalac's typer made of it: the
/// parts' texts, and where the source writes each (for `f`'s positions).
#[derive(Clone)]
pub struct InterpCall {
    depth: u32,
    kind: crate::intern::Name,
    parts: Vec<(String, Option<Span>)>,
    next: usize,
}

impl<'a> Worker<'a> {
    /// The application `e` of the file being typed, whose method is selected on an application
    /// (`a(parts).m(args)`): its receiver's arguments as written, the places of the parts of a
    /// `StringContext` it may make, for `interpolation_call` once the call resolves.
    #[cold]
    #[inline(never)]
    pub(super) fn note_interpolation_parts(&mut self, e: crate::ast::ExprId, head: crate::ast::ExprId) {
        use crate::ast::Expr;
        let ast = self.cur_ast();
        let Expr::Select(recv, _) = ast.expr(head) else { return };
        if let Expr::Apply(_, parts) = ast.expr(recv) {
            self.interp_parts = Some((ast.expr_span(e), parts));
        }
    }

    /// The interpolation the call `sym` on `recv` with `args` arguments, applied at `span`, is
    /// (`StringContextIntrinsic`): the standard interpolator's `s`, `raw` or `f` by the
    /// symbol's identity, on the `StringContext` an application of its companion's `apply`
    /// makes (`StringContextApply`: what the typer resolved, whatever the source called it) of
    /// parts the typer made literals (`Literals`: a literal, a constant member), as many as the
    /// arguments and one.
    pub(super) fn interpolation_call(&mut self, sym: crate::types::SymId, recv: Option<crate::tir::TExprId>, span: Span, args: usize) -> Option<InterpCall> {
        use crate::tir::TExpr;
        // The written arguments of this application's receiver, an inner application's left
        // for its own.
        let written = match self.interp_parts {
            Some((at, parts)) if at == span => {
                self.interp_parts = None;
                Some(parts)
            }
            _ => None,
        };
        let context = self.b.string_context?;
        let kind = self.syms.sym(sym).name;
        // The member itself, or one alternative of it (the interpolator beside its pattern's).
        let member = self.syms.class(context).members.get(&kind).copied();
        let is_member = member.is_some_and(|m| m == sym || self.syms.alternatives(m).is_some_and(|alts| alts.contains(&sym)));
        if !is_member || !matches!(kind, crate::names::S_INTERP | crate::names::RAW_INTERP | crate::names::F_INTERP) {
            return None;
        }
        // The companion's `apply`, which the typer makes the instance; a `new` is no
        // `StringContextApply`.
        let TExpr::New(c, ctor_args) = self.prog.expr(recv?) else { return None };
        let applied = written.is_some();
        if c != context || !applied {
            return None;
        }
        let [seq] = self.prog.expr_list(ctor_args)[..] else { return None };
        let TExpr::SeqLit(items) = self.prog.expr(seq) else { return None };
        let items = self.prog.expr_list(items).to_vec();
        if items.len() != args + 1 {
            return None;
        }
        let ast = self.cur_ast();
        let places: Vec<Span> = written.map(|l| ast.expr_list(l).iter().map(|&p| ast.expr_span(p)).collect()).unwrap_or_default();
        let mut parts = Vec::with_capacity(items.len());
        for (i, &item) in items.iter().enumerate() {
            let TExpr::Str(text) = self.prog.expr(item) else { return None };
            parts.push((self.prog.strings[text.idx()].to_string(), places.get(i).copied().filter(|_| places.len() == items.len())));
        }
        Some(InterpCall { depth: self.app_depth, kind, parts, next: 0 })
    }

    /// The argument of type `ty`, written at `span`, of the application at depth `depth`: linted
    /// where it is one of the interpolation's (`interp_call`), not spread (`args: _*`, no
    /// `SeqLiteral`).
    #[inline]
    pub(super) fn lint_interpolation_arg(&mut self, ty: TypeId, span: Span, spread: bool) {
        let Some(call) = self.interp_call.as_mut().filter(|c| c.depth == self.app_depth) else { return };
        let i = call.next;
        call.next += 1;
        let kind = call.kind;
        let part = call.parts.get(i + 1).cloned();
        if spread || !self.diags.policy.tostring_interpolated {
            return;
        }
        if kind == crate::names::F_INTERP {
            // The part's text, its offsets shifted from where the source writes the part
            // (`partPosAt`: a literal's own start, its quote).
            if let Some((text, Some(at))) = part {
                self.lint_formatted_at(ty, i, &text, at.start);
            }
        } else {
            self.lint_interpolated(ty, span);
        }
    }

    /// The argument of type `ty` an `s` or `raw` interpolation writes at `span`.
    pub(super) fn lint_interpolated(&mut self, ty: TypeId, span: Span) {
        if self.diags.policy.tostring_interpolated {
            self.lint_to_string(ty, span);
        }
    }

    /// The argument `i` (from 0) of type `ty` of an `f` interpolation whose parts are written at
    /// `parts` of the source: linted where the part after it leads with a `%s` conversion, or
    /// with none, which makes one (`TypedFormatChecker.checkPart`).
    pub(super) fn lint_formatted(&mut self, ty: TypeId, i: usize, part: Span) {
        if !self.diags.policy.tostring_interpolated {
            return;
        }
        let text = self.source(self.env.file).text.get(part.start as usize..part.end as usize).unwrap_or("").to_string();
        self.lint_formatted_at(ty, i, &text, part.start);
    }

    /// `lint_formatted` of the part `text`, whose offsets are shifted from `base`
    /// (`partPosAt`): the part's start in an interpolation, the literal's own start (its quote)
    /// in a written `StringContext("...")`.
    fn lint_formatted_at(&mut self, ty: TypeId, i: usize, text: &str, base: u32) {
        let n = i + 1;
        let part = Span::new(base, base + text.len() as u32);
        // The default `%s` before the part, where its conversion is missing, literal or another
        // argument's (`insertStringConversion`, whose descriptor has no letter: the place before).
        let at = match first_conversion(text) {
            None => None,
            Some((_, _, Some((_, cc)))) if cc == "%" || cc == "n" => None,
            Some((_, Some(index), cc)) if index != n => {
                let _ = cc;
                None
            }
            Some((_, _, Some((cc_at, cc)))) if cc == "s" || cc == "S" => Some(Some(part.start + cc_at as u32)),
            Some(_) => Some(None),
        };
        let pos = match at {
            None => part.start.saturating_sub(1),
            Some(Some(p)) => p,
            Some(None) => return,
        };
        self.lint_to_string(ty, Span::new(pos, pos + 1));
    }

    /// `lint_formatted` of the argument `i` of the interpolation whose parts are `parts`, at the
    /// part after it where the source writes it.
    pub(super) fn lint_formatted_part(&mut self, ty: TypeId, i: usize, parts: crate::ast::ListRef) {
        if let Some(part) = self.cur_ast().part_span(parts.start as usize + i + 1) {
            self.lint_formatted(ty, i, part);
        }
    }

    /// `checkIsStringify`: whether `ty` warns.
    fn lint_to_string(&mut self, ty: TypeId, span: Span) -> bool {
        let ty = self.deref(ty);
        let ty = self.widen_path(ty);
        let ty = self.widen_lit(ty);
        if let Type::Union(a, b) = self.types.get(ty) {
            return self.lint_to_string(a, span) || self.lint_to_string(b, span);
        }
        let w = crate::source::Warning::id(crate::warnings::id::FORMAT_INTERPOLATION_ERROR);
        let t_string = self.b.t_string;
        if self.is_same(ty, t_string) {
            return false;
        }
        let t_unit = self.b.t_unit;
        if self.is_same(ty, t_unit) {
            self.warn_as(span, "interpolated Unit value", w);
            return true;
        }
        let b = &self.b;
        let primitives = [b.t_boolean, b.t_byte, b.t_short, b.t_char, b.t_int, b.t_long, b.t_float, b.t_double];
        if primitives.iter().any(|&p| p == ty) {
            return false;
        }
        if primitives.iter().any(|&p| self.is_same(ty, p)) {
            return false;
        }
        self.warn_as(span, "interpolation uses toString", w);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::first_conversion;

    #[test]
    fn conversions_are_read_as_the_format_checker_reads_them() {
        assert_eq!(first_conversion("%s "), Some((0, None, Some((1, "s".to_string())))));
        assert_eq!(first_conversion(" x"), None);
        assert_eq!(first_conversion("%2$d"), Some((0, Some(2), Some((3, "d".to_string())))));
        assert_eq!(first_conversion("%-10.2f"), Some((0, None, Some((6, "f".to_string())))));
        assert_eq!(first_conversion("%%"), Some((0, None, Some((1, "%".to_string())))));
        assert_eq!(first_conversion("%tY"), Some((0, None, Some((1, "tY".to_string())))));
    }
}
