//! `teq compiler check --dump-defs`: every definition a file's recovered tree holds, with its owner, and
//! the tokens the parser's recovery skipped, for the mutation suite (`tests/parser.sh`) to compare
//! a broken file's tree with the tree of the file it was made from.
//!
//! One tab-separated line per record:
//! `def <kind> <name> <owner> <name start>:<name end> <range start>:<range end> <state>` (`-` for
//! the range of a definition the parser made up; the state `incomplete` for a definition whose
//! header the parser could not complete, `-` otherwise), `skip <site> <start>:<end>`,
//! `error <start>:<end> <message>`.
//! An owner is the path of the enclosing definitions, `kind:name` each, after the package.

use crate::ast::*;
use crate::intern::Interner;
use crate::source::Span;
use std::fmt::Write as _;

pub fn dump(asts: &[Ast], errors: &[(Span, String)], interner: &Interner) -> String {
    let mut out = String::new();
    for ast in asts {
        let package: Vec<&str> = ast.package.iter().map(|&n| interner.get(n)).collect();
        let owner = format!("package:{}", package.join("."));
        let mut d = Dumper { ast, interner, out: &mut out };
        for &def in &ast.top_level {
            d.def(def, &owner);
        }
        for r in &ast.recoveries {
            let _ = writeln!(out, "skip\t{}\t{}:{}", r.site.label(), r.span.start, r.span.end);
        }
    }
    for (span, msg) in errors {
        let _ = writeln!(out, "error\t{}:{}\t{}", span.start, span.end, msg.replace('\n', " "));
    }
    out
}

struct Dumper<'a> {
    ast: &'a Ast,
    interner: &'a Interner,
    out: &'a mut String,
}

impl Dumper<'_> {
    fn def(&mut self, id: DefId, owner: &str) {
        let ast = self.ast;
        let def = ast.def(id);
        let kind = match &def.kind {
            DefKind::Val { .. } if def.mods & mods::MUTABLE != 0 => "var",
            DefKind::Val { .. } => "val",
            DefKind::Fun(_) => "def",
            DefKind::Class(c) => match c.kind {
                ClassKind::Class => "class",
                ClassKind::Trait => "trait",
                ClassKind::Object => "object",
                ClassKind::Enum => "enum",
                ClassKind::EnumCase => "case",
            },
            DefKind::TypeAlias { .. } => "type",
            DefKind::Given(_) => "given",
        };
        let name = self.interner.get(def.name);
        let range = match ast.def_ranges.get(id.idx()) {
            Some(&r) if r != NO_RANGE => format!("{}:{}", r.start, r.end),
            _ => "-".to_string(),
        };
        let state = if def.mods & mods::INCOMPLETE != 0 { "incomplete" } else { "-" };
        let _ = writeln!(self.out, "def\t{}\t{}\t{}\t{}:{}\t{}\t{}", kind, name, owner, def.span.start, def.span.end, range, state);
        let me = format!("{}/{}:{}", owner, kind, name);
        match &def.kind {
            DefKind::Val { rhs, .. } => {
                if let Some(e) = rhs {
                    self.expr(*e, &me);
                }
            }
            DefKind::Fun(f) => {
                self.clauses(&f.clauses, &me);
                if let Some(e) = f.body {
                    self.expr(e, &me);
                }
            }
            DefKind::Class(c) => {
                self.clauses(&c.clauses, &me);
                for p in &c.parents {
                    for &(l, _) in &p.args {
                        self.exprs(l, &me);
                    }
                }
                self.stmts(&c.body, &me);
            }
            DefKind::TypeAlias { .. } => {}
            DefKind::Given(g) => {
                self.clauses(&g.clauses, &me);
                if let Some(e) = g.alias {
                    self.expr(e, &me);
                }
                self.stmts(&g.body, &me);
            }
        }
    }

    fn clauses(&mut self, clauses: &[ParamClause], owner: &str) {
        for c in clauses {
            for p in &c.params {
                if let Some(e) = p.default {
                    self.expr(e, owner);
                }
            }
        }
    }

    fn stmts(&mut self, stmts: &[Stmt], owner: &str) {
        for s in stmts {
            match *s {
                Stmt::Def(d) => self.def(d, owner),
                Stmt::Expr(e) => self.expr(e, owner),
                Stmt::Import(_) => {}
            }
        }
    }

    fn exprs(&mut self, l: ListRef, owner: &str) {
        for &e in self.ast.expr_list(l) {
            self.expr(e, owner);
        }
    }

    fn cases(&mut self, l: ListRef, owner: &str) {
        for c in self.ast.case_list(l) {
            if let Some(g) = c.guard {
                self.expr(g, owner);
            }
            self.expr(c.body, owner);
        }
    }

    fn expr(&mut self, e: ExprId, owner: &str) {
        let ast = self.ast;
        match ast.expr(e) {
            Expr::Select(q, _) | Expr::NamedArg(_, q) | Expr::Prefix(_, q) | Expr::Lambda(_, q) | Expr::PolyLambda(_, q)
            | Expr::Parens(q) | Expr::Typed(q, _) | Expr::Unchecked(q) | Expr::Throw(q) | Expr::Quote(q) | Expr::Splice(q) => {
                self.expr(q, owner)
            }
            Expr::Return(v) => {
                if let Some(v) = v {
                    self.expr(v, owner);
                }
            }
            Expr::Apply(f, l) | Expr::UsingApply(f, l) => {
                self.expr(f, owner);
                self.exprs(l, owner);
            }
            Expr::TypeApply(f, _) => self.expr(f, owner),
            Expr::Infix(a, _, b) | Expr::While(a, b) | Expr::Assign(a, b) => {
                self.expr(a, owner);
                self.expr(b, owner);
            }
            Expr::If(c, t, els) | Expr::InlineIf(c, t, els) => {
                self.expr(c, owner);
                self.expr(t, owner);
                if let Some(els) = els {
                    self.expr(els, owner);
                }
            }
            Expr::Match(s, cases) | Expr::InlineMatch(s, cases) => {
                self.expr(s, owner);
                self.cases(cases, owner);
            }
            Expr::Block(l) => {
                let stmts = ast.stmt_list(l).to_vec();
                self.stmts(&stmts, owner);
            }
            Expr::For(l, body, _) => {
                for en in &ast.enumerators[l.range()] {
                    match *en {
                        Enumerator::Gen(_, x) | Enumerator::CaseGen(_, x) | Enumerator::Guard(x) | Enumerator::Val(_, x) => self.expr(x, owner),
                    }
                }
                self.expr(body, owner);
            }
            Expr::Tuple(l) | Expr::NamedTuple(_, l) | Expr::New(_, l) | Expr::Interp(_, _, l) => self.exprs(l, owner),
            Expr::NewAnon(d) => self.def(d, owner),
            Expr::Try(i) => {
                let t = ast.try_expr(i);
                self.expr(t.body, owner);
                self.cases(t.cases, owner);
                if let Some(h) = t.handler {
                    self.expr(h, owner);
                }
                if let Some(f) = t.finalizer {
                    self.expr(f, owner);
                }
            }
            Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::CharLit(_)
            | Expr::StringLit(_) | Expr::UnitLit | Expr::Ident(_) | Expr::SymRef(_) | Expr::This | Expr::ThisOf(_)
            | Expr::QualThis(_) | Expr::ClassOf(_) | Expr::Unsupported(_) | Expr::Withheld(_) | Expr::Super(_) | Expr::Derived | Expr::NullLit
            | Expr::QuoteType(_) | Expr::SplicePat(_) | Expr::Error => {}
        }
    }
}
