//! `@tailrec`. The emitter turns the self tail calls of every def that cannot be overridden into
//! a loop, annotated or not; the annotation asks for scalac's errors where that is impossible.

use super::Worker;
use crate::source::Span;
use crate::tir::*;
use crate::types::SymId;

impl<'a> Worker<'a> {
    /// `first_expr` is where the expressions of the body start in `Program::exprs`, so that the
    /// recursive calls are counted without a walk; `moved` the roots of the expansions moved into
    /// the body's inline calls (`state::expand_pending`), which the count leaves out.
    pub fn check_tailrec(&mut self, sym: SymId, f: FunId, first_expr: usize, moved: &[TExprId], span: Span) {
        let name = self.name_str(self.syms.sym(sym).name);
        if !self.syms.is_effectively_final(sym) {
            let msg = format!(
                "TailRec optimisation not applicable, method {} is neither private nor final so can be overridden",
                name
            );
            self.error(span, msg);
            return;
        }
        let fun = &self.prog.funs[f.idx()];
        let Some(body) = fun.body else { return };
        let arity = fun.params.len();
        let is_self_call = |e: &TExpr| matches!(e, TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) if *s == sym);
        let calls = self.prog.exprs[first_expr..]
            .iter()
            .enumerate()
            .filter(|&(i, e)| is_self_call(e) && !moved.iter().any(|m| m.idx() == first_expr + i))
            .count();
        if calls == 0 {
            let msg = format!("TailRec optimisation not applicable, method {} contains no recursive calls", name);
            self.error(span, msg);
        } else if calls > self.prog.tail_self_calls(sym, arity, body).0 {
            self.error(span, "Cannot rewrite recursive call: it is not in tail position");
        }
    }
}
