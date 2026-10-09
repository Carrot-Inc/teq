//! What the text says at a completion's cursor (docs/TARGETS.md, "The language server"): the
//! fake identifier `teqCursorZz` is put at the offset of a copy of the request's text, which the
//! lexer and the parser alone read, so that the token at the cursor and the tree around it tell
//! what the position completes and which range of the text an item replaces. The copy is read
//! with an interner of its own and dropped: nothing of the session is read or written here, and
//! the semantics are the typed program's (`typer::complete`).

use crate::ast::{Ast, DefKind, Expr, ImportSel, Pat, TyExpr};
use crate::intern::{Interner, Name};
use crate::source::Span;
use crate::token::Tok;

pub const FAKE: &str = "teqCursorZz";

#[derive(Debug, Clone, PartialEq)]
pub enum Site {
    /// A name in an expression: what the scope binds.
    Scope,
    /// `recv.name`: the members of the receiver, whose node (its span in the request's text) has
    /// the receiver's type; `path` the receiver as a path of names where it is one, which a
    /// package or an object answers by.
    Member { recv: Span, path: Option<Vec<String>> },
    /// A type's name: the scope's types, or those of the package or object `path` (`a.b.Ty`).
    Type { path: Option<Vec<String>> },
    /// The class after `new`, likewise.
    New { path: Option<Vec<String>> },
    /// A segment or a selector of an import after `path`.
    Import { path: Vec<String> },
    /// What answers nothing: a binder, a declaration's name, a comment, a string.
    Nothing,
}

/// An argument a call of the copy's tree writes where the cursor's identifier stands: the
/// function's name, its receiver's span where it is selected (`r.f(..)`), the argument lists
/// written before its own in order (each whether a `using` one) and whether its own is one, its
/// position or the parameter it names (`f(x = |)`), and the type arguments written
/// (`f[Int => Int](|)`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArgOf {
    pub name: String,
    pub recv: Option<Span>,
    pub before: Vec<bool>,
    pub using: bool,
    pub index: usize,
    pub named: Option<String>,
    pub targs: Vec<TArg>,
}

/// A type argument as written: a function type, a path of names (`F`, `p.F`), anything else.
#[derive(Debug, Clone, PartialEq)]
pub enum TArg {
    Function,
    Path(Vec<String>),
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub site: Site,
    /// The name as written before the cursor, without a backquote.
    pub prefix: String,
    /// The whole name the cursor stands in, its backquotes included, and the part of it before
    /// the cursor: what an item replaces, and what it inserts over.
    pub replace: Span,
    pub insert: Span,
    /// Where no name is written after a dot (`o.|`, `o. |`, a comment between): the end of the
    /// dot, where the selection without a name stands.
    pub dot: Option<u32>,
    /// Where no name is written and the identifier stands for an argument of a call
    /// (`f(1, |)`, `xs.map(|)`), or for the whole right-hand side of a definition with a
    /// declared type (`val f: T = |`, the offset of the definition's name): what is expected
    /// there is read from, which no typing recorded where nothing was written.
    pub arg: Option<ArgOf>,
    pub rhs_of: Option<u32>,
    /// Whether the name is applied as written already: an argument list, a type argument list
    /// or a brace block follows it, or `_` (`f _`, its eta-expansion), or it stands as an
    /// operator between operands; or it is spliced bare into an interpolated string (`s"$f"`),
    /// where what follows it is the string's. An item there inserts its name alone.
    pub applied: bool,
}

impl Context {
    fn nothing(offset: u32) -> Context {
        Context { site: Site::Nothing, prefix: String::new(), replace: Span::new(offset, offset), insert: Span::new(offset, offset), dot: None, arg: None, rhs_of: None, applied: false }
    }
}

/// The site at `offset` of `text`, read from a copy with the fake identifier at the offset.
pub fn context(text: &str, offset: u32) -> Context {
    let offset = (offset as usize).min(text.len());
    let mut at = offset;
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let offset = at as u32;
    let fake = FAKE.len() as u32;
    let interner = Interner::new();
    // The name the cursor stands in or at, in every form a program writes it unquoted (letters,
    // an operator, letters with a symbolic suffix such as `foo_+`), the one the cursor follows
    // before the one it precedes: the identifier takes the name's place in the copy, since put
    // inside it the identifier could form a token of its own beside it (`a.+|+`, `a.foo_|+`), and
    // the name's whole range in the text is what an item replaces.
    let name = {
        let original = crate::lexer::lex(text, &interner);
        let plain = |t: &&crate::token::Token| (t.kind == Tok::Ident && text.as_bytes().get(t.span.start as usize) != Some(&b'`')) || t.kind == Tok::OpIdent;
        let holds = |t: &&crate::token::Token| t.span.start < offset && offset <= t.span.end;
        let precedes = |t: &&crate::token::Token| t.span.start == offset;
        let found = original.tokens.iter().filter(plain).find(holds).or_else(|| original.tokens.iter().filter(plain).find(precedes));
        found.map(|t| t.span)
    };
    let (marked, start, replace) = match name {
        Some(n) => (format!("{}{}{}", &text[..n.start as usize], FAKE, &text[n.end as usize..]), n.start, Some(n)),
        None => (format!("{}{}{}", &text[..at], FAKE, &text[at..]), offset, None),
    };
    let lexed = crate::lexer::lex(&marked, &interner);
    // The token the identifier stands in; none where the lexer skipped it (a comment).
    let Some(at_token) = lexed.tokens.iter().position(|t| t.span.start <= start && start + fake <= t.span.end) else {
        return Context::nothing(offset);
    };
    let token = &lexed.tokens[at_token];
    if token.kind != Tok::Ident {
        return Context::nothing(offset);
    }
    let backquoted = marked.as_bytes()[token.span.start as usize] == b'`';
    let replace = replace.unwrap_or(Span::new(token.span.start, token.span.end - fake));
    let insert = Span::new(token.span.start, offset);
    let written = &text[insert.start as usize..insert.end as usize];
    let prefix = if backquoted { written.strip_prefix('`').unwrap_or(written) } else { written }.to_string();
    let name = token.name;
    let (asts, _) = crate::frontend::parse_one(&marked, &interner, crate::parser::Syntax { index: true, ..Default::default() });
    let site = asts.iter().find_map(|ast| site_in(ast, name, &interner)).unwrap_or_else(|| {
        // The parser's recovery dropped the name: a reference, unless a selection was cut.
        let before = text[..insert.start as usize].trim_end();
        if before.ends_with('.') { Site::Nothing } else { Site::Scope }
    });
    // What follows the name at its nesting: the token after it (`new C(`, `f[`, `f {`, `f:` and
    // an indented block; `f _` and `(f) _`, whose `_` leaves no node), or the tree's application
    // of it, which a block on the next line can be.
    let next = lexed.tokens.get(at_token + 1).map(|t| t.kind);
    // Past the closing parentheses of the groups the name stands in alone, opened right before
    // it, not those of a list it is an argument of: a `(` after what ends an expression opens
    // an argument list. The `)` of an `if`'s or a `while`'s condition ends none.
    let tokens = &lexed.tokens;
    let condition_closed = |i: usize| {
        let mut depth = 0usize;
        for j in (0..=i).rev() {
            match tokens[j].kind {
                Tok::RParen => depth += 1,
                Tok::LParen => {
                    depth -= 1;
                    if depth == 0 {
                        return j > 0 && matches!(tokens[j - 1].kind, Tok::KwIf | Tok::KwWhile);
                    }
                }
                _ => {}
            }
        }
        false
    };
    let ends_expr = |i: usize| {
        let k = tokens[i].kind;
        matches!(
            k,
            Tok::Ident | Tok::RParen | Tok::RBracket | Tok::RBrace | Tok::IntLit | Tok::LongLit | Tok::DoubleLit | Tok::FloatLit | Tok::CharLit
                | Tok::StringLit | Tok::InterpEnd | Tok::KwThis | Tok::KwNull | Tok::KwTrue | Tok::KwFalse | Tok::Underscore
        ) && !(k == Tok::RParen && condition_closed(i))
    };
    // The groups open before the expression the name ends: before a path it is selected from.
    let mut path_start = at_token;
    while path_start >= 2 && lexed.tokens[path_start - 1].kind == Tok::Dot && matches!(lexed.tokens[path_start - 2].kind, Tok::Ident | Tok::KwThis | Tok::KwSuper) {
        path_start -= 2;
    }
    let before = &lexed.tokens[..path_start];
    let groups = (1..=before.len()).take_while(|&i| before[before.len() - i].kind == Tok::LParen && !(i < before.len() && ends_expr(before.len() - i - 1))).count();
    let closing = lexed.tokens[at_token + 1..].iter().take(groups).take_while(|t| t.kind == Tok::RParen).count();
    let eta = lexed.tokens.get(at_token + 1 + closing).is_some_and(|t| t.kind == Tok::Underscore);
    // A name no lexer starts after `$`, which is a letter of names, but an interpolation's
    // splice.
    let spliced = text[..replace.start as usize].ends_with('$');
    let applied = spliced || eta || matches!(next, Some(Tok::LParen | Tok::LBracket | Tok::LBrace | Tok::ColonEol)) || asts.iter().any(|ast| applied_in(ast, name));
    let dot = match lexed.tokens[..at_token].iter().rev().find(|t| !matches!(t.kind, Tok::Newline | Tok::Indent | Tok::Outdent)) {
        Some(t) if replace.start == replace.end && t.kind == Tok::Dot => Some(t.span.end),
        _ => None,
    };
    let empty = replace.start == replace.end;
    let arg = if empty { asts.iter().find_map(|ast| arg_in(ast, name, &interner)) } else { None };
    let rhs_of = if empty { asts.iter().find_map(|ast| rhs_in(ast, name)) } else { None };
    Context { site, prefix, replace, insert, dot, arg, rhs_of, applied }
}

/// The argument `fake` stands for in an application of `ast`, if it is one.
fn arg_in(ast: &Ast, fake: Name, interner: &Interner) -> Option<ArgOf> {
    let text = |n: Name| interner.get(n).to_string();
    for e in &ast.exprs {
        let (Expr::Apply(f, args) | Expr::UsingApply(f, args)) = *e else { continue };
        let using = matches!(*e, Expr::UsingApply(..));
        let found = ast.expr_list(args).iter().enumerate().find_map(|(i, &a)| match ast.expr(a) {
            Expr::Ident(n) if n == fake => Some((i, None)),
            Expr::NamedArg(p, v) if matches!(ast.expr(v), Expr::Ident(n) if n == fake) => Some((i, Some(text(p)))),
            _ => None,
        });
        let Some((index, named)) = found else { continue };
        let (mut head, mut before, mut targs) = (f, Vec::new(), Vec::new());
        loop {
            match ast.expr(head) {
                Expr::Apply(g, _) | Expr::UsingApply(g, _) => {
                    before.push(matches!(ast.expr(head), Expr::UsingApply(..)));
                    head = g;
                }
                Expr::TypeApply(g, l) => {
                    targs = ast
                        .ty_list(l)
                        .iter()
                        .map(|&t| match ast.ty(t) {
                            TyExpr::Fun(..) | TyExpr::CtxFun(..) => TArg::Function,
                            _ => ty_path(ast, t, interner).map_or(TArg::Other, TArg::Path),
                        })
                        .collect();
                    head = g;
                }
                _ => break,
            }
        }
        before.reverse();
        let (name, recv) = match ast.expr(head) {
            Expr::Ident(n) => (text(n), None),
            Expr::Select(q, n) => {
                let mut r = q;
                while let Expr::Parens(inner) = ast.expr(r) {
                    r = inner;
                }
                (text(n), Some(ast.expr_span(r)))
            }
            _ => return None,
        };
        return Some(ArgOf { name, recv, before, using, index, named, targs });
    }
    None
}

/// The offset of the name of the definition with a declared type whose whole right-hand side
/// `fake` is, if one is.
fn rhs_in(ast: &Ast, fake: Name) -> Option<u32> {
    let is_fake = |e: crate::ast::ExprId| matches!(ast.expr(e), Expr::Ident(n) if n == fake);
    ast.defs.iter().find_map(|d| match &d.kind {
        DefKind::Val { ty: Some(_), rhs: Some(r), .. } if is_fake(*r) => Some(d.span.start),
        DefKind::Fun(f) if f.ret.is_some() && f.body.is_some_and(is_fake) => Some(d.span.start),
        _ => None,
    })
}

/// Whether the node that names `fake` in `ast` is applied: the function of an application or of
/// a type application, or an operator with its operands.
fn applied_in(ast: &Ast, fake: Name) -> bool {
    let names = |f: crate::ast::ExprId| matches!(ast.expr(f), Expr::Ident(n) | Expr::Select(_, n) if n == fake);
    ast.exprs.iter().any(|e| match *e {
        Expr::Apply(f, _) | Expr::UsingApply(f, _) | Expr::TypeApply(f, _) => names(f),
        _ => expr_names(e, fake),
    })
}

/// The site of the node that names `fake` in `ast`, if one does.
fn site_in(ast: &Ast, fake: Name, interner: &Interner) -> Option<Site> {
    let path = |names: &[Name]| -> Vec<String> { names.iter().map(|&n| interner.get(n).to_string()).collect() };
    for imp in ast.imports.iter().chain(&ast.local_imports).chain(&ast.exports).chain(&ast.top_exports).chain(&ast.language_imports) {
        if let Some(i) = imp.path.iter().position(|&n| n == fake) {
            return Some(Site::Import { path: path(&imp.path[..i]) });
        }
        match imp.sel {
            ImportSel::Name(n, _) if n == fake => return Some(Site::Import { path: path(&imp.path) }),
            ImportSel::Name(_, Some(alias)) if alias == fake => return Some(Site::Nothing),
            _ => {}
        }
    }
    for e in &ast.exprs {
        match *e {
            Expr::Ident(n) if n == fake => return Some(Site::Scope),
            Expr::Select(q, n) if n == fake => {
                // The receiver as the typer types it: inside its parentheses.
                let mut r = q;
                while let Expr::Parens(inner) = ast.expr(r) {
                    r = inner;
                }
                return Some(Site::Member { recv: ast.expr_span(r), path: expr_path(ast, q, interner) });
            }
            Expr::NamedArg(n, _) if n == fake => return Some(Site::Nothing),
            Expr::Infix(..) | Expr::Prefix(..) if expr_names(e, fake) => return Some(Site::Scope),
            _ => {}
        }
    }
    for (i, t) in ast.tys.iter().enumerate() {
        let (named, path) = match *t {
            TyExpr::Name(n) if n == fake => (true, None),
            TyExpr::Select(q, n) if n == fake => (true, Some(ty_path(ast, q, interner)?)),
            _ => (false, None),
        };
        if !named {
            continue;
        }
        let id = crate::ast::TyExprId(i as u32);
        return Some(if after_new(ast, id) { Site::New { path } } else { Site::Type { path } });
    }
    for p in &ast.pats {
        if let Pat::Bind(n, _) | Pat::NamedField(n, _) = *p {
            if n == fake {
                return Some(Site::Nothing);
            }
        }
    }
    if ast.lambda_params.iter().any(|p| p.name == fake) || ast.name_lists.contains(&fake) {
        return Some(Site::Nothing);
    }
    for d in &ast.defs {
        if d.name == fake {
            return Some(Site::Nothing);
        }
        let params = match &d.kind {
            DefKind::Fun(f) => Some((&f.tparams, &f.clauses)),
            DefKind::Class(c) => Some((&c.tparams, &c.clauses)),
            DefKind::Given(g) => Some((&g.tparams, &g.clauses)),
            DefKind::TypeAlias { tparams, .. } => {
                if tparams.iter().any(|p| p.name == fake) {
                    return Some(Site::Nothing);
                }
                None
            }
            DefKind::Val { .. } => None,
        };
        if let Some((tparams, clauses)) = params {
            if tparams.iter().any(|p| p.name == fake) || clauses.iter().any(|c| c.params.iter().any(|p| p.name == fake)) {
                return Some(Site::Nothing);
            }
        }
    }
    None
}

/// Whether an operator expression's own name is the fake one (`a fo|` read as a postfix).
fn expr_names(e: &Expr, fake: Name) -> bool {
    matches!(*e, Expr::Infix(_, n, _) | Expr::Prefix(n, _) if n == fake)
}

/// The receiver as a path of names, `a.b.c`, where it is one; `super` as itself.
fn expr_path(ast: &Ast, e: crate::ast::ExprId, interner: &Interner) -> Option<Vec<String>> {
    match ast.expr(e) {
        Expr::Ident(n) => Some(vec![interner.get(n).to_string()]),
        // `super[P]` names the parent it selects in.
        Expr::Super(q) if q == crate::names::EMPTY => Some(vec!["super".to_string()]),
        Expr::Super(q) => Some(vec!["super".to_string(), interner.get(q).to_string()]),
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

/// Whether the type `t` is the class of a `new`: `new T(..)`, `new T[A]`, `new T { .. }`.
fn after_new(ast: &Ast, t: crate::ast::TyExprId) -> bool {
    let heads = |ty: crate::ast::TyExprId| ty == t || matches!(ast.ty(ty), TyExpr::Apply(f, _) if f == t);
    ast.exprs.iter().any(|e| match *e {
        Expr::New(ty, _) => heads(ty),
        Expr::NewAnon(d) => match &ast.def(d).kind {
            DefKind::Class(c) => c.parents.first().map_or(false, |p| heads(p.ty)),
            _ => false,
        },
        _ => false,
    })
}

/// The keywords a scope completion offers.
pub const KEYWORDS: &[&str] = &[
    "case", "catch", "class", "def", "do", "else", "enum", "extension", "false", "finally", "for", "given", "if", "implicit", "import", "lazy", "match", "new",
    "null", "object", "return", "super", "then", "this", "throw", "trait", "true", "try", "type", "val", "var", "while", "yield",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Context {
        let offset = text.find('|').unwrap();
        let text = text.replacen('|', "", 1);
        context(&text, offset as u32)
    }

    #[test]
    fn sites() {
        assert_eq!(at("object A { val x = 1; def f = x.|\n}").site, Site::Member { recv: Span::new(30, 31), path: Some(vec!["x".into()]) });
        let c = at("object A { def f = fo|obar }");
        assert_eq!(c.site, Site::Scope);
        assert_eq!(c.prefix, "fo");
        assert_eq!((c.insert, c.replace), (Span::new(19, 21), Span::new(19, 25)));
        assert_eq!(at("object A { val x: Li| = ??? }").site, Site::Type { path: None });
        assert_eq!(at("object A { val x = new Li|() }").site, Site::New { path: None });
        assert_eq!(at("import scala.collection.Se|").site, Site::Import { path: vec!["scala".into(), "collection".into()] });
        assert_eq!(at("import scala.coll|").site, Site::Import { path: vec!["scala".into()] });
        assert_eq!(at("object A { // fo|\n }").site, Site::Nothing);
        assert_eq!(at("object A { val s = \"fo|\" }").site, Site::Nothing);
        assert_eq!(at("object A { def fo| = 1 }").site, Site::Nothing);
        assert_eq!(at("object A { 1 match { case fo| => 1 } }").site, Site::Nothing);
        let c = at("object A { val s = s\"$na|\" }");
        assert_eq!((c.site, c.prefix.as_str()), (Site::Scope, "na"));
        let c = at("object A { val x = `my v|al` }");
        assert_eq!((c.site.clone(), c.prefix.as_str()), (Site::Scope, "my v"));
        assert_eq!(c.replace, Span::new(19, 27));
        assert_eq!(at("object A { s\"$x.fo|\" }").site, Site::Nothing);
        // An operator's whole token, the cursor before it or in it, a member's.
        let c = at("object A { val r = a.|++(1) }");
        assert_eq!((c.site.clone(), c.prefix.as_str(), c.insert, c.replace), (Site::Member { recv: Span::new(19, 20), path: Some(vec!["a".into()]) }, "", Span::new(21, 21), Span::new(21, 23)));
        let c = at("object A { val r = a.foo_|+(1) }");
        assert_eq!((c.site.clone(), c.prefix.as_str(), c.insert, c.replace), (Site::Member { recv: Span::new(19, 20), path: Some(vec!["a".into()]) }, "foo_", Span::new(21, 25), Span::new(21, 26)));
        assert_eq!(at("object A { val r = a.foo_+| }").site, Site::Member { recv: Span::new(19, 20), path: Some(vec!["a".into()]) });
        let c = at("object A { val r = a.+|+(1) }");
        assert_eq!((c.site.clone(), c.prefix.as_str(), c.insert, c.replace), (Site::Member { recv: Span::new(19, 20), path: Some(vec!["a".into()]) }, "+", Span::new(21, 22), Span::new(21, 23)));
    }

    #[test]
    fn applied() {
        // An argument list, a type argument list, a brace block or a colon's block after the
        // name, or the name an operator between operands.
        for applied in ["in|(2)", "id|[Int](2)", "fo| { 1 }", "x.fo|:\n    1\n", "new Poi|(1, 2)", "new Poi|[Int]", "new Sha| { }", "a fo| b", "x.|own(1)", "in| _", "(in|) _", "((in|)) _", "(a.b.in|) _", "if (t()) (in|) _ else in", "s\"$in|\""] {
            assert!(at(&format!("object A {{ val r = {} }}", applied)).applied, "{}", applied);
        }
        for alone in ["fo|", "f(fo|)", "x.fo|.bar", "x.fo| + 1", "new Poi|", "x.|", "List(1).map(fo|)", "s\"${in|}\"", "outer(in|) _", "outer((in|)) _", "outer(a.in|) _"] {
            assert!(!at(&format!("object A {{ val r = {} }}", alone)).applied, "{}", alone);
        }
        // The dot before no name, through spaces and comments.
        assert_eq!(at("object A { val r = (x.|) }").dot, Some(22));
        assert_eq!(at("object A { val r = (x. /* c */ |) }").dot, Some(22));
        assert_eq!(at("object A { val r = (x. // c\n  |) }").dot, Some(22));
        assert_eq!(at("object A { val r = (x.fo|) }").dot, None);
        // What an empty name stands for.
        let arg = |name: &str, recv: Option<Span>, before: Vec<bool>, index| Some(ArgOf { name: name.into(), recv, before, using: false, index, named: None, targs: Vec::new() });
        assert_eq!(at("object A { val r = f(1, |) }").arg, arg("f", None, vec![], 1));
        assert_eq!(at("object A { val r = xs.map(|) }").arg, arg("map", Some(Span::new(19, 21)), vec![], 0));
        assert_eq!(at("object A { val r = xs.fold(0)(|) }").arg, arg("fold", Some(Span::new(19, 21)), vec![false], 0));
        assert_eq!(at("object A { val r = f(fo|) }").arg, None);
        let named = at("object A { val r = f(1)(using 2)(g = |) }").arg.unwrap();
        assert_eq!((named.before, named.using, named.index, named.named.as_deref()), (vec![false, true], false, 0, Some("g")));
        let using = at("object A { val r = f(1)(using |) }").arg.unwrap();
        assert_eq!((using.before, using.using, using.index), (vec![false], true, 0));
        let path = |p: &[&str]| TArg::Path(p.iter().map(|s| s.to_string()).collect());
        assert_eq!(at("object A { val r = id[Int => Int, F, a.F, List[Int]](|) }").arg.unwrap().targs, vec![TArg::Function, path(&["F"]), path(&["a", "F"]), TArg::Other]);
        assert_eq!(at("object A { val f: Int => Int = | }").rhs_of, Some(15));
        assert_eq!(at("object A { def f: Int => Int = | }").rhs_of, Some(15));
        assert_eq!(at("object A { val f = | }").rhs_of, None);
    }
}
