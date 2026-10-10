//! Signature help (docs/TARGETS.md, "The language server"): the signatures of the call whose
//! argument list holds the cursor, the one the call means and the parameter the cursor's
//! argument goes to.
//!
//! The call is read from the request's text in two passes, as completion reads its site. The
//! lexer's tokens give the innermost argument list left open before the cursor (a `(` not
//! closed at its nesting), the commas at its nesting (the argument's index), a `name =` that
//! starts the argument (the parameter it names) and an `=>` at that nesting (the cursor in a
//! function literal's body, where no call is the cursor's); a string, a character or a comment
//! the cursor stands in is the argument it is in. Then a reparse of a copy with the fake
//! identifier at the cursor (`crate::complete::FAKE`, placed only where no token holds the
//! cursor) names what owns that list: an application (`f(..)`, `q.m(..)`, `f[T](..)`,
//! `factory()(..)`), a `new`, an infix operator's list, a constructor pattern; a list that
//! nothing owns as a call (a tuple, parentheses, a condition) passes the search on to the list
//! around it, a block, a brace or a type argument list ends it.
//!
//! The semantics are the build's, read in query mode as completion reads them (`in_query`): the
//! head of a call is resolved by the scope at the offset (a name), by the type of the receiver's
//! node (a member, an extension method, a conversion's member) or by the node the build recorded
//! for the expression (any other callee, and a list that applies an application's result:
//! `Worker::index_value`). The alternative the build chose is the index's record of the head's
//! name; without one, the alternatives the written arguments rule out (their number, their
//! names, the types the build gave them or a literal has) are passed over. The labels are
//! printed with each parameter's range in UTF-16 units of the label.

use super::index::{Files, Kind, Target};
use super::resolve::{TermRef, TypeRef};
use super::{Env, Worker};
use crate::ast::{Ast, Expr, ExprId, Pat, TyExpr};
use crate::intern::Interner;
use crate::lsp::json::{obj, Json};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::token::Tok;
use crate::types::*;
use std::path::Path;

// ---- the call at the cursor, from the text ----

/// What a call's head is, as the text writes it.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Head {
    /// `f(..)`: a name the scope binds.
    Name(String),
    /// `q.m(..)`: a member of the receiver whose node spans `recv`, `path` the receiver as a
    /// path of names where it is one.
    Member { recv: Span, name: String, path: Option<Vec<String>> },
    /// `super.m(..)`, `super[P].m(..)`.
    Super { qualifier: Option<String>, name: String },
    /// `new C(..)`: the class's path; an anonymous class's parent (`new C(..) { .. }`), which
    /// may be abstract.
    New { path: Vec<String>, anonymous: bool },
    /// `a op (..)`: the operator, a member of the left operand, whose node spans `recv`, or
    /// which is a literal, a name or `this` (`left`).
    Infix { recv: Span, name: String, left: Fact },
    /// `case P(..)`: the extractor or the case class the path names.
    Pattern { path: Vec<String> },
    /// Any other expression applied: its value.
    Value,
}

/// What an argument written before the cursor's is, for the alternatives it rules out.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Fact {
    Int,
    Long,
    Double,
    Float,
    Boolean,
    Char,
    String,
    Null,
    /// A function literal of that many parameters.
    Lambda(usize),
    /// A name: the value the scope binds to it.
    Name(String),
    /// `this`: the enclosing class's.
    This,
    /// Another expression: its type is the node the build recorded at its span, if any.
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Arg {
    pub span: Span,
    /// The parameter a `name = value` argument names.
    pub name: Option<String>,
    pub fact: Fact,
}

/// A written argument list of the call, up to the cursor's.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct List {
    pub using: bool,
    /// The span of the expression the list applies to.
    pub callee: Span,
    /// Its arguments; of the cursor's list, those before the cursor's argument.
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Call {
    pub head: Head,
    /// The head's span, the callee of the first list, and its name's span where it has one,
    /// which the index's record of the call is at.
    pub head_span: Span,
    pub name_span: Option<Span>,
    /// Whether type arguments are written on the head.
    pub targs: bool,
    /// The written lists up to the cursor's, which is the last.
    pub lists: Vec<List>,
    /// The cursor's argument in its list, and the parameter a `name =` before it names.
    pub arg: usize,
    pub named: Option<String>,
    /// The argument the cursor stands before, written already, which the alternatives must
    /// take too.
    pub current: Option<Arg>,
    /// Every argument of the cursor's list, of which `current` is taken.
    written: Vec<Arg>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Open {
    Paren,
    Bracket,
    Brace,
    Interp,
    Indent,
}

/// A delimiter left open before the cursor, and what its nesting holds so far.
struct Frame {
    open: Open,
    at: u32,
    commas: usize,
    /// An `=>` at this nesting since the last comma: a function literal's body.
    arrow: bool,
    /// The token that starts the current argument, and the name a `name =` there gives.
    first: usize,
    named: Option<crate::intern::Name>,
}

/// The call whose argument list holds `offset` of `text`.
pub(super) fn call_at(text: &str, offset: u32) -> Option<Call> {
    let mut at = (offset as usize).min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let offset = at as u32;
    let interner = Interner::new();
    let lexed = crate::lexer::lex(text, &interner);
    let tokens = &lexed.tokens;
    let mut stack: Vec<Frame> = Vec::new();
    let mut inside = false;
    let mut last_end = 0;
    for (i, t) in tokens.iter().enumerate() {
        if t.span.start >= offset {
            break;
        }
        // A token the cursor stands in (a string, a name) is the argument it is in.
        if t.span.end > offset {
            inside = true;
            break;
        }
        last_end = t.span.end;
        let open = match t.kind {
            Tok::LParen => Some(Open::Paren),
            Tok::LBracket => Some(Open::Bracket),
            Tok::LBrace | Tok::Splice => Some(Open::Brace),
            Tok::InterpStart => Some(Open::Interp),
            Tok::Indent => Some(Open::Indent),
            _ => None,
        };
        if let Some(open) = open {
            stack.push(Frame { open, at: t.span.start, commas: 0, arrow: false, first: i + 1, named: None });
            continue;
        }
        let close = match t.kind {
            Tok::RParen => Some(Open::Paren),
            Tok::RBracket => Some(Open::Bracket),
            Tok::RBrace => Some(Open::Brace),
            Tok::InterpEnd => Some(Open::Interp),
            Tok::Outdent => Some(Open::Indent),
            _ => None,
        };
        if let Some(close) = close {
            // What the recovery left unbalanced closes with it; a closer that matches nothing
            // open is passed over.
            if let Some(k) = stack.iter().rposition(|f| f.open == close) {
                stack.truncate(k);
            }
            continue;
        }
        let Some(top) = stack.last_mut() else { continue };
        if top.open != Open::Paren {
            continue;
        }
        // `(using name = ..`: the argument starts after the soft keyword, as the parser reads it.
        let soft_using = t.kind == Tok::Ident && i == top.first && top.commas == 0 && interner.get(t.name) == "using";
        if soft_using && !matches!(tokens.get(i + 1).map(|n| n.kind), Some(Tok::RParen | Tok::Comma | Tok::Dot | Tok::LParen)) {
            top.first = i + 1;
            continue;
        }
        match t.kind {
            Tok::Comma => {
                top.commas += 1;
                top.arrow = false;
                top.first = i + 1;
                top.named = None;
            }
            Tok::Arrow | Tok::CtxArrow => top.arrow = true,
            Tok::Eq if i == top.first + 1 && tokens[top.first].kind == Tok::Ident => top.named = Some(tokens[top.first].name),
            _ => {}
        }
    }
    // A comma that ends its line before a closer is a trailing one, which the lexer drops: the
    // cursor after it, the argument after it.
    if !inside && comma_between(text, last_end, offset) {
        if let Some(top) = stack.last_mut().filter(|f| f.open == Open::Paren) {
            top.commas += 1;
            top.arrow = false;
            top.named = None;
        }
    }
    // The fake identifier stands for an argument where none is written yet: where nothing of an
    // argument follows the cursor before a closer or a comma (`f(using |)`, `f(1, |`), never
    // where one does, which it would join or change (`f(|x)`, `f(|-1)`).
    let fake = crate::complete::FAKE;
    let next = skip_trivia(text, offset, text.len() as u32);
    let mark = !inside && text.as_bytes().get(next as usize).map_or(true, |c| matches!(c, b')' | b']' | b'}' | b',' | b';'));
    // The lists that may own the cursor, the innermost first: a block, a brace or a type
    // argument list around it ends the search, an interpolated string is passed through. Whether
    // the cursor stands in a function literal's body the list holds the tree tells, which tells a
    // literal from a function type (`f(x: Int => |Int, 2)`); an `=>` at the list's nesting tells
    // it where the tree names no owner of the list.
    let mut parens: Vec<&Frame> = Vec::new();
    for f in stack.iter().rev() {
        match f.open {
            Open::Paren => parens.push(f),
            Open::Interp => {}
            Open::Bracket | Open::Brace | Open::Indent => break,
        }
    }
    if parens.is_empty() {
        return None;
    }
    let marked = if mark { format!("{}{}{}", &text[..at], fake, &text[at..]) } else { text.to_string() };
    let (asts, _) = crate::frontend::parse_one(&marked, &interner, crate::parser::Syntax { index: true, ..Default::default() });
    // Each `(` before the cursor with the end of its `)`, which bounds a `new`'s first list.
    let mut closes: Vec<(u32, u32)> = Vec::new();
    let mut open: Vec<u32> = Vec::new();
    for t in tokens.iter().take_while(|t| t.span.start < offset) {
        match t.kind {
            Tok::LParen => open.push(t.span.start),
            Tok::RParen => {
                if let Some(o) = open.pop() {
                    closes.push((o, t.span.end));
                }
            }
            _ => {}
        }
    }
    for f in parens {
        for ast in &asts {
            if let Some(mut call) = owner(ast, &marked, f.at, offset, &closes, &interner) {
                // The cursor in a function literal's body that the list holds: no call's.
                if in_lambda_body(ast, &marked, f.at, offset) {
                    return None;
                }
                call.arg = f.commas;
                call.named = f.named.map(|n| interner.get(n).to_string());
                // The argument after the cursor, at its place in the text.
                let shift = if mark { fake.len() as u32 } else { 0 };
                call.current = call.written.get(f.commas).filter(|a| a.span.start >= offset + shift && f.named.is_none()).map(|a| Arg { span: Span::new(a.span.start - shift, a.span.end - shift), ..a.clone() });
                return Some(call);
            }
        }
        if f.arrow {
            return None;
        }
    }
    None
}

/// Whether a comma stands between `from` and `to` of `text`, where no token does, outside the
/// comments there.
fn comma_between(text: &str, from: u32, to: u32) -> bool {
    let b = text.as_bytes();
    let (mut i, to) = (from as usize, (to as usize).min(b.len()));
    while i < to {
        match comment_end(b, i, to) {
            Some(end) => i = end,
            None if b[i] == b',' => return true,
            None => i += 1,
        }
    }
    false
}

/// Whether the offset `at` of the tree's text `text` stands in the body of a function literal
/// that the list opening at `paren` holds: a lambda's or a polymorphic one's (`[T] => |`), from
/// the `=>` before it (`x =>|` and a body on the next line) to its end; a placeholder's
/// (`_ + _`), which has none, past its start, before which the cursor stands before the argument.
fn in_lambda_body(ast: &Ast, text: &str, paren: u32, at: u32) -> bool {
    ast.exprs.iter().enumerate().any(|(i, e)| match *e {
        Expr::Lambda(_, body) | Expr::PolyLambda(_, body) => {
            let (whole, body) = (ast.expr_span(ExprId(i as u32)), ast.expr_span(body));
            let mut from = body.start as usize;
            while from > 0 && text.as_bytes().get(from - 1).is_some_and(|b| b.is_ascii_whitespace()) {
                from -= 1;
            }
            let from = if text.get(..from).is_some_and(|t| t.ends_with("=>")) { from as u32 } else { body.start };
            whole.start > paren && whole.start < at && from <= at && at <= body.end
        }
        _ => false,
    })
}

/// Whether only blanks and comments stand between `from` and `to` of `text`.
pub(super) fn blank(text: &str, from: u32, to: u32) -> bool {
    from <= to && text.get(from as usize..to as usize).is_some() && skip_trivia(text, from, to) == to
}

/// The first position from `from` of `text`, before `to`, that is no blank and in no comment.
pub(super) fn skip_trivia(text: &str, from: u32, to: u32) -> u32 {
    let b = text.as_bytes();
    let to = (to as usize).min(b.len());
    let mut i = from as usize;
    while i < to {
        match comment_end(b, i, to) {
            Some(end) => i = end,
            None if b[i].is_ascii_whitespace() => i += 1,
            None => break,
        }
    }
    i.min(to) as u32
}

/// The end of the comment that starts at `i` of `b`, before `to`, if one does.
fn comment_end(b: &[u8], i: usize, to: usize) -> Option<usize> {
    match (b[i], b.get(i + 1)) {
        (b'/', Some(b'/')) => Some(b[i..to].iter().position(|&c| c == b'\n').map_or(to, |k| i + k)),
        (b'/', Some(b'*')) => {
            let (mut depth, mut j) = (0, i);
            while j < to {
                if b[j] == b'/' && b.get(j + 1) == Some(&b'*') {
                    depth += 1;
                    j += 2;
                } else if b[j] == b'*' && b.get(j + 1) == Some(&b'/') {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        return Some(j);
                    }
                } else {
                    j += 1;
                }
            }
            Some(to)
        }
        _ => None,
    }
}

/// The call of the tree `ast` (of the text `text`, the cursor at `offset`) whose argument list
/// opens at `paren`.
fn owner(ast: &Ast, text: &str, paren: u32, offset: u32, closes: &[(u32, u32)], interner: &Interner) -> Option<Call> {
    let opens = |callee: Span, whole: Span| callee.end <= paren && whole.end > paren && blank(text, callee.end, paren);
    // Where the expression a list applies ends: a `new`'s span holds its further lists, its own
    // ends with the `)` of its first.
    // A `new`'s own span: to the `)` of its first list, though its node's holds the further ones.
    let new_end = |f: ExprId| -> Span {
        let span = ast.expr_span(f);
        let Expr::New(ty, _) = ast.expr(f) else { return span };
        let after = ast.ty_spans[ty.idx()].end;
        let end = closes.iter().find(|&&(o, _)| o >= after && blank(text, after, o)).map_or(span.end, |&(_, end)| end);
        Span::new(span.start, end)
    };
    // A callee in parentheses ends with the `)` of those that open before it.
    let callee_end = |f: ExprId| -> Span {
        let span = ast.expr_span(f);
        let mut end = new_end(f).end;
        while let Some(&(_, close)) = closes.iter().find(|&&(o, c)| o <= span.start && c > end && c <= paren && blank(text, end, c - 1)) {
            end = close;
        }
        Span::new(span.start, end)
    };
    for (i, e) in ast.exprs.iter().enumerate() {
        let id = ExprId(i as u32);
        match *e {
            Expr::Apply(f, _) | Expr::UsingApply(f, _) if opens(callee_end(f), ast.expr_span(id)) => return application(ast, id, offset, &new_end, interner),
            Expr::New(ty, _) if opens(ast.ty_spans[ty.idx()], ast.expr_span(id)) => return application(ast, id, offset, &new_end, interner),
            // `a op (b, c)`, `a op (b)`: the operand's span with its parentheses or without them,
            // which the parser drops around a single expression. A right-associative operator's
            // right operand is its receiver.
            Expr::Infix(l, op, r)
                if !interner.get(op).ends_with(':')
                    && ((matches!(ast.expr(r), Expr::Tuple(_) | Expr::Parens(_) | Expr::UnitLit) && ast.expr_span(r).start == paren)
                        || (text.as_bytes().get(paren as usize) == Some(&b'(') && ast.expr_span(r).start > paren && blank(text, paren + 1, ast.expr_span(r).start))) =>
            {
                let items: Vec<ExprId> = match ast.expr(r) {
                    Expr::Tuple(items) => ast.expr_list(items).to_vec(),
                    Expr::Parens(x) => vec![x],
                    Expr::UnitLit => Vec::new(),
                    _ => vec![r],
                };
                let span = ast.expr_span(id);
                return Some(Call {
                    head: Head::Infix { recv: receiver(ast, l), name: interner.get(op).to_string(), left: fact(ast, l, interner) },
                    head_span: Span::new(span.start, paren),
                    name_span: ast.name_span(id),
                    targs: false,
                    lists: vec![List { using: false, callee: Span::new(span.start, paren), args: args(ast, &items, offset, interner) }],
                    arg: 0,
                    named: None,
                    current: None,
                    written: all_args(ast, &items, interner),
                });
            }
            _ => {}
        }
    }
    // `new C(..) { .. }`: the parent's constructor lists, the one that opens at `paren` found by
    // walking them from the class's name.
    for e in &ast.exprs {
        let Expr::NewAnon(d) = *e else { continue };
        let crate::ast::DefKind::Class(cls) = &ast.def(d).kind else { continue };
        let Some(parent) = cls.parents.first() else { continue };
        let span = ast.ty_spans[parent.ty.idx()];
        if span.end > paren {
            continue;
        }
        let mut pos = span.end;
        for (j, &(items, _)) in parent.args.iter().enumerate() {
            let Some(open) = text.get(pos as usize..).and_then(|t| t.find('(')).map(|k| pos + k as u32) else { break };
            if !blank(text, pos, open) {
                break;
            }
            if open == paren {
                let class = match ast.ty(parent.ty) {
                    TyExpr::Apply(f, _) => f,
                    _ => parent.ty,
                };
                let path = ty_path(ast, class, interner)?;
                let name_span = match ast.ty(class) {
                    TyExpr::Name(_) => Some(ast.ty_spans[class.idx()]),
                    _ => ast.ty_name_span(class),
                };
                let lists = parent.args[..=j].iter().map(|&(l, using)| List { using, callee: span, args: args(ast, ast.expr_list(l), offset, interner) }).collect();
                return Some(Call { head: Head::New { path, anonymous: true }, head_span: span, name_span, targs: false, lists, arg: 0, named: None, current: None, written: all_args(ast, ast.expr_list(items), interner) });
            }
            match closes.iter().find(|&&(o, _)| o == open) {
                Some(&(_, end)) => pos = end,
                None => break,
            }
        }
    }
    for (i, p) in ast.pats.iter().enumerate() {
        let Pat::Ctor(path, subs) = *p else { continue };
        let span = ast.pat_spans[i];
        let callee = ast.expr_span(path);
        if !opens(callee, span) {
            continue;
        }
        let Some(names) = expr_path(ast, path, interner) else { return None };
        let name_span = match ast.expr(path) {
            Expr::Ident(_) => Some(callee),
            _ => ast.name_span(path),
        };
        let all: Vec<Arg> = ast.pat_list(subs).iter().map(|&s| Arg { span: ast.pat_spans[s.idx()], name: None, fact: Fact::Other }).collect();
        let before = all.iter().filter(|a| a.span.end <= offset).cloned().collect();
        return Some(Call { head: Head::Pattern { path: names }, head_span: callee, name_span, targs: false, lists: vec![List { using: false, callee, args: before }], arg: 0, named: None, current: None, written: all });
    }
    None
}

/// The application `app`, whose last list is the cursor's: its head and its lists.
fn application(ast: &Ast, app: ExprId, offset: u32, new_end: &dyn Fn(ExprId) -> Span, interner: &Interner) -> Option<Call> {
    let mut lists: Vec<List> = Vec::new();
    let written = match ast.expr(app) {
        Expr::Apply(_, items) | Expr::UsingApply(_, items) | Expr::New(_, items) => all_args(ast, ast.expr_list(items), interner),
        _ => Vec::new(),
    };
    let mut at = app;
    let mut targs = false;
    let head = loop {
        match ast.expr(at) {
            Expr::Apply(f, items) | Expr::UsingApply(f, items) => {
                let using = matches!(ast.expr(at), Expr::UsingApply(..));
                // What the list applies: a `new` by its own span, which the build records its
                // instance at (`new C[Int](1)(|)`).
                let callee = match ast.expr(f) {
                    Expr::New(..) => new_end(f),
                    _ => receiver(ast, f),
                };
                lists.push(List { using, callee, args: args(ast, ast.expr_list(items), offset, interner) });
                at = f;
            }
            Expr::TypeApply(f, _) => {
                targs = true;
                break f;
            }
            Expr::New(ty, items) => {
                // `new C[T](..)`: the class, its type arguments apart.
                let (class, applied) = match ast.ty(ty) {
                    TyExpr::Apply(f, _) => (f, true),
                    _ => (ty, false),
                };
                let span = ast.ty_spans[ty.idx()];
                lists.push(List { using: false, callee: span, args: args(ast, ast.expr_list(items), offset, interner) });
                lists.reverse();
                let path = ty_path(ast, class, interner)?;
                let name_span = match ast.ty(class) {
                    TyExpr::Name(_) => Some(ast.ty_spans[class.idx()]),
                    _ => ast.ty_name_span(class),
                };
                return Some(Call { head: Head::New { path, anonymous: false }, head_span: span, name_span, targs: applied, lists, arg: 0, named: None, current: None, written });
            }
            _ => break at,
        }
    };
    lists.reverse();
    // A head in parentheses is typed inside them, its node there.
    let head_span = receiver(ast, head);
    let (kind, name_span) = match ast.expr(head) {
        Expr::Ident(n) => (Head::Name(interner.get(n).to_string()), Some(head_span)),
        Expr::Select(q, n) => match ast.expr(q) {
            Expr::Super(p) => (Head::Super { qualifier: (p != crate::names::EMPTY).then(|| interner.get(p).to_string()), name: interner.get(n).to_string() }, ast.name_span(head)),
            _ => (Head::Member { recv: receiver(ast, q), name: interner.get(n).to_string(), path: expr_path(ast, q, interner) }, ast.name_span(head)),
        },
        _ => (Head::Value, None),
    };
    Some(Call { head: kind, head_span, name_span, targs, lists, arg: 0, named: None, current: None, written })
}

/// The span of a receiver's node: inside its parentheses, as the typer types it.
fn receiver(ast: &Ast, mut q: ExprId) -> Span {
    while let Expr::Parens(inner) = ast.expr(q) {
        q = inner;
    }
    ast.expr_span(q)
}

/// The arguments of a list written before the cursor's.
fn args(ast: &Ast, items: &[ExprId], offset: u32, interner: &Interner) -> Vec<Arg> {
    all_args(ast, items, interner).into_iter().filter(|a| a.span.end <= offset).collect()
}

/// The arguments of a list.
fn all_args(ast: &Ast, items: &[ExprId], interner: &Interner) -> Vec<Arg> {
    items
        .iter()
        .map(|&a| {
            let (name, value) = match ast.expr(a) {
                Expr::NamedArg(n, v) => (Some(interner.get(n).to_string()), v),
                _ => (None, a),
            };
            Arg { span: ast.expr_span(value), name, fact: fact(ast, value, interner) }
        })
        .collect()
}

/// What the text says of an expression's value.
fn fact(ast: &Ast, e: ExprId, interner: &Interner) -> Fact {
    match ast.expr(e) {
        Expr::IntLit(_) => Fact::Int,
        Expr::LongLit(_) => Fact::Long,
        Expr::DoubleLit(_) | Expr::DecimalLit(_) => Fact::Double,
        Expr::FloatLit(_) => Fact::Float,
        Expr::BoolLit(_) => Fact::Boolean,
        Expr::CharLit(_) => Fact::Char,
        Expr::StringLit(_) | Expr::Interp(..) => Fact::String,
        Expr::NullLit => Fact::Null,
        Expr::Lambda(params, _) => Fact::Lambda(params.len as usize),
        Expr::Ident(n) => Fact::Name(interner.get(n).to_string()),
        Expr::This => Fact::This,
        Expr::Parens(x) => fact(ast, x, interner),
        // `-1`, `+2.0`: the literal's.
        Expr::Prefix(op, x) if matches!(interner.get(op), "-" | "+") && matches!(ast.expr(x), Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_)) => fact(ast, x, interner),
        _ => Fact::Other,
    }
}

fn expr_path(ast: &Ast, e: ExprId, interner: &Interner) -> Option<Vec<String>> {
    match ast.expr(e) {
        Expr::Ident(n) => Some(vec![interner.get(n).to_string()]),
        Expr::Select(q, n) => {
            let mut p = expr_path(ast, q, interner)?;
            p.push(interner.get(n).to_string());
            Some(p)
        }
        _ => None,
    }
}

fn ty_path(ast: &Ast, t: crate::ast::TyExprId, interner: &Interner) -> Option<Vec<String>> {
    match ast.ty(t) {
        TyExpr::Name(n) => Some(vec![interner.get(n).to_string()]),
        TyExpr::Select(q, n) => {
            let mut p = ty_path(ast, q, interner)?;
            p.push(interner.get(n).to_string());
            Some(p)
        }
        _ => None,
    }
}

// ---- the signatures ----

/// A parameter as a signature shows it; an extractor's component has no name.
#[derive(Clone)]
struct Param {
    name: Option<crate::intern::Name>,
    ty: TypeId,
    by_name: bool,
    repeated: bool,
    default: bool,
}

#[derive(Clone)]
struct Clause {
    using: bool,
    /// A `using` clause written `implicit`, as Scala 2 writes it.
    implicit: bool,
    params: Vec<Param>,
}

/// A signature as the call's syntax shows it: an extension's receiver clause and type
/// parameters left out where the call selects it on the receiver, a constructor named by its
/// class.
#[derive(Clone)]
struct Shown {
    name: String,
    /// What the index's record of the call names when the build chose this one.
    sym: Option<SymId>,
    /// The type parameters shown, and every one a parameter's type may name that no
    /// substitution fixed (an extension's own among them), which no argument is checked against.
    tparams: Vec<TParamId>,
    open: Vec<TParamId>,
    clauses: Vec<Clause>,
    /// The result, none for a constructor or a pattern; a constructor's instance, which a further
    /// list applies (`new C(1)(x)`), not shown.
    ret: Option<TypeId>,
    instance: Option<TypeId>,
    subst: Subst,
    /// A pattern's components, which name no parameter but their types.
    pattern: bool,
    /// For a member `apply` of a value's type, the value's type: where no such alternative takes
    /// the written lists, the extension `apply`s that take the value are tried, as `c(x)` does.
    of_value: Option<TypeId>,
}

/// Where the written lists put the cursor's in a signature.
enum Mapped {
    /// In its clause `clause`, the clauses before it taken by the lists before it.
    At { clause: usize },
    /// Past its clauses, the lists before it consuming that many: the result takes it.
    Past { consumed: usize },
    /// A list it does not take where it stands (a `using` one for a plain clause).
    Wrong,
}

/// What the index holds for the call, read before the query sets it aside.
struct Facts {
    recv: Option<(crate::tir::TExprId, TypeId)>,
    /// The value of the head, and per list of the expression it applies to.
    head: Option<TypeId>,
    callees: Vec<Option<TypeId>>,
    /// The alternative the build chose, by the record of the head's name.
    chosen: Option<SymId>,
    /// The types the build gave the arguments, per list, per argument.
    args: Vec<Vec<Option<TypeId>>>,
    /// An extractor's result at the pattern.
    extracted: Option<TypeId>,
    /// The type the build gave the argument the cursor stands before.
    current: Option<TypeId>,
    /// A `new`'s type arguments as written, in the build's tree (`new C[Int](..)`), which its
    /// instance is read at.
    new_targs: Vec<crate::ast::TyExprId>,
}

impl<'a> Worker<'a> {
    /// The answer to `signature <path> <offset>`: a `SignatureHelp`, or null outside a call.
    pub(super) fn signature(&mut self, path: &Path, offset: u32, files: &Files) -> Json {
        let ids = files.ids_of(path);
        let Some(&first) = ids.first() else { return Json::Null };
        let sources: &'a crate::source::Sources = self.files;
        let text: &'a str = &sources[first.0 as usize].text;
        let Some(call) = call_at(text, offset) else { return Json::Null };
        // The head is resolved where it is written, which the build's tree holds whatever its
        // recovery made of a list cut at the end of the file.
        let site = call.head_span.start;
        let file = self.file_at(&ids, site);
        let facts = self.facts(&ids, file, &call);
        let binders = self.binders(file);
        let base = Env { file, frames: Vec::new(), imports: Vec::new() };
        let answer = self.in_query(base, |w| {
            w.enter_scope_at(file, site, &binders);
            let at = Span::new(site, site);
            let shown = w.head_signatures(&call, &facts, at);
            w.help(&call, &facts, shown)
        });
        self.keep_imports(binders);
        answer
    }

    fn facts(&mut self, ids: &[FileId], file: FileId, call: &Call) -> Facts {
        let node_type = |w: &mut Self, span: Span| w.index_node_at(ids, span).and_then(|te| w.prog.type_of(te));
        let recv = match &call.head {
            Head::Member { recv, .. } | Head::Infix { recv, .. } => self.index_node_at(ids, *recv).and_then(|te| Some((te, self.prog.type_of(te)?))),
            _ => None,
        };
        let head = node_type(self, call.head_span);
        let callees = call.lists.iter().map(|l| node_type(self, l.callee)).collect();
        let chosen = call.name_span.and_then(|s| self.index_record_at(ids, s)).and_then(|(t, k)| match (t, k) {
            (Target::Sym(s), Kind::Call | Kind::Ref) => Some(s),
            _ => None,
        });
        let args = call.lists.iter().map(|l| l.args.iter().map(|a| node_type(self, a.span)).collect()).collect();
        // The pattern's span in the build's tree, which the extractor's result is recorded at.
        let extracted = match call.head {
            Head::Pattern { .. } => {
                let ast = self.ast(file);
                let found = ast.pats.iter().enumerate().find_map(|(i, p)| match *p {
                    Pat::Ctor(path, _) if ast.expr_span(path) == call.head_span => Some(ast.pat_spans[i]),
                    _ => None,
                });
                found.and_then(|span| node_type(self, span))
            }
            _ => None,
        };
        let current = call.current.as_ref().and_then(|a| node_type(self, a.span));
        let new_targs = match call.head {
            Head::New { anonymous: false, .. } if call.targs => {
                let ast = self.ast(file);
                let found = ast.exprs.iter().find_map(|e| match *e {
                    Expr::New(ty, _) if ast.ty_spans[ty.idx()] == call.head_span => match ast.ty(ty) {
                        TyExpr::Apply(_, targs) => Some(ast.ty_list(targs).to_vec()),
                        _ => None,
                    },
                    _ => None,
                });
                found.unwrap_or_default()
            }
            _ => Vec::new(),
        };
        Facts { recv, head, callees, chosen, args, extracted, current, new_targs }
    }

    // ---- what the head names ----

    /// The signatures of the call's head, for its first list.
    fn head_signatures(&mut self, call: &Call, facts: &Facts, at: Span) -> Vec<Shown> {
        match &call.head {
            Head::Name(name) => {
                let Some(n) = self.interner.lookup(name) else { return Vec::new() };
                // A name applied finds an extension method too (`ext(recv)(args)`).
                let outer = std::mem::replace(&mut self.extension_call_head, true);
                let r = self.resolved_term(n, at);
                self.extension_call_head = outer;
                match r {
                    Some(r) => self.term_signatures(r, n),
                    None => Vec::new(),
                }
            }
            Head::Member { name, path, .. } => {
                let Some(n) = self.interner.lookup(name) else { return Vec::new() };
                match facts.recv {
                    Some((te, ty)) => {
                        let ty = self.zonk(ty);
                        let through_this = matches!(self.prog.expr(te), crate::tir::TExpr::This | crate::tir::TExpr::Super(_));
                        self.member_signatures(ty, n, through_this)
                    }
                    // An object or a package of a static path has no node: its member by the path.
                    None => match path.as_ref().and_then(|p| self.term_path(p, at.start)) {
                        Some(TermRef::Package(p)) => match self.quietly(|w| w.pkg_term(p, n)) {
                            Some(r) => self.term_signatures(r, n),
                            None => Vec::new(),
                        },
                        Some(r) => match self.path_object(r) {
                            Some(c) => match self.quietly(|w| w.module_term(c, n)) {
                                Some(r) => self.term_signatures(r, n),
                                None => Vec::new(),
                            },
                            None => Vec::new(),
                        },
                        None => Vec::new(),
                    },
                }
            }
            Head::Infix { name, left, recv } => {
                let Some(n) = self.interner.lookup(name) else { return Vec::new() };
                let known = match facts.recv {
                    Some((_, ty)) => Some(ty),
                    None => self.fact_type(left, Span::new(recv.start, recv.start)),
                };
                match known {
                    Some(ty) => {
                        let ty = self.zonk(ty);
                        self.member_signatures(ty, n, matches!(left, Fact::This))
                    }
                    None => Vec::new(),
                }
            }
            Head::Super { qualifier, name } => {
                let Some(n) = self.interner.lookup(name) else { return Vec::new() };
                let class = self.env.frames.iter().rev().find_map(|f| match f {
                    super::Frame::Class(c) => Some(*c),
                    _ => None,
                });
                let Some(c) = class else { return Vec::new() };
                self.super_signatures(c, qualifier.as_deref(), n)
            }
            Head::New { path, anonymous } => match self.new_class_at(path, at) {
                Some(c) => {
                    let mut shown = self.constructor_signatures(c, *anonymous);
                    // The instance a further list applies at the type arguments written.
                    let tparams = self.syms.class(c).tparams.clone();
                    if !facts.new_targs.is_empty() && facts.new_targs.len() == tparams.len() {
                        let targs: Vec<TypeId> = facts.new_targs.iter().map(|&t| self.resolve_type(t)).collect();
                        let subst: Subst = tparams.into_iter().zip(targs).collect();
                        for s in &mut shown {
                            s.instance = s.instance.map(|i| self.types.subst(i, &subst));
                        }
                    }
                    shown
                }
                None => Vec::new(),
            },
            Head::Pattern { path } => self.pattern_signatures(path, facts.extracted, at),
            Head::Value => match facts.head {
                Some(ty) => self.value_signatures(ty),
                None => Vec::new(),
            },
        }
    }

    /// `super.n` in the class `c` (`super[P].n` with `qualifier`): the alternatives of `n` the
    /// classes behind `c` in its linearisation define (those of `P`'s alone), one that a class
    /// earlier in it overrides passed over, each seen from `c`.
    fn super_signatures(&mut self, c: ClassId, qualifier: Option<&str>, n: crate::intern::Name) -> Vec<Shown> {
        self.complete_class(c);
        let all: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        let mut bases: Vec<(ClassId, TypeId)> = all.iter().skip(1).copied().collect();
        if let Some(q) = qualifier {
            let parents = self.syms.class(c).parents.clone();
            let classes: Vec<ClassId> = parents.into_iter().filter_map(|p| self.class_of(p)).collect();
            let named = classes.into_iter().find(|&k| self.name_ref(self.syms.class(k).name) == q);
            let Some(k) = named else { return Vec::new() };
            self.complete_class(k);
            let within: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
            bases.retain(|(b, _)| within.contains(b));
        }
        let mut picked: Vec<(SymId, TypeId)> = Vec::new();
        for &(b, bt) in &bases {
            self.complete_class(b);
            let mut k = 0;
            while let Some(m) = self.own_alternative(b, n, k) {
                k += 1;
                if self.is_private(m) {
                    continue;
                }
                let overridden = picked.clone().into_iter().any(|(p, _)| self.same_parameters(c, &all, p, m, bt));
                if !overridden {
                    picked.push((m, bt));
                }
            }
        }
        let mut out = Vec::new();
        for (m, bt) in picked {
            let subst = self.owner_subst(bt);
            if self.syms.sym(m).kind == SymKind::Def {
                let sig = self.sig_arc(m);
                out.push(self.method_shown(n, m, &sig, subst, false));
            } else {
                let ty = self.sig_of(m).ret;
                let ty = self.types.subst(ty, &subst);
                out.extend(self.value_signatures(ty));
            }
        }
        out
    }

    /// The signatures a term the scope or a path resolves to takes arguments with: a method's
    /// alternatives, a class's constructors or its companion's `apply`, a value's `apply`.
    fn term_signatures(&mut self, r: TermRef, name: crate::intern::Name) -> Vec<Shown> {
        let (s, owner_ty) = match r {
            TermRef::Class(c) | TermRef::ValueClass(_, c) | TermRef::ModuleClass(_, c) => return self.class_apply_signatures(c),
            TermRef::Package(_) => return Vec::new(),
            TermRef::SelfAlias(c) => {
                let ty = self.syms.this_type(c);
                return self.value_signatures(ty);
            }
            TermRef::This(c, s) => (s, Some(self.syms.this_type(c))),
            TermRef::ModuleMember(c, s) => (s, Some(self.types.class(c, &[]))),
            TermRef::Local(s) | TermRef::Global(s) | TermRef::ValueMember(_, s) => (s, None),
        };
        if let SymKind::Object(c) = self.syms.sym(s).kind {
            let ty = self.types.class(c, &[]);
            return self.value_signatures(ty);
        }
        let alts: Vec<SymId> = self.syms.alternatives(s).map_or_else(|| vec![s], |a| a.to_vec());
        let subst = owner_ty.map(|o| self.owner_subst(o)).unwrap_or_default();
        let mut out = Vec::new();
        for alt in alts {
            if !self.is_accessible(alt) {
                continue;
            }
            if self.syms.sym(alt).kind == SymKind::Def {
                let sig = self.sig_arc(alt);
                out.push(self.method_shown(name, alt, &sig, subst.clone(), false));
            } else {
                // A value: what its type's `apply` takes.
                let ty = self.sig_of(alt).ret;
                let ty = self.types.subst(ty, &subst);
                out.extend(self.value_signatures(ty));
            }
        }
        out
    }

    /// The signatures of the member `n` of a receiver of type `t`: its alternatives the site
    /// reaches, seen from the receiver; else the extension methods that take the receiver;
    /// else the members of the one conversion of the nearest level that gives one.
    fn member_signatures(&mut self, t: TypeId, n: crate::intern::Name, through_this: bool) -> Vec<Shown> {
        if t == ERROR || self.types.contains_error(t) {
            return Vec::new();
        }
        if let Some((s, owner_ty)) = self.find_member(t, n) {
            let shown = self.alternatives_seen_from(s, owner_ty, n, Some((through_this, t)));
            if !shown.is_empty() {
                return shown;
            }
        }
        let shown = self.extension_signatures(t, n);
        if !shown.is_empty() {
            return shown;
        }
        let targets = self.conversion_targets(t);
        let mut level: Option<usize> = None;
        let mut found: Vec<(TypeId, SymId, TypeId)> = Vec::new();
        for (_, to, l) in targets {
            if level.is_some_and(|k| k != l) {
                continue;
            }
            if let Some((s, owner_ty)) = self.find_member(to, n) {
                level = Some(l);
                found.push((to, s, owner_ty));
            }
        }
        match found.as_slice() {
            [(to, s, owner_ty)] => {
                let (to, s, owner_ty) = (*to, *s, *owner_ty);
                self.alternatives_seen_from(s, owner_ty, n, Some((false, to)))
            }
            _ => Vec::new(),
        }
    }

    /// The alternatives of `s`, members of `owner_ty`, those the site reaches from a receiver
    /// (`reach`: through `this`, of that type), each seen from the owner; a value's `apply`.
    fn alternatives_seen_from(&mut self, s: SymId, owner_ty: TypeId, n: crate::intern::Name, reach: Option<(bool, TypeId)>) -> Vec<Shown> {
        let alts: Vec<SymId> = self.syms.alternatives(s).map_or_else(|| vec![s], |a| a.to_vec());
        let subst = self.owner_subst(owner_ty);
        let mut out = Vec::new();
        for alt in alts {
            let reached = match reach {
                Some((through_this, recv_ty)) => self.member_reachable(alt, through_this, recv_ty),
                None => self.is_accessible(alt),
            };
            if !reached {
                continue;
            }
            match self.syms.sym(alt).kind {
                SymKind::Def => {
                    let sig = self.sig_arc(alt);
                    out.push(self.method_shown(n, alt, &sig, subst.clone(), false));
                }
                SymKind::Object(c) => {
                    let ty = self.types.class(c, &[]);
                    out.extend(self.value_signatures(ty));
                }
                _ => {
                    let ty = self.sig_of(alt).ret;
                    let ty = self.types.subst(ty, &subst);
                    out.extend(self.value_signatures(ty));
                }
            }
        }
        out
    }

    /// The extension methods `n` that take a receiver of type `t`, as the selection finds them
    /// (lexically, in the receiver's implicit scope, as members of a given's type), each with
    /// its receiver clause and type parameters left out.
    fn extension_signatures(&mut self, t: TypeId, n: crate::intern::Name) -> Vec<Shown> {
        use super::apply::MethodCall;
        let mut found: Vec<(SymId, Subst, Option<TypeId>)> = Vec::new();
        for e in self.muted(|w| w.lexical_extensions(n)) {
            let trait_owner = match self.syms.sym(e).owner {
                Owner::Class(c) if self.syms.class(c).kind != ClassKind::Object => Some(c),
                _ => None,
            };
            let subst = match trait_owner {
                Some(c) => {
                    let site = self.trait_member_site(e, c);
                    self.trait_member_subst(site, c)
                }
                None => Vec::new(),
            };
            found.push((e, subst, None));
        }
        let mark = self.ext_modules.len();
        let in_scope = self.muted(|w| w.implicit_scope_extensions(t, n));
        let mut sites: Vec<Option<(SymId, super::implicits::GivenScope)>> = self.ext_modules[mark..].iter().copied().map(Some).collect();
        self.ext_modules.truncate(mark);
        for e in in_scope {
            let module = sites.iter_mut().find(|s| s.is_some_and(|(x, _)| x == e)).and_then(Option::take).map(|(_, m)| m);
            let trait_owner = match self.syms.sym(e).owner {
                Owner::Class(c) if self.syms.class(c).kind != ClassKind::Object => Some(c),
                _ => None,
            };
            let (subst, prefix) = match (module, trait_owner) {
                (Some(scope), Some(c)) => self.scope_extension_site(scope, c),
                _ => (Vec::new(), None),
            };
            found.push((e, subst, prefix));
        }
        let givens = self.muted(|w| w.givens_with_extension(n, t).0);
        for ((g, _), given_ty) in givens {
            let tparams = self.sig_of(g).tparams.clone();
            let fresh: Vec<(TParamId, TypeId)> = tparams.iter().map(|&p| (p, self.fresh_var())).collect();
            let given_ty = self.types.subst(given_ty, &fresh);
            let Some(gc) = self.class_of(given_ty) else { continue };
            self.complete_class(gc);
            let bases: Vec<ClassId> = self.syms.class(gc).base_types.iter().map(|&(b, _)| b).collect();
            for b in bases {
                let exts: Vec<SymId> = self.syms.class(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == n).collect();
                for e in exts {
                    if let Some(bt) = self.base_type(given_ty, b) {
                        let subst = self.owner_subst(bt);
                        found.push((e, subst, None));
                    }
                }
            }
        }
        let mut out: Vec<Shown> = Vec::new();
        let mut offered: Vec<SymId> = Vec::new();
        for (e, subst, prefix) in found {
            if !self.syms.sym(e).is_extension || offered.contains(&e) {
                continue;
            }
            let call = MethodCall { recv: None, sym: e, owner_subst: subst, ext_recv: None, prefix };
            let mark = self.snapshot();
            let applies = self.extension_applicable(&call, t);
            self.rollback(mark);
            if !applies {
                continue;
            }
            let sig = self.sig_arc(e);
            let sig = match call.prefix {
                Some(prefix) => self.sig_seen_from(sig, prefix, e),
                None => sig,
            };
            let subst = self.extension_receiver_subst(e, &sig, call.owner_subst, t);
            out.push(self.method_shown(n, e, &sig, subst, true));
            offered.push(e);
        }
        out
    }

    /// `subst` with the extension's own type parameters, which its receiver clause takes, fixed
    /// by the receiver's type `t` where it fixes them (`extension [T](xs: List[T])` on a
    /// `List[Int]`: `T` is `Int`).
    fn extension_receiver_subst(&mut self, e: SymId, sig: &MethodSig, mut subst: Subst, t: TypeId) -> Subst {
        let (ext_tparams, ext_clauses) = (self.syms.sym(e).ext_tparams as usize, self.syms.sym(e).ext_clauses as usize);
        let Some(rc) = sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using) else { return subst };
        let Some(param) = sig.clauses[rc].params.first() else { return subst };
        if ext_tparams == 0 {
            return subst;
        }
        let vars: Vec<(TParamId, TypeId)> = sig.tparams.iter().take(ext_tparams).map(|&p| (p, self.fresh_var())).collect();
        let mut all = subst.clone();
        all.extend(vars.iter().copied());
        let declared = self.types.subst(param.ty, &all);
        let mark = self.snapshot();
        if self.is_sub(t, declared) {
            for (p, v) in vars {
                let fixed = self.solve_in(v);
                let fixed = self.zonk(fixed);
                if !self.types.has_vars(fixed) {
                    subst.push((p, fixed));
                }
            }
        }
        self.rollback(mark);
        subst
    }

    /// A method's signature as the call shows it: a selected extension without its receiver
    /// clause and the type parameters that clause takes.
    fn method_shown(&mut self, n: crate::intern::Name, s: SymId, sig: &MethodSig, subst: Subst, selected_extension: bool) -> Shown {
        let (ext_tparams, ext_clauses) = if selected_extension && self.syms.sym(s).is_extension {
            let info = self.syms.sym(s);
            (info.ext_tparams as usize, info.ext_clauses as usize)
        } else {
            (0, 0)
        };
        let recv_clause = if ext_clauses > 0 { sig.clauses.iter().take(ext_clauses).position(|c| !c.is_using) } else { None };
        let clauses = sig.clauses.iter().enumerate().filter(|(i, _)| Some(*i) != recv_clause).map(|(_, c)| clause_of(c)).collect();
        Shown { name: super::complete::spelled(&self.name_str(n)), sym: Some(s), tparams: sig.tparams.iter().skip(ext_tparams).copied().collect(), open: sig.tparams.clone(), clauses, ret: Some(sig.ret), instance: None, subst, pattern: false, of_value: None }
    }

    /// What a value of type `t` takes arguments with: a function's parameters, else its type's
    /// `apply` alternatives.
    fn value_signatures(&mut self, t: TypeId) -> Vec<Shown> {
        let t = self.zonk(t);
        if t == ERROR || self.types.contains_error(t) {
            return Vec::new();
        }
        if let Some((params, ret)) = self.as_function(t) {
            let params = params.iter().enumerate().map(|(i, &ty)| Param { name: Some(self.interner.intern(&format!("v{}", i + 1))), ty, by_name: false, repeated: false, default: false }).collect();
            return vec![Shown { name: "apply".to_string(), sym: None, tparams: Vec::new(), open: Vec::new(), clauses: vec![Clause { using: false, implicit: false, params }], ret: Some(ret), instance: None, subst: Vec::new(), pattern: false, of_value: None }];
        }
        match self.find_member(t, crate::names::APPLY) {
            Some((s, owner_ty)) if self.syms.sym(s).kind == SymKind::Def || matches!(self.syms.sym(s).kind, SymKind::Overloaded(_)) => {
                let mut out = self.alternatives_seen_from(s, owner_ty, crate::names::APPLY, Some((false, t)));
                for s in &mut out {
                    s.of_value = Some(t);
                }
                out
            }
            // No member `apply`: an extension `apply` that takes the value, as `c(x)` finds it.
            None => self.extension_signatures(t, crate::names::APPLY),
            _ => Vec::new(),
        }
    }

    /// `C(..)` of a class's name: its companion's `apply` alternatives, the one the compiler
    /// makes for a case class shown as the constructor; else the constructors.
    fn class_apply_signatures(&mut self, c: ClassId) -> Vec<Shown> {
        self.complete_class(c);
        let companion = self.syms.class(c).companion.filter(|&k| self.syms.class(k).kind == ClassKind::Object);
        if let Some(module) = companion {
            let module_ty = self.types.class(module, &[]);
            if let Some((s, owner_ty)) = self.find_member(module_ty, crate::names::APPLY) {
                let alts: Vec<SymId> = self.syms.alternatives(s).map_or_else(|| vec![s], |a| a.to_vec());
                let subst = self.owner_subst(owner_ty);
                let mut out = Vec::new();
                let mut made_up = false;
                for alt in alts {
                    if self.syms.sym(alt).kind != SymKind::Def || !self.is_accessible(alt) {
                        continue;
                    }
                    if self.syms.sym(alt).def.is_none() && !self.in_jar(self.syms.sym(alt).file) {
                        made_up = true;
                        continue;
                    }
                    let sig = self.sig_arc(alt);
                    out.push(self.method_shown(crate::names::APPLY, alt, &sig, subst.clone(), false));
                }
                if made_up || out.is_empty() {
                    let mut ctors = self.constructor_signatures(c, false);
                    ctors.append(&mut out);
                    return ctors;
                }
                return out;
            }
        }
        self.constructor_signatures(c, false)
    }

    /// The constructors of a class: its primary one, then its secondary ones, named by the
    /// class.
    fn constructor_signatures(&mut self, c: ClassId, anonymous: bool) -> Vec<Shown> {
        self.complete_class(c);
        if self.loaded.is_some() {
            self.absorb_java_ctors(c);
        }
        let (kind, mods) = (self.syms.class(c).kind, self.syms.class(c).mods);
        if !matches!(kind, ClassKind::Class | ClassKind::EnumCase) || (mods & crate::ast::mods::ABSTRACT != 0 && !anonymous) {
            return Vec::new();
        }
        let name = super::complete::spelled(&self.name_str(self.syms.class(c).name));
        let tparams = self.syms.class(c).tparams.clone();
        let secondaries = self.syms.class(c).ctors.clone();
        let instance = Some(self.syms.class(c).base_types[0].1);
        let mut out = Vec::new();
        if (secondaries.is_empty() || !self.is_java_class(c)) && self.ctor_accessible(c) {
            let clauses = self.syms.class(c).ctor.iter().map(clause_of).collect();
            let sym = self.syms.class(c).primary_ctor;
            out.push(Shown { name: name.clone(), sym, tparams: tparams.clone(), open: tparams.clone(), clauses, ret: None, instance, subst: Vec::new(), pattern: false, of_value: None });
        }
        for s in secondaries {
            if !self.ctor_callable_here(s) {
                continue;
            }
            let sig = self.sig_arc(s);
            let clauses = sig.clauses.iter().map(clause_of).collect();
            let mut open = tparams.clone();
            open.extend(sig.tparams.iter().copied());
            out.push(Shown { name: name.clone(), sym: Some(s), tparams: tparams.clone(), open, clauses, ret: None, instance, subst: Vec::new(), pattern: false, of_value: None });
        }
        out
    }

    /// The class a path of a `new` names at the site.
    fn new_class_at(&mut self, path: &[String], at: Span) -> Option<ClassId> {
        let (last, prefix) = path.split_last()?;
        let n = self.interner.lookup(last)?;
        let r = if prefix.is_empty() {
            self.resolved_type(n, at)?
        } else {
            match self.term_path(prefix, at.start)? {
                TermRef::Package(p) => self.quietly(|w| w.pkg_type(p, n))?,
                other => {
                    let c = self.path_object(other)?;
                    self.quietly(|w| w.module_type(c, n))?
                }
            }
        };
        match r {
            TypeRef::Class(c) => Some(c),
            TypeRef::Alias(a) => {
                self.complete_alias(a);
                let rhs = self.syms.aliases[a.idx()].rhs;
                let rhs = self.dealias(rhs);
                self.class_of(rhs)
            }
            _ => None,
        }
    }

    /// A constructor pattern's components: a case class's fields, else what its extractor's
    /// result holds (`Option[(A, B)]` two, `Option[A]` one, an `unapplySeq`'s elements
    /// repeated), at the type the build gave the result where it recorded one.
    fn pattern_signatures(&mut self, path: &[String], extracted: Option<TypeId>, at: Span) -> Vec<Shown> {
        let r = match path {
            [one] => {
                let Some(n) = self.interner.lookup(one) else { return Vec::new() };
                self.resolved_term(n, at)
            }
            _ => self.term_path(path, at.start),
        };
        let Some(r) = r else { return Vec::new() };
        let name = super::complete::spelled(path.last().map_or("", |s| s.as_str()));
        let case_class = |w: &Self, c: ClassId| w.syms.class(c).mods & crate::ast::mods::CASE != 0 || w.syms.class(c).kind == ClassKind::EnumCase;
        let fields = |w: &mut Self, c: ClassId| -> Vec<Shown> {
            w.complete_class(c);
            let first = w.syms.class(c).ctor.first().map(clause_of).unwrap_or(Clause { using: false, implicit: false, params: Vec::new() });
            let tparams = w.syms.class(c).tparams.clone();
            vec![Shown { name: name.clone(), sym: None, tparams: tparams.clone(), open: tparams, clauses: vec![first], ret: None, instance: None, subst: Vec::new(), pattern: false, of_value: None }]
        };
        let module = match r {
            TermRef::Class(c) if case_class(self, c) && extracted.is_none() => return fields(self, c),
            TermRef::Class(c) => self.syms.class(c).companion.filter(|&k| self.syms.class(k).kind == ClassKind::Object),
            other => self.path_object(other),
        };
        let Some(module) = module else { return Vec::new() };
        let module_ty = self.types.class(module, &[]);
        let found = self.find_member(module_ty, crate::names::UNAPPLY).map(|f| (f, false)).or_else(|| self.find_member(module_ty, crate::names::UNAPPLY_SEQ).map(|f| (f, true)));
        let Some(((s, owner_ty), seq)) = found else { return Vec::new() };
        let s = self.syms.alternatives(s).and_then(|a| a.first().copied()).unwrap_or(s);
        // The case class's own extractor, which the compiler makes: its fields, by name.
        if self.syms.sym(s).def.is_none() && !self.in_jar(self.syms.sym(s).file) && extracted.is_none() {
            if let Some(c) = self.syms.class(module).companion.filter(|&c| case_class(self, c)) {
                return fields(self, c);
            }
        }
        let rty = match extracted {
            Some(t) => self.zonk(t),
            None => {
                let ret = self.sig_of(s).ret;
                let subst = self.owner_subst(owner_ty);
                self.types.subst(ret, &subst)
            }
        };
        let Some(components) = self.extracted_components(rty, seq) else { return Vec::new() };
        let params = components.into_iter().map(|(ty, repeated)| Param { name: None, ty, by_name: false, repeated, default: false }).collect();
        vec![Shown { name, sym: None, tparams: Vec::new(), open: Vec::new(), clauses: vec![Clause { using: false, implicit: false, params }], ret: None, instance: None, subst: Vec::new(), pattern: true, of_value: None }]
    }

    /// What an extractor's result `rty` matches: each component's type, the last repeated for
    /// an `unapplySeq`.
    fn extracted_components(&mut self, rty: TypeId, seq: bool) -> Option<Vec<(TypeId, bool)>> {
        if rty == self.b.t_boolean {
            return Some(Vec::new());
        }
        let option = self.b.option.and_then(|o| self.base_type(rty, o));
        let value = match option.map(|o| self.types.get(o)) {
            Some(Type::Class(_, args)) => self.types.items(args)[0],
            _ => {
                // A product result: its fields.
                let fields = self.class_field_types(rty)?;
                return Some(fields.into_iter().map(|t| (t, false)).collect());
            }
        };
        let value = self.zonk(value);
        let value_d = self.deref(value);
        let tuple = match self.types.get(value_d) {
            Type::Class(c, args) => {
                let items = self.types.items(args).to_vec();
                (items.len() > 1 && self.std_class(&format!("Tuple{}", items.len())) == Some(c)).then_some(items)
            }
            _ => None,
        };
        if seq {
            // An `unapplySeq`'s `Seq[T]`, or a product of fixed components and a `Seq[T]` last.
            let seq_class = self.std_class("Seq")?;
            let elem_of = |w: &mut Self, t: TypeId| match w.base_type(t, seq_class).map(|b| w.types.get(b)) {
                Some(Type::Class(_, args)) => Some(w.types.items(args)[0]),
                _ => None,
            };
            if let Some(elem) = elem_of(self, value) {
                return Some(vec![(elem, true)]);
            }
            let mut items = tuple?;
            let last = items.pop()?;
            let elem = elem_of(self, last)?;
            let mut out: Vec<(TypeId, bool)> = items.into_iter().map(|t| (t, false)).collect();
            out.push((elem, true));
            return Some(out);
        }
        Some(match tuple {
            Some(items) if items.len() > 1 => items.into_iter().map(|t| (t, false)).collect(),
            _ => vec![(value, false)],
        })
    }

    // ---- the answer ----

    /// The `SignatureHelp` of the call: the signatures its cursor's list takes, the active one
    /// and its active parameter.
    fn help(&mut self, call: &Call, facts: &Facts, head: Vec<Shown>) -> Json {
        let mut shown = head;
        let mut lists: Vec<&List> = call.lists.iter().collect();
        let mut arg_types: Vec<Vec<Option<TypeId>>> = facts.args.clone();
        let mut chosen = facts.chosen;
        // Where every alternative leaves the cursor's list to its result, the result's `apply`
        // takes it, and so on through the results of the lists between: each the value the build
        // recorded for what its list applies, else the result of the one before (the head's first
        // alternative's), whose `apply` takes as many lists as its clauses.
        let past: Vec<Option<usize>> = shown.iter().map(|s| match self.mapped(s, &lists) {
            Mapped::Past { consumed } => Some(consumed),
            _ => None,
        }).collect();
        if !shown.is_empty() && past.iter().all(Option::is_some) {
            let k = lists.len() - 1;
            let mut from = past[0].unwrap_or(0).min(k);
            let mut value = shown[0].ret.or(shown[0].instance).map(|r| self.types.subst(r, &shown[0].subst));
            let mut applied: Vec<Shown> = Vec::new();
            loop {
                if let Some(t) = facts.callees.get(from).copied().flatten().filter(|_| from > 0 || matches!(call.head, Head::Value)) {
                    value = Some(t);
                }
                let Some(t) = value else {
                    applied.clear();
                    break;
                };
                applied = self.value_signatures(t);
                let rest: Vec<&List> = lists[from..].to_vec();
                match applied.first().map(|s| self.mapped(s, &rest)) {
                    Some(Mapped::Past { consumed }) if consumed > 0 && from + consumed <= k => {
                        value = applied[0].ret.map(|r| self.types.subst(r, &applied[0].subst));
                        from += consumed;
                    }
                    _ => break,
                }
            }
            shown = applied;
            lists = lists.split_off(from);
            arg_types = arg_types.split_off(from);
            chosen = None;
        }
        if shown.is_empty() {
            return Json::Null;
        }
        let mut fits: Vec<(Option<u32>, bool, bool)> = shown.iter().map(|s| self.fit(s, call, &lists, &arg_types, facts.current)).collect();
        // A value's member `apply`s none of which takes the written arguments by their number and
        // names: the extension `apply`s that take the value join them (`c("s", |true)` beside a
        // member `apply(i: Int)`), as the selection tries them where the member does not apply.
        let value = shown.first().and_then(|s| s.of_value);
        if value.is_some() && shown.iter().all(|s| s.of_value == value) && !fits.iter().any(|f| f.1) {
            let more = self.extension_signatures(value.unwrap(), crate::names::APPLY);
            for s in more {
                fits.push(self.fit(&s, call, &lists, &arg_types, facts.current));
                shown.push(s);
            }
        }
        let mut signatures = Vec::new();
        let mut actives = Vec::new();
        let mut fitting: Vec<bool> = Vec::new();
        let mut shaped: Vec<bool> = Vec::new();
        for (s, &(active, shape, types)) in shown.iter().zip(&fits) {
            let (label, ranges) = self.label(s);
            let parameters: Vec<Json> = ranges.iter().map(|&(a, b)| obj([("label", Json::Arr(vec![a.into(), b.into()]))])).collect();
            let count = ranges.len() as u32;
            // A parameter out of the label's range marks none active: an empty clause's, an
            // argument past every parameter's, a list the signature does not take.
            let active = active.unwrap_or(count);
            signatures.push(obj([("label", label.into()), ("parameters", Json::Arr(parameters)), ("activeParameter", active.into())]));
            actives.push(active);
            fitting.push(shape && types);
            shaped.push(shape);
        }
        // The build's choice took the arguments as written, conversions included: it stands where
        // their number and names fit it, whatever their types are before conversion.
        let recorded = chosen.and_then(|c| shown.iter().position(|s| s.sym == Some(c))).filter(|&i| shaped[i]);
        let active = recorded.or_else(|| fitting.iter().position(|&f| f)).unwrap_or(0);
        obj([("signatures", Json::Arr(signatures)), ("activeSignature", (active as u32).into()), ("activeParameter", actives[active].into())])
    }

    /// The active parameter of `s` and whether the written lists fit it by their shape and by
    /// their types (`active_parameter`), none where the cursor's list is not one of its clauses.
    fn fit(&mut self, s: &Shown, call: &Call, lists: &[&List], arg_types: &[Vec<Option<TypeId>>], current: Option<TypeId>) -> (Option<u32>, bool, bool) {
        match self.mapped(s, lists) {
            Mapped::At { clause } => self.active_parameter(s, clause, call, lists, arg_types, current),
            _ => (None, false, false),
        }
    }

    /// Where the written lists put the cursor's in `s`: each plain list takes the next plain
    /// clause, a `using` list the next `using` clause, a `using` clause no list is written for
    /// is inferred and passed over.
    fn mapped(&self, s: &Shown, lists: &[&List]) -> Mapped {
        let mut clause = 0;
        for (j, l) in lists.iter().enumerate() {
            let cursor = j + 1 == lists.len();
            loop {
                if clause == s.clauses.len() {
                    return Mapped::Past { consumed: j };
                }
                match takes(&s.clauses[clause], l, cursor) {
                    Some(true) => break,
                    Some(false) => clause += 1,
                    None => return Mapped::Wrong,
                }
            }
            if cursor {
                return Mapped::At { clause };
            }
            clause += 1;
        }
        Mapped::Wrong
    }

    /// The flat index of the parameter the cursor's argument goes to in `s`, its list in the
    /// clause `clause`, whether the written arguments' number and names fit `s`, and whether the
    /// types known of them do, against the clauses they go to.
    #[allow(clippy::too_many_arguments)]
    fn active_parameter(&mut self, s: &Shown, clause: usize, call: &Call, lists: &[&List], arg_types: &[Vec<Option<TypeId>>], current: Option<TypeId>) -> (Option<u32>, bool, bool) {
        let mut fits = true;
        let mut typed = true;
        // The lists before the cursor's, against their clauses.
        let mut c = 0;
        for (j, l) in lists.iter().enumerate() {
            let complete = j + 1 < lists.len();
            while c < s.clauses.len() && takes(&s.clauses[c], l, !complete) == Some(false) {
                c += 1;
            }
            if c >= s.clauses.len() || takes(&s.clauses[c], l, !complete).is_none() {
                return (None, false, false);
            }
            let types = arg_types.get(j).map_or(&[][..], |v| v.as_slice());
            let (shape, conform) = self.list_fits(s, &s.clauses[c], &l.args, types, complete);
            fits &= shape;
            typed &= conform;
            c += 1;
        }
        let params = &s.clauses[clause].params;
        let within = match &call.named {
            Some(name) => params.iter().position(|p| p.name.is_some_and(|n| self.name_ref(n) == name)),
            None if call.arg < params.len() => Some(call.arg),
            None => params.last().filter(|p| p.repeated).map(|_| params.len() - 1),
        };
        if within.is_none() {
            fits = false;
        }
        // The argument the cursor stands before goes to the active parameter.
        if let (Some(i), Some(cur)) = (within, call.current.as_ref()) {
            let declared = self.types.subst(params[i].ty, &s.subst);
            let open = !s.open.is_empty() && self.mentions_tparam_of(declared, Some(&s.open));
            let known = current.or_else(|| self.fact_type(&cur.fact, Span::new(cur.span.start, cur.span.start)));
            if let (false, false, Some(t)) = (open, s.pattern, known) {
                let t = self.zonk(t);
                if t != ERROR && !self.types.contains_error(t) && !self.admits(t, declared) {
                    typed = false;
                }
            }
        }
        let before: usize = s.clauses[..clause].iter().map(|c| c.params.len()).sum();
        (within.map(|i| (before + i) as u32), fits, typed)
    }

    /// Whether the arguments `args` fit `clause` by their number and names (no more than its
    /// parameters but for a repeated one, each name one of its parameters; a list complete, not
    /// the cursor's, also supplying every parameter without a default), and whether the types
    /// known of them (`types`, a literal's, a name's value) conform, a function literal's arity
    /// included.
    fn list_fits(&mut self, s: &Shown, clause: &Clause, args: &[Arg], types: &[Option<TypeId>], complete: bool) -> (bool, bool) {
        let params = &clause.params;
        let repeated = params.last().is_some_and(|p| p.repeated);
        if !repeated && args.len() > params.len() {
            return (false, false);
        }
        let mut typed = true;
        let mut filled = vec![false; params.len()];
        let mut pos = 0;
        for (i, a) in args.iter().enumerate() {
            let k = match &a.name {
                Some(name) => match params.iter().position(|p| p.name.is_some_and(|n| self.name_ref(n) == name)) {
                    Some(k) => k,
                    None => return (false, false),
                },
                None if pos < params.len() => pos,
                None if repeated => params.len() - 1,
                None => return (false, false),
            };
            pos = pos.max(k + 1);
            filled[k] = true;
            let param = &params[k];
            let declared = self.types.subst(param.ty, &s.subst);
            // A type a method's own parameter leaves open is no fact against it.
            if !s.open.is_empty() && self.mentions_tparam_of(declared, Some(&s.open)) {
                continue;
            }
            let known = types.get(i).copied().flatten().or_else(|| self.fact_type(&a.fact, Span::new(a.span.start, a.span.start)));
            if let Some(t) = known {
                let t = self.zonk(t);
                if t != ERROR && !self.types.contains_error(t) && !s.pattern && !self.admits(t, declared) {
                    typed = false;
                }
            }
            if let Fact::Lambda(n) = a.fact {
                if let Some((ps, _)) = self.as_function(declared) {
                    if ps.len() != n && n != 1 {
                        typed = false;
                    }
                }
            }
        }
        (!complete || params.iter().zip(&filled).all(|(p, &f)| f || p.default || p.repeated), typed)
    }

    /// The type a literal argument has.
    fn fact_type(&mut self, fact: &Fact, at: Span) -> Option<TypeId> {
        Some(match fact {
            Fact::Int => self.b.t_int,
            Fact::Long => self.b.t_long,
            Fact::Double => self.b.t_double,
            Fact::Float => self.b.t_float,
            Fact::Boolean => self.b.t_boolean,
            Fact::Char => self.b.t_char,
            Fact::String => self.b.t_string,
            Fact::Null => self.b.t_null,
            Fact::Name(name) => {
                // A value's declared type, seen from the class it is a member of.
                let n = self.interner.lookup(name)?;
                let r = self.resolved_term(n, at)?;
                let s = r.sym()?;
                if !matches!(self.syms.sym(s).kind, SymKind::Val | SymKind::Var | SymKind::Param) {
                    return None;
                }
                let ty = self.sig_of(s).ret;
                match r {
                    TermRef::This(c, _) => {
                        let this = self.syms.this_type(c);
                        let subst = self.owner_subst(this);
                        self.types.subst(ty, &subst)
                    }
                    _ => ty,
                }
            }
            Fact::This => {
                let c = self.env.frames.iter().rev().find_map(|f| match f {
                    super::Frame::Class(c) => Some(*c),
                    _ => None,
                })?;
                self.syms.this_type(c)
            }
            Fact::Lambda(_) | Fact::Other => return None,
        })
    }

    /// The label of `s` and each parameter's range in it, in UTF-16 units, start included.
    fn label(&mut self, s: &Shown) -> (String, Vec<(u32, u32)>) {
        let mut out = Label { text: String::new(), units: 0 };
        out.push(&s.name);
        if !s.tparams.is_empty() {
            out.push("[");
            for (i, &tp) in s.tparams.iter().enumerate() {
                if i > 0 {
                    out.push(", ");
                }
                let name = super::complete::spelled(&self.name_str(self.syms.tparam(tp).name));
                out.push(&name);
                let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
                if lower != NOTHING {
                    let t = self.types.subst(lower, &s.subst);
                    let t = self.show(t);
                    out.push(&format!(" >: {}", t));
                }
                if upper != ANY {
                    let t = self.types.subst(upper, &s.subst);
                    let t = self.show(t);
                    out.push(&format!(" <: {}", t));
                }
            }
            out.push("]");
        }
        let mut ranges = Vec::new();
        for c in &s.clauses {
            out.push("(");
            if c.implicit {
                out.push("implicit ");
            } else if c.using {
                out.push("using ");
            }
            for (i, p) in c.params.iter().enumerate() {
                if i > 0 {
                    out.push(", ");
                }
                let start = out.units;
                if let Some(n) = p.name.filter(|_| !s.pattern) {
                    let name = super::complete::spelled(&self.name_str(n));
                    out.push(&name);
                    out.push(": ");
                }
                if p.by_name {
                    out.push("=> ");
                }
                let ty = self.types.subst(p.ty, &s.subst);
                let ty = self.show(ty);
                out.push(&ty);
                if p.repeated {
                    out.push("*");
                }
                if p.default {
                    out.push(" = ...");
                }
                ranges.push((start, out.units));
            }
            out.push(")");
        }
        if let Some(ret) = s.ret {
            let ret = self.types.subst(ret, &s.subst);
            let ret = self.show(ret);
            out.push(": ");
            out.push(&ret);
        }
        (out.text, ranges)
    }
}

/// A label as it is printed, with its length in UTF-16 units.
struct Label {
    text: String,
    units: u32,
}

impl Label {
    fn push(&mut self, s: &str) {
        self.text.push_str(s);
        self.units += s.encode_utf16().count() as u32;
    }
}

/// Whether a clause takes the written list `l` (`Some(true)`), is passed over by it (`Some(false)`)
/// or is one it cannot go to (`None`): a plain clause takes a plain list; a `using` clause a
/// `using` list, and a plain one passes it over; a Scala 2 `implicit` clause takes a `using` list,
/// and a plain one too where it is written (the cursor's, or one with arguments, as the typer takes
/// it), which scalac accepts with a warning.
fn takes(c: &Clause, l: &List, cursor: bool) -> Option<bool> {
    match (c.using, c.implicit, l.using) {
        (false, _, false) => Some(true),
        (false, _, true) => None,
        (true, _, true) => Some(true),
        (true, true, false) => Some(cursor || !l.args.is_empty()),
        (true, false, false) => Some(false),
    }
}

fn clause_of(c: &ClauseSig) -> Clause {
    Clause { using: c.is_using, implicit: c.is_implicit, params: c.params.iter().map(|p| Param { name: Some(p.name), ty: p.ty, by_name: p.by_name, repeated: p.repeated, default: p.has_default }).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Option<Call> {
        let offset = text.find('|').unwrap();
        let text = text.replacen('|', "", 1);
        call_at(&text, offset as u32)
    }

    fn head(text: &str) -> Option<(Head, usize, usize)> {
        at(text).map(|c| (c.head, c.lists.len() - 1, c.arg))
    }

    #[test]
    fn calls() {
        let name = |n: &str| Head::Name(n.to_string());
        assert_eq!(head("object A { f(|) }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(1, |) }"), Some((name("f"), 0, 1)));
        assert_eq!(head("object A { f(1, |"), Some((name("f"), 0, 1)));
        assert_eq!(head("object A { f(1)(|) }"), Some((name("f"), 1, 0)));
        assert_eq!(head("object A { f(g(|)) }"), Some((name("g"), 0, 0)));
        assert_eq!(head("object A { f(g(1), |) }"), Some((name("f"), 0, 1)));
        assert_eq!(head("object A { f(\"a,|b\") }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(x => g(|)) }"), Some((name("g"), 0, 0)));
        assert_eq!(head("object A { f(x => |) }"), None);
        assert_eq!(head("object A { f { |} }"), None);
        assert_eq!(head("object A { f((1, |)) }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(a, { g; | }) }"), None);
        assert_eq!(head("object A { f(1)| }"), None);
        assert_eq!(head("object A { f[Int](|) }"), Some((name("f"), 0, 0)));
        assert!(at("object A { f[Int](|) }").unwrap().targs);
        assert_eq!(head("object A { f[|] }"), None);
        let c = at("object A { f(a = 1, b = |) }").unwrap();
        assert_eq!((c.arg, c.named.as_deref()), (1, Some("b")));
        assert_eq!(c.lists[0].args.len(), 1);
        assert_eq!(c.lists[0].args[0].name.as_deref(), Some("a"));
        let c = at("object A { x.m(1, |) }").unwrap();
        assert!(matches!(c.head, Head::Member { ref name, .. } if name == "m"));
        assert_eq!(c.lists[0].args[0].fact, Fact::Int);
        let c = at("object A { new C(|) }").unwrap();
        assert_eq!(c.head, Head::New { path: vec!["C".into()], anonymous: false });
        let c = at("object A { factory()(|) }").unwrap();
        assert_eq!((c.lists.len(), c.lists[1].callee), (2, Span::new(11, 20)));
        let c = at("object A { 1 match { case E(a, |) => } }").unwrap();
        assert_eq!((c.head, c.arg), (Head::Pattern { path: vec!["E".into()] }, 1));
        let c = at("object A { f(1)(using |) }").unwrap();
        assert!(c.lists[1].using);
        // The argument the cursor stands before.
        let c = at("object A { f(1, |\"s\") }").unwrap();
        assert_eq!(c.current.map(|a| (a.span, a.fact)), Some((Span::new(16, 19), Fact::String)));
        assert_eq!(at("object A { f(1, |) }").unwrap().current, None);
        assert!(matches!(head("object A { a op (1, |) }"), Some((Head::Infix { left: Fact::Name(_), .. }, 0, 1))));
        assert_eq!(head("object A { s\"${f(|)}\" }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(s\"a|\") }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(/* , | */) }"), Some((name("f"), 0, 0)));
        // `using` before a named argument, a comment before the list, a callee in parentheses, a
        // `new`'s second list.
        let c = at("object A { f(using b = |1) }").unwrap();
        assert_eq!(c.named.as_deref(), Some("b"));
        assert_eq!(head("object A { f(g /* ( */ (|1)) }"), Some((name("g"), 0, 0)));
        let c = at("object A { (g)(|1) }").unwrap();
        assert_eq!((c.head, c.head_span), (name("g"), Span::new(12, 13)));
        assert_eq!(at("object A { (g())(|1) }").unwrap().lists[1].callee, Span::new(12, 15));
        let c = at("object A { new C(1)(|2) }").unwrap();
        assert_eq!((c.head, c.lists.len()), (Head::New { path: vec!["C".into()], anonymous: false }, 2));
        let c = at("object A { new C(1)(|2) {} }").unwrap();
        assert_eq!((c.head, c.lists.len()), (Head::New { path: vec!["C".into()], anonymous: true }, 2));
        assert!(matches!(head("object A { f(c op (|1)) }"), Some((Head::Infix { .. }, 0, 0))));
        assert!(matches!(head("object A { f(c +: (|1)) }"), Some((Head::Name(_), 0, 0))));
        // No marker before a written argument, which keeps its fact; a function type's arrow is
        // no literal's, a literal's body no call's.
        let c = at("object A { f(|-1) }").unwrap();
        assert_eq!(c.current.map(|a| (a.span, a.fact)), Some((Span::new(13, 15), Fact::Int)));
        assert_eq!(head("object A { f(x: Int => Int|, 2) }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(i => i|, 2) }"), None);
        assert_eq!(head("object A { f(i => |i, 2) }"), None);
        assert_eq!(head("object A { f(i => g(|), 2) }"), Some((name("g"), 0, 0)));
        // The tree's test without the marker too, a polymorphic literal's body, a body after a
        // line break.
        assert_eq!(head("object A { f(x: Int => |Int, 2) }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f(x: Int => I|nt, 2) }"), Some((name("f"), 0, 0)));
        assert_eq!(head("object A { f([T] => |) }"), None);
        assert_eq!(head("object A { f(0)(|_ + _) }"), Some((name("f"), 1, 0)));
        assert_eq!(head("object A { f(0)(_ + |_) }"), None);
        assert_eq!(head("object A {\n  f(i =>\n    |i, 2)\n}"), None);
        // A trailing comma, which the lexer drops, before the cursor.
        assert_eq!(head("object A { f(1, |\n} "), Some((name("f"), 0, 1)));
        assert_eq!(head("object A {\n  f(\n    1,\n    |\n  )\n}"), Some((name("f"), 0, 1)));
        assert_eq!(head("object A {\n  f(\n    1, // a, b\n    |\n  )\n}"), Some((name("f"), 0, 1)));
    }
}
