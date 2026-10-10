//! The `@nowarn` annotations of the program's files as the reporting policy's suppressions
//! (`warnings.rs`), dotty's `Typer.registerNowarn` and `Run.suppressions.registerNowarn`: an
//! annotation of a definition covers the definition's whole range, its annotations included; one
//! of an ascription (`e: @nowarn`) covers the ascription. Its argument, `value` by position or by
//! name, is read as `-Wconf`'s filters where it is a constant string
//! (`Annotation.argumentConstantString(0)`): a literal as written, anything else typed where the
//! annotation stands and folded as scalac's typer folds constants (a `final val`, a
//! concatenation, a constant of a library). Another argument is a warning, and the annotation
//! suppresses nothing. They are read of the files' trees once the typing is over, so that they
//! are the same whatever order the workers typed in, and the next report of a watch session reads
//! an edited file's anew.

use super::Worker;
use crate::ast::{self, Annot, DefId, Expr, ExprId};
use crate::source::{Diagnostic, FileId, Span};
use crate::symbols::Owner;
use crate::warnings::Suppression;

impl<'a> Worker<'a> {
    /// The suppressions of every program file, with the warnings their filters give, for the
    /// report (`Diagnostics::suppressions`).
    pub fn register_suppressions(&mut self) {
        let mut sups = Vec::new();
        let mut warnings = Vec::new();
        for f in 0..self.files.len() {
            let file = FileId(f as u32);
            if !self.program_source(file) {
                continue;
            }
            let ast = self.ast(file);
            if !ast.has_nowarn {
                continue;
            }
            for (i, def) in ast.defs.iter().enumerate() {
                for a in def.annots.iter().filter(|a| a.name == crate::names::NOWARN) {
                    let range = ast.def_range(DefId(i as u32));
                    self.register(file, a, range, &mut sups, &mut warnings);
                }
                // A parameter's own `@nowarn` covers the parameter, its annotations through its
                // type (the parameter's `ValDef`).
                let clauses: &[ast::ParamClause] = match &def.kind {
                    ast::DefKind::Fun(f) => &f.clauses,
                    ast::DefKind::Class(c) => &c.clauses,
                    ast::DefKind::Given(g) => &g.clauses,
                    _ => &[],
                };
                for p in clauses.iter().flat_map(|c| c.params.iter()) {
                    let annots = ast.param_annots(p);
                    for a in annots.iter().filter(|a| a.name == crate::names::NOWARN) {
                        let start = annots.first().map_or(p.span.start, |first| ast.expr_span(first.instance).start);
                        let range = Span::new(start, ast.ty_spans[p.ty.idx()].end.max(p.span.end));
                        self.register(file, a, range, &mut sups, &mut warnings);
                    }
                }
            }
            for (range, a) in &ast.nowarn_ascriptions {
                self.register(file, a, *range, &mut sups, &mut warnings);
            }
        }
        // The innermost first (`reportSuspendedMessages`' order): the one a warning matches is
        // used, those around it that match it too are superseded.
        sups.sort_by_key(|s| (s.file, std::cmp::Reverse(s.start)));
        self.diags.suppressions = sups;
        self.diags.suppression_warnings = warnings;
    }

    /// The suppression of the annotation `a` of `file` over `range` (`registerNowarn`): its
    /// filter the constant string of its argument, none for `@nowarn` and `@nowarn()`.
    fn register(&mut self, file: FileId, a: &Annot, range: Span, sups: &mut Vec<Suppression>, warnings: &mut Vec<Diagnostic>) {
        let ast = self.ast(file);
        let annot = ast.expr_span(a.instance);
        let first = match ast.expr(a.instance) {
            Expr::New(_, args) => ast.expr_list(args).first().copied(),
            _ => None,
        };
        // Named or by position, the argument of `value`.
        let arg = first.map(|e| match ast.expr(e) {
            Expr::NamedArg(_, v) => v,
            _ => e,
        });
        let conf = match (ast.annot_args(a).first(), arg) {
            (Some(&s), _) => ast.str(s).to_string(),
            (None, None) => String::new(),
            (None, Some(arg)) => match self.constant_string(file, arg) {
                Some(Ok(s)) => s,
                Some(Err(())) => {
                    warnings.push(Diagnostic::plain(file, ast.expr_span(arg), "filter needs to be a compile-time constant string".to_string(), true, true));
                    "none".to_string()
                }
                // An argument teq cannot type where the annotation stands (a name local to a
                // block around it) registers nothing.
                None => return,
            },
        };
        let (sup, warning) = Suppression::new(file, annot, range, &conf);
        if let (Some(msg), Some(arg)) = (warning, arg) {
            warnings.push(Diagnostic::plain(file, ast.expr_span(arg), msg, true, true));
        }
        if sup.start != sup.end {
            sups.push(sup);
        }
    }

    /// The string constant the argument `arg` of `file` is, typed where it stands, as scalac's
    /// typer folds it: `Err` for another value, `None` where its typing reports an error.
    fn constant_string(&mut self, file: FileId, arg: ExprId) -> Option<Result<String, ()>> {
        let span = self.ast(file).expr_span(arg);
        let owner = self.class_around(file, span.start);
        let env = self.env_at(file, owner, span.start);
        let recorded = self.index_mark();
        let capture = self.prog.capture.take();
        let deps = self.deps.take();
        let outer = std::mem::replace(&mut self.typing_annotation, true);
        let mark = self.diags.items.len();
        let frame = self.var_frame_begin();
        let trail = self.snapshot();
        let pending = self.attempts.pending_len();
        let t_string = self.b.t_string;
        // The constant of the argument's type (`argumentConstant`: its type widened from a term
        // reference and normalized, a `ConstantType`), never what evaluating it would give: a
        // literal type it has or its call or member is declared with (`one(0)` of `def one(i:
        // Int): "x"`, whatever `one` does), else the constant the typer folded the tree to, a
        // literal, a constant member, an operation over constants (`ConstFold`'s type).
        let value = self.with_env(env, |t| {
            let (te, ty) = t.type_expr(arg, Some(t_string));
            if t.attempts.pending_len() > pending {
                t.flush_pending_from(pending);
            }
            let ty = t.widen_path(ty);
            t.constant_by_type(te, Some(ty)).or_else(|| t.fold_constant_as_typed(te))
        });
        self.rollback(trail);
        self.var_frame_end(frame);
        // What the argument's typing reports at it is the annotation's; what it does elsewhere
        // on the way keeps its diagnostics.
        let own = |d: &Diagnostic| d.file == file && d.span.start >= span.start && d.span.end <= span.end;
        let failed = self.diags.items[mark..].iter().any(|d| own(d) && !d.is_warning);
        let elsewhere: Vec<Diagnostic> = self.diags.items.drain(mark..).filter(|d| !own(d)).collect();
        self.diags.items.extend(elsewhere);
        self.typing_annotation = outer;
        self.deps = deps;
        self.prog.capture = capture;
        self.index_drop_records_within(recorded, file, span);
        if failed {
            return None;
        }
        Some(match value {
            Some(crate::types::LitVal::Str(n)) => Ok(self.name_str(n)),
            _ => Err(()),
        })
    }

    /// The innermost class or object of `file` whose definition holds `pos`, its owner where
    /// none does: where an annotation's argument is typed.
    fn class_around(&self, file: FileId, pos: u32) -> Owner {
        let ast = self.ast(file);
        let innermost = (0..ast.defs.len())
            .map(|i| DefId(i as u32))
            .filter(|&d| matches!(ast.def(d).kind, ast::DefKind::Class(_)))
            .filter(|&d| {
                let r = ast.def_range(d);
                r != ast::NO_RANGE && r.start <= pos && pos < r.end
            })
            .max_by_key(|&d| ast.def_range(d).start);
        match innermost.and_then(|d| self.def_classes.get(file.0 as usize, &d).copied()) {
            Some(c) => Owner::Class(c),
            None => Owner::Package(self.file_pkgs[file.0 as usize]),
        }
    }
}
