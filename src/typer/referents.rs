//! What an expression of the program refers to once typed: the definitions its head names, as
//! dotty's later phases read them off the typed tree's identifiers and selections
//! (`CrossVersionChecks.transformIdent`/`transformSelect`/`transformNew`, `CheckUnused`'s): the
//! symbol a name, a selection or the head of an application resolved to, the class an instance
//! or an object reference names, and whether the reference is the target of an assignment.
//! teq reads it off the typed node of the expression and the expression's head as written.

use super::Worker;
use crate::ast;
use crate::source::Span;
use crate::symbols::Owner;
use crate::tir::{TExpr, TExprId};
use crate::types::{ClassId, SymId};

/// A definition a reference names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Referent {
    Sym(SymId),
    Class(ClassId),
    Alias(crate::types::AliasId),
}

/// The definitions the expression names at its head, at the head's place.
pub struct Reference {
    pub span: Span,
    pub referents: [Option<Referent>; 2],
    /// The head is the target of an assignment (`x = e`, `o.x = e`): written, not read.
    pub assigned: bool,
}

impl<'a> Worker<'a> {
    /// The reference the expression `e`, typed as `te`, makes where its head is a name, a
    /// selection, an operator or an instance: `None` for any other expression.
    pub(super) fn head_reference(&self, e: ast::ExprId, te: TExprId) -> Option<Reference> {
        let ast = self.cur_ast();
        let mut head = e;
        loop {
            match ast.expr(head) {
                ast::Expr::Apply(f, _) | ast::Expr::UsingApply(f, _) | ast::Expr::TypeApply(f, _) => head = f,
                _ => break,
            }
        }
        let mut node = te;
        while let TExpr::Block(_, res) = self.prog.expr(node) {
            node = res;
        }
        let (span, assigned) = match ast.expr(head) {
            // `x += e` of a `var x` the typer makes `x = x + e`: the target written, which the
            // operation's read of it is too, scalac's desugaring sharing the tree.
            ast::Expr::Infix(lhs, _, _) if matches!(self.prog.expr(node), TExpr::Assign(..)) => {
                if let TExpr::Assign(l, _) = self.prog.expr(node) {
                    node = l;
                }
                (ast.expr_span(lhs), true)
            }
            ast::Expr::Ident(_) | ast::Expr::Select(..) | ast::Expr::Infix(..) => (ast.expr_span(head), false),
            // An instance's place is its class's, as scalac's `New` takes the type's.
            ast::Expr::New(t, _) => (ast.ty_spans[t.idx()], false),
            // The assignment's target, read off the node of the assignment.
            ast::Expr::Assign(lhs, _) => {
                let mut target = lhs;
                while let ast::Expr::Apply(f, _) = ast.expr(target) {
                    target = f;
                }
                if !matches!(ast.expr(target), ast::Expr::Ident(_) | ast::Expr::Select(..)) {
                    return None;
                }
                node = match self.prog.expr(node) {
                    TExpr::Assign(l, _) => l,
                    _ => node,
                };
                (ast.expr_span(target), true)
            }
            _ => return None,
        };
        let referents = match self.prog.expr(node) {
            // `f(x)` of a value `f`: the `apply` the typer inserts, and the value it calls it on.
            TExpr::CallMethod(recv, s, _) if self.syms.sym(s).name == crate::names::APPLY && !matches!(ast.expr(head), ast::Expr::Select(_, n) | ast::Expr::Ident(n) if n == crate::names::APPLY) => {
                [Some(Referent::Sym(s)), self.value_referent(recv)]
            }
            // `b += e` of a value with a `+=`: the method, and its receiver, whose own typing the
            // operation's target left unmarked (`Worker::use_expr`).
            TExpr::CallMethod(recv, s, _) if matches!(ast.expr(head), ast::Expr::Infix(..)) && self.assign_op_target_name(ast, head) => [Some(Referent::Sym(s)), self.value_referent(recv)],
            TExpr::Local(s) | TExpr::Static(s) | TExpr::Field(_, s) | TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => [Some(Referent::Sym(s)), None],
            // A function value called: the value (`f(x)` of `f: A => B`).
            TExpr::CallClosure(recv, _) => [self.value_referent(recv), None],
            TExpr::Module(c) | TExpr::New(c, _) => [Some(Referent::Class(c)), None],
            // A constant member folded to its value (`apply.rs`'s `folded_paths`).
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) => match self.folded_paths.get(&node) {
                Some(&(_, s)) => [Some(Referent::Sym(s)), None],
                None => return None,
            },
            // A method's eta-expansion: the closure the typer makes calls it.
            TExpr::Lambda(_, body) if matches!(ast.expr(head), ast::Expr::Ident(_) | ast::Expr::Select(..)) => {
                let mut b = body;
                while let TExpr::Block(_, res) = self.prog.expr(b) {
                    b = res;
                }
                match self.prog.expr(b) {
                    TExpr::CallStatic(s, _) | TExpr::CallMethod(_, s, _) => [Some(Referent::Sym(s)), None],
                    _ => return None,
                }
            }
            TExpr::NewVia(s, _) => [
                Some(Referent::Sym(s)),
                match self.syms.sym(s).owner {
                    Owner::Class(c) => Some(Referent::Class(c)),
                    _ => None,
                },
            ],
            _ => return None,
        };
        Some(Reference { span, referents, assigned })
    }

    /// The value `te` reads where it is a name's: a local, a field, an object.
    fn value_referent(&self, te: TExprId) -> Option<Referent> {
        match self.prog.expr(te) {
            TExpr::Local(v) | TExpr::Static(v) | TExpr::Field(_, v) => Some(Referent::Sym(v)),
            TExpr::Module(c) => Some(Referent::Class(c)),
            _ => None,
        }
    }

    /// Whether the operation `head` is an assignment's (`+=`, not `<=`).
    fn assign_op_target_name(&self, ast: &ast::Ast, head: ast::ExprId) -> bool {
        let ast::Expr::Infix(_, op, _) = ast.expr(head) else { return false };
        let text = self.name_ref(op);
        text.len() > 1 && text.ends_with('=') && !text.starts_with('=') && !matches!(text, "<=" | ">=" | "!=")
    }
}
