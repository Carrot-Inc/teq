//! `@tailrec`. The emitter turns the self tail calls of every def that cannot be overridden into
//! a loop, annotated or not; the annotation asks for scalac's errors where that is impossible.

use super::Worker;
use crate::source::Span;
use crate::tir::*;
use crate::types::SymId;

impl<'a> Worker<'a> {
    /// The recursive calls are those of the body as its inline calls expanded it (the check runs
    /// after them, as scalac's `TailRec` after `Inlining`), counted by a walk of what the body
    /// reaches, as `TailRec.transformDefDef` walks the transformed tree: an inline argument's
    /// tree that the expansion copied, and an expansion's root moved into its call's node, are
    /// counted where they stand, not where they were made. A lambda's body is walked, its calls
    /// in no tail position; neither a local def's (scalac warns of a call there, which teq does
    /// not) nor a class's the body makes (`TailRecElimination.transform`'s `DefDef` and `TypeDef`).
    pub fn check_tailrec(&mut self, sym: SymId, f: FunId, span: Span) {
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
        let calls = self.prog.descendants(body).without_local_defs().filter(|&e| is_self_call(&self.prog.expr(e))).count();
        if calls == 0 {
            let msg = format!("TailRec optimisation not applicable, method {} contains no recursive calls", name);
            self.error(span, msg);
        } else if calls > self.prog.tail_self_calls(sym, arity, body).0 {
            self.error(span, "Cannot rewrite recursive call: it is not in tail position");
        }
    }
}
