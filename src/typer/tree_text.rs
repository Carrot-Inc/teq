//! dotty's `RefinedPrinter` over untyped trees, as a message that interpolates one shows it
//! (`DeprecatedAssignmentSyntax`'s value): teq's expression printed as dotty prints the tree its parser
//! makes of the same source, laid out by `Texts.Text` at the page width (`-pagewidth`'s default, 80).
//!
//! The parentheses and braces the parser leaves no node of (`Ast::written_arg`) are dotty's `Parens` and
//! `Block(Nil, e)`; a number literal is its digits as the scanner reads them (`Number`); a placeholder's
//! parameter is named as dotty's unit-wide fresh name (`_$1`, `_$2`, in the order of the file's `_`s). A
//! definition is printed whole, as the parser and `Namer` leave its tree (a plain class parameter's
//! `private[this] val`, a given alias's `final lazy given val`); a structural given (`given T with ...`)
//! prints `given` and its name. A name is printed as written: dotty prints a typed `Ident` through its
//! prefix (`Y.k` of an object's member), which this printer does not know.

use crate::ast::{self, mods, Ast, CaseClause, DefKind, Enumerator, Expr, ExprId, Pat, PatId, Stmt, TyExpr, TyExprId};
use crate::intern::{Interner, Name};

/// dotty's `Texts.Text`, its elements in order (dotty keeps them reversed): a string, lines, and a
/// fluid run of elements the layout fills lines with, closed where it is no longer split.
#[derive(Clone, Debug)]
pub enum Text {
    Str(String),
    Vertical(Vec<Text>),
    Fluid(Vec<Text>),
    Closed(Vec<Text>),
}

const INDENT: i64 = 2;

impl Text {
    pub fn str(s: impl Into<String>) -> Text {
        Text::Str(s.into())
    }

    fn is_empty(&self) -> bool {
        match self {
            Text::Str(s) => s.is_empty(),
            Text::Fluid(e) | Text::Closed(e) => e.iter().all(|t| t.is_empty()),
            Text::Vertical(e) => e.is_empty(),
        }
    }

    fn is_vertical(&self) -> bool {
        matches!(self, Text::Vertical(_))
    }

    fn is_splittable(&self) -> bool {
        matches!(self, Text::Fluid(_))
    }

    pub fn close(self) -> Text {
        match self {
            Text::Fluid(e) => Text::Closed(e),
            t => t,
        }
    }

    /// dotty's `relems`, in order: a string is its own one.
    fn elems(self) -> Vec<Text> {
        match self {
            Text::Fluid(e) | Text::Closed(e) | Text::Vertical(e) => e,
            s => vec![s],
        }
    }

    fn remaining(&self, width: i64) -> i64 {
        match self {
            Text::Str(s) => width - s.chars().count() as i64,
            Text::Fluid(e) | Text::Closed(e) => {
                let mut w = width;
                for t in e.iter().rev() {
                    w = t.remaining(w);
                    if w < 0 {
                        return w;
                    }
                }
                w
            }
            Text::Vertical(_) => -1,
        }
    }

    fn last_line(&self) -> &str {
        match self {
            Text::Str(s) => s,
            Text::Fluid(e) | Text::Closed(e) | Text::Vertical(e) => e.last().map_or("", |t| t.last_line()),
        }
    }

    fn append_to_last_line(self, that: Text) -> Text {
        match that {
            Text::Str(s2) => match self {
                Text::Str(s1) => Text::Str(s1 + &s2),
                Text::Fluid(mut e) | Text::Closed(mut e) => {
                    match e.last_mut() {
                        Some(Text::Str(s1)) => s1.push_str(&s2),
                        _ => e.push(Text::Str(s2)),
                    }
                    Text::Fluid(e)
                }
                v => v,
            },
            Text::Fluid(e) | Text::Closed(e) => e.into_iter().fold(self, |acc, t| acc.append_to_last_line(t)),
            Text::Vertical(_) => self,
        }
    }

    fn append_indented(self, that: Text, width: i64) -> Text {
        let mut e = self.elems();
        e.push(that.layout(width - INDENT).indented());
        Text::Fluid(e)
    }

    fn append(self, width: i64, that: Text) -> Text {
        if self.is_empty() {
            that.layout(width)
        } else if that.is_empty() {
            self
        } else if that.is_vertical() {
            self.append_indented(that, width)
        } else if self.is_vertical() {
            let mut e = self.elems();
            e.push(that.layout(width));
            Text::Fluid(e)
        } else if that.remaining(width - self.last_line().chars().count() as i64) >= 0 {
            self.append_to_last_line(that)
        } else if that.is_splittable() {
            that.elems().into_iter().fold(self, |acc, t| acc.append(width, t))
        } else {
            self.append_indented(that, width)
        }
    }

    fn layout(self, width: i64) -> Text {
        match self {
            Text::Str(_) => self,
            Text::Fluid(e) | Text::Closed(e) => e.into_iter().fold(Text::str(""), |acc, t| acc.append(width, t)),
            Text::Vertical(e) => Text::Vertical(e.into_iter().map(|t| t.layout(width)).collect()),
        }
    }

    fn indented(self) -> Text {
        match self {
            Text::Str(s) => Text::Str(format!("  {}", s)),
            Text::Fluid(e) | Text::Closed(e) => Text::Fluid(e.into_iter().map(Text::indented).collect()),
            Text::Vertical(e) => Text::Vertical(e.into_iter().map(Text::indented).collect()),
        }
    }

    fn print(&self, out: &mut String) {
        match self {
            Text::Str(s) => out.push_str(s.trim_end_matches(' ')),
            Text::Fluid(e) | Text::Closed(e) | Text::Vertical(e) => {
                for (i, t) in e.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    t.print(out);
                }
            }
        }
    }

    pub fn mk_string(self, width: i64) -> String {
        let mut out = String::new();
        self.layout(width).print(&mut out);
        out
    }

    /// dotty's `~`.
    pub fn cat(self, that: impl Into<Text>) -> Text {
        let that = that.into();
        if self.is_empty() {
            return that;
        }
        if that.is_empty() {
            return self;
        }
        match self {
            Text::Fluid(mut e1) => match that {
                Text::Fluid(e2) => {
                    e1.extend(e2);
                    Text::Fluid(e1)
                }
                t => {
                    e1.push(t);
                    Text::Fluid(e1)
                }
            },
            s => match that {
                Text::Fluid(mut e2) => {
                    e2.insert(0, s);
                    Text::Fluid(e2)
                }
                t => Text::Fluid(vec![s, t]),
            },
        }
    }

    /// dotty's `~~`: the two with a blank between.
    pub fn cat_sp(self, that: Text) -> Text {
        if self.is_empty() {
            return that;
        }
        if that.is_empty() {
            return self;
        }
        Text::Fluid(vec![self, Text::str(" "), that])
    }

    /// `Text(xs, sep)`: lines for `"\n"`, else the non-empty ones with `sep` after each but the last.
    pub fn join(xs: Vec<Text>, sep: &str) -> Text {
        if sep == "\n" {
            return Text::Vertical(xs);
        }
        let mut ys: Vec<Text> = xs.into_iter().filter(|t| !t.is_empty()).collect();
        let Some(mut acc) = ys.pop() else { return Text::str("") };
        while let Some(a) = ys.pop() {
            acc = a.cat(sep).close().cat(acc);
        }
        acc
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Text {
        Text::str(s)
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        Text::Str(s)
    }
}

const DOT_PREC: i32 = 11;
const GLOBAL_PREC: i32 = 0;
const INFIX_PREC: i32 = 1;
const OR_PREC: i32 = 2;
const AND_PREC: i32 = 4;

/// `parsing.precedence`: a letter's 1, an assignment operator's 0, else by the first character.
fn precedence(op: &str) -> i32 {
    let Some(first) = op.chars().next() else { return -1 };
    if first.is_alphabetic() || first == '_' || first == '$' {
        return 1;
    }
    let assignment = op.len() > 1 && op.ends_with('=') && !op.starts_with('=') && !matches!(op, "<=" | ">=" | "!=") && is_operator_part(first);
    if assignment {
        return 0;
    }
    match first {
        '|' => 2,
        '^' => 3,
        '&' => 4,
        '=' | '!' => 5,
        '<' | '>' => 6,
        ':' => 7,
        '+' | '-' => 8,
        '*' | '/' | '%' => 9,
        _ => 10,
    }
}

/// `Chars.isOperatorPart`: an ASCII operator character, or past ASCII one that is no letter or digit.
fn is_operator_part(c: char) -> bool {
    if c.is_ascii() { crate::lexer::is_op_char(c as u8) } else { !c.is_alphanumeric() }
}

/// `Chars.escapedChar` into `out`.
fn escape_char(out: &mut String, c: char) {
    match c {
        '\u{8}' => out.push_str("\\b"),
        '\t' => out.push_str("\\t"),
        '\n' => out.push_str("\\n"),
        '\u{c}' => out.push_str("\\f"),
        '\r' => out.push_str("\\r"),
        '"' => out.push_str("\\\""),
        '\'' => out.push_str("\\'"),
        '\\' => out.push_str("\\\\"),
        c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
        c => out.push(c),
    }
}

/// `Chars.escapedString`.
fn escaped_string(s: &str, quoted: bool) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    if quoted {
        out.push('"');
    }
    for c in s.chars() {
        escape_char(&mut out, c);
    }
    if quoted {
        out.push('"');
    }
    out
}

/// The printer over one file's tree, at the precedence `prec` (`Printer.atPrec`).
pub struct TreePrinter<'p> {
    ast: &'p Ast,
    text: &'p str,
    interner: &'p Interner,
    prec: i32,
    /// The placeholders' parameters in scope, teq's name with dotty's.
    placeholders: Vec<(Name, String)>,
}

impl<'p> TreePrinter<'p> {
    pub fn new(ast: &'p Ast, text: &'p str, interner: &'p Interner) -> Self {
        TreePrinter { ast, text, interner, prec: GLOBAL_PREC, placeholders: Vec::new() }
    }

    /// `tree.show`: the expression laid out at the page width.
    pub fn show(mut self, e: ExprId) -> String {
        self.expr(e).mk_string(80)
    }

    fn name(&self, n: Name) -> String {
        self.interner.get(n).to_string()
    }

    fn at_prec(&mut self, p: i32, f: impl FnOnce(&mut Self) -> Text) -> Text {
        let outer = std::mem::replace(&mut self.prec, p);
        let t = f(self);
        self.prec = outer;
        t
    }

    /// `changePrec`: in parentheses where the context binds tighter.
    fn change_prec(&mut self, p: i32, f: impl FnOnce(&mut Self) -> Text) -> Text {
        if p < self.prec {
            self.at_prec(p, |s| Text::str("(").cat(f(s)).cat(")"))
        } else {
            self.at_prec(p, f)
        }
    }

    fn local(&mut self, e: ExprId) -> Text {
        self.at_prec(DOT_PREC, |s| s.expr(e))
    }

    fn global(&mut self, e: ExprId) -> Text {
        self.at_prec(GLOBAL_PREC, |s| s.expr(e))
    }

    fn exprs(&mut self, l: &[ExprId], sep: &str) -> Text {
        let ts: Vec<Text> = l.iter().map(|&e| self.expr(e)).collect();
        Text::join(ts, sep)
    }

    fn global_exprs(&mut self, l: &[ExprId], sep: &str) -> Text {
        self.at_prec(GLOBAL_PREC, |s| s.exprs(l, sep))
    }

    /// `blockText`: the trees in braces, one a line.
    fn block_text(&mut self, lines: Vec<Text>) -> Text {
        Text::str("{").cat(Text::join(lines, "\n")).cat("}").close()
    }

    /// The parentheses and braces written around `e` that no node keeps, outermost first, past the blanks and
    /// comments between them.
    fn wrappers(&self, e: ExprId) -> Vec<u8> {
        let Some((written, _)) = self.ast.written_arg(e) else { return Vec::new() };
        let inner = (self.ast.expr_span(e).start as usize).min(self.text.len());
        let bytes = self.text.as_bytes();
        let mut out = Vec::new();
        let mut i = written.start as usize;
        while i < inner {
            match bytes[i] {
                b @ (b'(' | b'{') => out.push(b),
                b' ' | b'\t' | b'\n' | b'\r' => {}
                b'/' if bytes.get(i + 1) == Some(&b'/') => {
                    while i < inner && bytes[i] != b'\n' {
                        i += 1;
                    }
                }
                // A block comment, which nests.
                b'/' if bytes.get(i + 1) == Some(&b'*') => {
                    let mut depth = 0;
                    while i < inner {
                        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                            depth += 1;
                            i += 1;
                        } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                            depth -= 1;
                            i += 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        i += 1;
                    }
                }
                _ => break,
            }
            i += 1;
        }
        out
    }

    /// `toText(tree)`: the tree's text, closed but a block's.
    fn expr(&mut self, e: ExprId) -> Text {
        let wraps = self.wrappers(e);
        self.wrapped(e, &wraps)
    }

    fn wrapped(&mut self, e: ExprId, wraps: &[u8]) -> Text {
        match wraps.split_first() {
            None => {
                let t = self.core(e);
                if matches!(self.ast.expr(e), Expr::Block(_)) { t } else { t.close() }
            }
            Some((b'(', rest)) => Text::str("(").cat(self.at_prec(GLOBAL_PREC, |s| s.wrapped(e, rest))).cat(")").close(),
            Some((_, rest)) => {
                let inner = self.wrapped(e, rest);
                self.block_text(vec![inner])
            }
        }
    }

    /// A number literal's digits as dotty's scanner keeps them: without `_`, a hexadecimal one's
    /// without its `0x`, a negated one's sign joined, its suffix (`L`, `f`, `d`) kept (`Number`).
    fn digits(&self, e: ExprId) -> String {
        let span = self.ast.expr_span(e);
        let suffixed = matches!(self.text.as_bytes().get(span.end as usize), Some(b'l' | b'L' | b'f' | b'F' | b'd' | b'D'));
        let end = span.end as usize + suffixed as usize;
        let written = self.text.get(span.start as usize..end).unwrap_or("");
        let (neg, rest) = match written.strip_prefix('-') {
            Some(r) => (true, r.trim_start()),
            None => (false, written),
        };
        let rest = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")).unwrap_or(rest);
        let mut out = String::new();
        if neg {
            out.push('-');
        }
        out.extend(rest.chars().filter(|&c| c != '_'));
        out
    }

    fn core(&mut self, e: ExprId) -> Text {
        let ast = self.ast;
        match ast.expr(e) {
            Expr::IntLit(_) | Expr::LongLit(_) | Expr::DoubleLit(_) | Expr::DecimalLit(_) | Expr::FloatLit(_) => self.digits(e).into(),
            Expr::BoolLit(b) => (if b { "true" } else { "false" }).into(),
            Expr::CharLit(c) => {
                let mut out = String::from("'");
                if let Some(c) = char::from_u32(c) {
                    escape_char(&mut out, c);
                }
                out.push('\'');
                out.into()
            }
            Expr::StringLit(s) => escaped_string(ast.str(s), true).into(),
            Expr::UnitLit => "()".into(),
            Expr::NullLit => "null".into(),
            Expr::Ident(n) => {
                if let Some((_, shown)) = self.placeholders.iter().rev().find(|(p, _)| *p == n) {
                    return shown.clone().into();
                }
                let span = ast.expr_span(e);
                if self.text.as_bytes().get(span.start as usize) == Some(&b'`') {
                    return format!("`{}`", self.name(n)).into();
                }
                self.name(n).into()
            }
            Expr::This => "this".into(),
            Expr::ThisOf(t) => self.ty(t).cat(".this"),
            Expr::QualThis(q) => Text::str(self.name(q)).cat(".this"),
            Expr::Super(mix) => {
                if mix == crate::names::EMPTY {
                    "super".into()
                } else {
                    Text::str("super[").cat(self.name(mix)).cat("]")
                }
            }
            Expr::ClassOf(t) => {
                let ty = self.at_prec(GLOBAL_PREC, |s| s.ty(t));
                Text::str("classOf[").cat(ty).cat("]")
            }
            Expr::SymRef(_) | Expr::Unsupported(_) | Expr::Withheld(_) | Expr::Derived | Expr::Error | Expr::SplicePat(_) => "<error>".into(),
            Expr::Select(q, n) => self.local(q).cat(".").cat(self.name(n)),
            Expr::Apply(f, args) | Expr::UsingApply(f, args) => {
                let using = matches!(ast.expr(e), Expr::UsingApply(..));
                let fun = self.local(f);
                let args = self.global_exprs(ast.expr_list(args), ", ");
                fun.cat("(").cat(if using { "using " } else { "" }).cat(args).cat(")")
            }
            Expr::NamedArg(n, v) => Text::str(self.name(n)).cat(" = ").cat(self.expr(v)),
            Expr::TypeApply(f, targs) => {
                let fun = self.local(f);
                let args = self.global_types(ast.ty_list(targs));
                fun.cat("[").cat(args).cat("]")
            }
            Expr::Infix(l, op, r, _) => {
                let op = self.name(op);
                self.change_prec(precedence(&op), |s| s.expr(l).cat(" ").cat(op.as_str()).cat(" ").cat(s.expr(r)))
            }
            Expr::Prefix(op, r) => {
                let op = self.name(op);
                self.change_prec(DOT_PREC, |s| Text::str(op).cat(" ").cat(s.expr(r)))
            }
            Expr::Lambda(params, body) => self.lambda(params, body),
            Expr::PolyLambda(tparams, body) => {
                let names: Vec<Text> = ast.name_lists[tparams.range()].iter().map(|&n| Text::str(self.name(n))).collect();
                self.change_prec(GLOBAL_PREC, |s| Text::str("[").cat(Text::join(names, ", ")).cat("]").cat(" => ").cat(s.expr(body)))
            }
            Expr::If(c, t, els) | Expr::InlineIf(c, t, els) => {
                let kw = if matches!(ast.expr(e), Expr::InlineIf(..)) { "inline if " } else { "if " };
                let parens = self.wrappers(c).first() == Some(&b'(');
                self.change_prec(GLOBAL_PREC, |s| {
                    let head = Text::str(kw).cat(s.expr(c)).cat(if parens { "" } else { " then" });
                    let mut t = head.cat_sp(s.expr(t));
                    if let Some(els) = els {
                        t = t.cat(Text::str(" else ").cat(s.expr(els)));
                    }
                    t
                })
            }
            Expr::Match(sel, cases) | Expr::InlineMatch(sel, cases) => {
                let inline = matches!(ast.expr(e), Expr::InlineMatch(..));
                self.change_prec(GLOBAL_PREC, |s| {
                    let sel = s.expr(sel);
                    let head = if inline { Text::str("inline ").cat(sel) } else { sel };
                    let cases = s.cases(ast.case_list(cases));
                    head.cat(" match ").cat(cases)
                })
            }
            Expr::Block(stmts) => {
                let stmts = ast.stmt_list(stmts).to_vec();
                let mut lines: Vec<Text> = stmts.iter().flat_map(|&st| self.stmt_lines(st)).collect();
                // `Block(stats, EmptyTree)` where the last statement is no expression.
                if !matches!(stmts.last(), Some(Stmt::Expr(_))) {
                    lines.push("<empty>".into());
                }
                self.block_text(lines)
            }
            Expr::While(c, body) => self.change_prec(GLOBAL_PREC, |s| Text::str("while ").cat(s.expr(c)).cat(" do ").cat(s.expr(body))),
            Expr::For(enums, body, is_yield) => {
                let enums: Vec<Enumerator> = ast.enumerators[enums.range()].to_vec();
                self.change_prec(GLOBAL_PREC, |s| {
                    let texts: Vec<Text> = enums.iter().map(|en| s.enumerator(en)).collect();
                    Text::str("for ").cat(Text::join(texts, "; ")).cat(if is_yield { " yield " } else { " do " }).cat(s.expr(body))
                })
            }
            Expr::Assign(l, r) => self.change_prec(GLOBAL_PREC, |s| s.local(l).cat(" = ").cat(s.expr(r))),
            Expr::Tuple(items) => {
                let items = self.global_exprs(ast.expr_list(items), ", ");
                Text::str("(").cat(items).cat(")")
            }
            Expr::NamedTuple(names, values) => {
                let pairs: Vec<(Name, ExprId)> = ast.name_lists[names.range()].iter().copied().zip(ast.expr_list(values).iter().copied()).collect();
                let items = self.at_prec(GLOBAL_PREC, |s| {
                    let ts: Vec<Text> = pairs.iter().map(|&(n, v)| Text::str(s.name(n)).cat(" = ").cat(s.expr(v)).close()).collect();
                    Text::join(ts, ", ")
                });
                Text::str("(").cat(items).cat(")")
            }
            Expr::Parens(inner) => Text::str("(").cat(self.global(inner)).cat(")"),
            Expr::Typed(inner, t) => self.change_prec(DOT_PREC, |s| {
                let text = s.expr(inner);
                if matches!(ast.ty(t), TyExpr::Repeated(_)) {
                    return text.cat("*");
                }
                let colon = if text.last_line().chars().last().is_some_and(is_operator_part) { " :" } else { ":" };
                text.cat(colon).cat(s.ty(t))
            }),
            Expr::SeqLit(items, t) => {
                let items = self.global_exprs(ast.expr_list(items), ",");
                Text::str("[").cat(items).cat(" : ").cat(self.ty(t)).cat("]")
            }
            Expr::Unchecked(inner) => self.change_prec(DOT_PREC, |s| s.expr(inner).cat(": @unchecked")),
            Expr::New(t, args) => {
                let ty = self.at_prec(DOT_PREC, |s| s.ty(t));
                let args = self.global_exprs(ast.expr_list(args), ", ");
                Text::str("new ").cat(ty).close().cat("(").cat(args).cat(")")
            }
            Expr::NewAnon(d) => Text::str("new ").cat(self.template(d)),
            Expr::Interp(id, parts, holes) => self.interpolation(id, parts, ast.expr_list(holes)),
            Expr::Return(v) => self.change_prec(GLOBAL_PREC, |s| match v {
                Some(v) => Text::str("return").cat(Text::str(" ").cat(s.expr(v))),
                None => "return".into(),
            }),
            Expr::Throw(v) => self.change_prec(GLOBAL_PREC, |s| Text::str("throw ").cat(s.expr(v))),
            Expr::Try(i) => {
                let t = ast.try_expr(i);
                self.change_prec(GLOBAL_PREC, |s| {
                    let handler = if !t.cases.is_empty() {
                        s.cases(ast.case_list(t.cases))
                    } else {
                        match t.handler {
                            Some(h) => s.expr(h),
                            None => "<empty>".into(),
                        }
                    };
                    let mut text = Text::str("try ").cat(s.expr(t.body)).cat(" catch {").cat(handler).cat("}");
                    if let Some(f) = t.finalizer {
                        text = text.cat(Text::str(" finally ").cat(s.expr(f)));
                    }
                    text
                })
            }
            Expr::Quote(body) => Text::str("'{").cat(self.global(body)).cat("}"),
            Expr::QuoteType(t) => Text::str("'[").cat(self.ty(t)).cat("]"),
            Expr::Splice(body) => Text::str("${").cat(self.global(body)).cat("}"),
            Expr::AsFunction(q) => self.change_prec(INFIX_PREC, |s| s.expr(q).cat(" _")),
        }
    }

    /// `Function(args, body)`: one parameter without a type alone, else in parentheses; a `case`
    /// literal's the block of its cases (`Match(EmptyTree, cases)`). A placeholder's parameter takes
    /// dotty's name, its ordinal among the file's placeholders.
    fn lambda(&mut self, params: ast::ListRef, body: ExprId) -> Text {
        let ast = self.ast;
        let lps: Vec<ast::LambdaParam> = ast.lambda_params[params.range()].to_vec();
        if let [p] = lps.as_slice() {
            if p.cases {
                if let Expr::Match(_, cases) = ast.expr(body) {
                    return self.cases(ast.case_list(cases));
                }
            }
        }
        let scope = self.placeholders.len();
        let mut names = Vec::with_capacity(lps.len());
        for p in &lps {
            let text = self.name(p.name);
            let shown = if text.starts_with("_$") {
                let before = ast.lambda_params.iter().filter(|q| q.span.start < p.span.start && self.interner.get(q.name).starts_with("_$")).count();
                let shown = format!("_${}", before + 1);
                self.placeholders.push((p.name, shown.clone()));
                shown
            } else {
                text
            };
            names.push(shown);
        }
        let mut implicit_seen = false;
        let mut texts = Vec::with_capacity(lps.len());
        for (p, n) in lps.iter().zip(&names) {
            let mut t = Text::str("");
            if p.implicit && !implicit_seen {
                implicit_seen = true;
                t = t.cat("implicit ");
            }
            t = t.cat(n.as_str());
            if let Some(ty) = p.ty {
                t = t.cat(Text::str(": ").cat(self.ty(ty)));
            }
            texts.push(t);
        }
        let args = match lps.as_slice() {
            [p] if p.ty.is_none() => texts.pop().unwrap_or_else(|| Text::str("")),
            _ => Text::str("(").cat(Text::join(texts, ", ")).cat(")"),
        };
        let arrow = if lps.iter().any(|p| p.contextual) { "?=>" } else { "=>" };
        let body = self.expr(body);
        self.placeholders.truncate(scope);
        args.cat(" ").cat(arrow).cat(" ").cat(body)
    }

    fn cases(&mut self, cases: &[CaseClause]) -> Text {
        let lines: Vec<Text> = cases.iter().map(|c| self.case(c)).collect();
        self.block_text(lines)
    }

    /// `CaseDef`: its body the lines of the block the parser makes of it.
    fn case(&mut self, c: &CaseClause) -> Text {
        let pat = self.pat(c.pat);
        let mut t = Text::str("case ").cat(pat);
        if let Some(g) = c.guard {
            t = t.cat(Text::str(" if ").cat(self.expr(g)));
        }
        let ast = self.ast;
        let body = match ast.expr(c.body) {
            Expr::Block(stmts) => {
                let stmts = ast.stmt_list(stmts).to_vec();
                let mut lines: Vec<Text> = stmts.iter().flat_map(|&st| self.stmt_lines(st)).collect();
                if !matches!(stmts.last(), Some(Stmt::Expr(_))) {
                    lines.push("<empty>".into());
                }
                lines
            }
            // A block of one expression is the expression (`block`'s `last.isInstanceOf[Block]`).
            _ => {
                let wraps = self.wrappers(c.body);
                let wraps = if wraps.first() == Some(&b'{') { &wraps[1..] } else { &wraps[..] };
                vec![self.wrapped(c.body, wraps)]
            }
        };
        t.cat(" => ").cat(Text::join(body, "\n"))
    }

    /// A statement's lines: an import statement's one a clause (dotty's `Import` trees), none for a companion
    /// the parser made (`Ast::synthetic_companions`, which `Desugar` adds after the tree is parsed).
    fn stmt_lines(&mut self, st: Stmt) -> Vec<Text> {
        match st {
            Stmt::Def(d) if self.ast.synthetic_companions.binary_search(&d).is_ok() => Vec::new(),
            Stmt::Expr(e) => vec![self.expr(e)],
            Stmt::Def(d) => vec![self.def(d)],
            Stmt::Import(i) => self.import_lines(i),
        }
    }

    /// `importText`: `import`, the clause's path and its selectors, one alone, several in braces.
    fn import_lines(&mut self, i: u32) -> Vec<Text> {
        let ast = self.ast;
        let range = ast.import_stmts[i as usize];
        let sels = &ast.local_imports[range.start as usize..(range.start + range.len) as usize];
        let mut lines = Vec::new();
        let mut k = 0;
        while k < sels.len() {
            // The selectors of one clause share its start.
            let start = sels[k].span.start;
            let mut end = k + 1;
            while end < sels.len() && sels[end].span.start == start {
                end += 1;
            }
            let path: Vec<String> = sels[k].path.iter().map(|&n| self.name(n)).collect();
            let shown: Vec<Text> = sels[k..end]
                .iter()
                .map(|sel| match sel.sel {
                    ast::ImportSel::Wildcard => Text::str("*"),
                    ast::ImportSel::Given => match sel.bound {
                        Some(b) => Text::str("given ").cat(self.ty(b)),
                        None => Text::str("given"),
                    },
                    ast::ImportSel::Name(n, None) => Text::str(self.name(n)),
                    ast::ImportSel::Name(n, Some(r)) => Text::str(self.name(n)).cat(" as ").cat(self.name(r)),
                })
                .collect();
            let selectors = if shown.len() == 1 { shown.into_iter().next().unwrap_or_else(|| Text::str("")) } else { Text::str("{").cat(Text::join(shown, ", ")).cat("}") };
            lines.push(Text::str("import ").cat(path.join(".")).cat(".").cat(selectors));
            k = end;
        }
        lines
    }

    /// `toTextTemplate(impl, ofNew = true)` of an anonymous class: its parents (`constrText`, without
    /// their `new`), and its body, the `<empty>` of `templateStatSeq` where it has no statement.
    fn template(&mut self, d: ast::DefId) -> Text {
        let ast = self.ast;
        let DefKind::Class(cls) = &ast.def(d).kind else { return Text::str("{...}") };
        let parents: Vec<Text> = cls.parents.iter().map(|p| self.parent_text(p)).collect();
        let self_text = match (cls.self_alias, cls.self_type) {
            (n, None) if n == crate::names::EMPTY => Text::str(""),
            (n, ty) => {
                let name = if n == crate::names::EMPTY { "this".to_string() } else { self.name(n) };
                let mut t = Text::str(name);
                if let Some(ty) = ty {
                    t = t.cat(Text::str(": ").cat(self.ty(ty)));
                }
                t.cat(" =>").close()
            }
        };
        let mut body: Vec<Text> = cls.body.iter().flat_map(|&st| self.stmt_lines(st)).collect();
        if body.is_empty() {
            body.push("<empty>".into());
        }
        let body = self.at_prec(GLOBAL_PREC, |_| Text::join(body, "\n"));
        Text::join(parents, " with ").cat(Text::str(" {").cat_sp(self_text).cat(body).cat("}"))
    }

    /// `modText`'s flags, in the order dotty's printer gives them.
    fn mods_text(&self, m: ast::Mods) -> Text {
        let mut words = Vec::new();
        for (flag, word) in [
            (mods::PRIVATE, "private"),
            (mods::PROTECTED, "protected"),
            (mods::ABSTRACT, "abstract"),
            (mods::FINAL, "final"),
            (mods::SEALED, "sealed"),
            (mods::CASE, "case"),
            (mods::IMPLICIT, "implicit"),
            (mods::LAZY, "lazy"),
            (mods::GIVEN, "given"),
            (mods::OVERRIDE, "override"),
            (mods::INLINE, "inline"),
            (mods::TRANSPARENT, "transparent"),
            (mods::OPEN, "open"),
            (mods::OPAQUE, "opaque"),
            (mods::INFIX, "infix"),
        ] {
            if m & flag != 0 {
                words.push(Text::str(word));
            }
        }
        Text::join(words, " ")
    }

    /// `[A, +B <: U, C: Ord]`: a definition's type parameters, nothing for none.
    fn tparams_text(&mut self, tps: &[ast::TypeParam]) -> Text {
        if tps.is_empty() {
            return Text::str("");
        }
        let ts: Vec<Text> = tps
            .iter()
            .map(|p| {
                let variance = match p.variance {
                    1 => "+",
                    -1 => "-",
                    _ => "",
                };
                let mut t = Text::str(format!("{}{}", variance, self.name(p.name)));
                if let Some(lo) = p.lower {
                    t = t.cat(Text::str(" >: ").cat(self.ty(lo)));
                }
                if let Some(hi) = p.upper {
                    t = t.cat(Text::str(" <: ").cat(self.ty(hi)));
                }
                match p.context_bounds.as_slice() {
                    [] => {}
                    [b] => t = t.cat(Text::str(": ").cat(self.ty(*b))),
                    bs => {
                        let shown: Vec<Text> = bs.iter().map(|&b| self.ty(b)).collect();
                        t = t.cat(": {").cat(Text::join(shown, ", ")).cat("}");
                    }
                }
                t
            })
            .collect();
        Text::str("[").cat(Text::join(ts, ", ")).cat("]")
    }

    /// A parameter clause, `(using ...)` for a using one. A class's parameter shows the accessor dotty's
    /// `Namer` makes of it (`class_of`, whether the class is a case class): one written `val` or `var` as so,
    /// a plain one `private[this] val`, but a case class's, which is `val`.
    fn clause_text(&mut self, clause: &ast::ParamClause, class_of: Option<bool>) -> Text {
        let ps: Vec<Text> = clause
            .params
            .iter()
            .map(|p| {
                let mut head = Text::str("");
                if let Some(case) = class_of {
                    head = if p.mods & mods::FIELD != 0 {
                        let kw = if p.mods & mods::MUTABLE != 0 { "var" } else { "val" };
                        self.mods_text(p.mods & (mods::PRIVATE | mods::PROTECTED | mods::OVERRIDE | mods::FINAL)).cat_sp(Text::str(kw))
                    } else if case {
                        Text::str("val")
                    } else {
                        Text::str("private[this] val")
                    };
                } else if clause.is_implicit {
                    head = Text::str("implicit");
                }
                let mut t = head.cat_sp(Text::str(self.name(p.name))).cat(": ").cat(self.ty(p.ty));
                if let Some(dflt) = p.default {
                    t = t.cat(Text::str(" = ").cat(self.expr(dflt)));
                }
                t
            })
            .collect();
        Text::str("(").cat(if clause.is_using { "using " } else { "" }).cat(Text::join(ps, ", ")).cat(")")
    }

    fn def(&mut self, d: ast::DefId) -> Text {
        let ast = self.ast;
        let def = ast.def(d);
        let name = self.name(def.name);
        match &def.kind {
            DefKind::Val { pat, ty, rhs } => {
                let kw = if def.mods & mods::MUTABLE != 0 { "var" } else { "val" };
                let head = self.mods_text(def.mods).cat_sp(Text::str(kw));
                let mut t = match pat {
                    Some(p) => head.cat_sp(self.pat(*p)),
                    None => head.cat_sp(Text::str(name)),
                };
                if let Some(ty) = ty {
                    t = t.cat(Text::str(": ").cat(self.ty(*ty)));
                }
                if let Some(rhs) = rhs {
                    t = t.cat(Text::str(" = ").cat(self.expr(*rhs)));
                }
                t
            }
            DefKind::Fun(f) => {
                let mut t = self.mods_text(def.mods).cat_sp(Text::str("def")).cat_sp(Text::str(name));
                t = t.cat(self.tparams_text(&f.tparams));
                for clause in &f.clauses {
                    t = t.cat(self.clause_text(clause, None));
                }
                if let Some(ret) = f.ret {
                    t = t.cat(Text::str(": ").cat(self.ty(ret)));
                }
                if let Some(body) = f.body {
                    t = t.cat(Text::str(" = ").cat(self.expr(body)));
                }
                t
            }
            DefKind::Class(c) => {
                // An object is a `ModuleDef`, whose flags `modText` shows with `module`.
                let kw = match c.kind {
                    ast::ClassKind::Trait => "trait",
                    ast::ClassKind::Object => "module object",
                    _ => "class",
                };
                let case = def.mods & mods::CASE != 0;
                let mut t = self.mods_text(def.mods).cat_sp(Text::str(kw)).cat_sp(Text::str(name));
                t = t.cat(self.tparams_text(&c.tparams));
                for clause in &c.clauses {
                    t = t.cat(self.clause_text(clause, Some(case)));
                }
                let parents: Vec<Text> = c.parents.iter().map(|p| self.parent_text(p)).collect();
                if !parents.is_empty() {
                    t = t.cat(" extends").cat_sp(Text::join(parents, ", "));
                }
                // The body's statements; written braces without any are `templateStatSeq`'s `<empty>`.
                let mut body: Vec<Text> = c.body.iter().flat_map(|&st| self.stmt_lines(st)).collect();
                if body.is_empty() && self.text.get(..ast.def_range(d).end as usize).is_some_and(|t| t.trim_end().ends_with('}')) {
                    body.push("<empty>".into());
                }
                let body = self.at_prec(GLOBAL_PREC, |_| Text::join(body, "\n"));
                t.cat(Text::str(" {").cat(body).cat("}"))
            }
            DefKind::TypeAlias { tparams, rhs, lower, upper } => {
                let mut t = self.mods_text(def.mods).cat_sp(Text::str("type")).cat_sp(Text::str(name));
                t = t.cat(self.tparams_text(tparams));
                if let Some(rhs) = rhs {
                    t = t.cat(Text::str(" = ").cat(self.ty(*rhs)));
                } else {
                    if let Some(lo) = lower {
                        t = t.cat(Text::str(" >: ").cat(self.ty(*lo)));
                    }
                    if let Some(hi) = upper {
                        t = t.cat(Text::str(" <: ").cat(self.ty(*hi)));
                    }
                }
                t
            }
            // An alias given as `Desugar` makes it: a `final lazy given val`, or a `final given def` where it
            // takes parameters; an anonymous one's synthetic name is not shown.
            DefKind::Given(g) if g.alias.is_some() => {
                let written = self.text.get(def.span.start as usize..def.span.end as usize) == Some(name.as_str());
                let shown = if written { name } else { String::new() };
                let mut t = if g.tparams.is_empty() && g.clauses.is_empty() {
                    Text::str("final lazy given val").cat_sp(Text::str(shown))
                } else {
                    let mut t = Text::str("final given def").cat_sp(Text::str(shown)).cat(self.tparams_text(&g.tparams));
                    for clause in &g.clauses {
                        t = t.cat(self.clause_text(clause, None));
                    }
                    t
                };
                t = t.cat(Text::str(": ").cat(self.ty(g.ty)));
                if let Some(alias) = g.alias {
                    t = t.cat(Text::str(" = ").cat(self.expr(alias)));
                }
                t
            }
            DefKind::Given(_) => Text::str("given").cat_sp(Text::str(name)),
        }
    }

    /// A parent of a class's template (`constrText`): its type, then each argument list.
    fn parent_text(&mut self, p: &ast::Parent) -> Text {
        let ast = self.ast;
        let mut t = self.at_prec(DOT_PREC, |s| s.ty(p.ty));
        for &(args, using) in &p.args {
            let args = self.global_exprs(ast.expr_list(args), ", ");
            t = t.cat("(").cat(if using { "using " } else { "" }).cat(args).cat(")");
        }
        t
    }

    fn enumerator(&mut self, en: &Enumerator) -> Text {
        match *en {
            Enumerator::Gen(p, e, _) => self.pat(p).cat(" <- ").cat(self.expr(e)),
            Enumerator::CaseGen(p, e) => Text::str("case ").cat(self.pat(p)).cat(" <- ").cat(self.expr(e)),
            Enumerator::Guard(e) => Text::str("if ").cat(self.expr(e)),
            Enumerator::Val(p, e) => self.pat(p).cat(" = ").cat(self.expr(e)),
        }
    }

    /// `InterpolatedString`: each part escaped, each hole after it in braces, a hole written in braces
    /// itself a block (`blockExpr`).
    fn interpolation(&mut self, id: Name, parts: ast::ListRef, holes: &[ExprId]) -> Text {
        let ast = self.ast;
        let bytes = self.text.as_bytes();
        let part_text = |i: usize| -> String {
            let raw = match ast.part_span(parts.start as usize + i) {
                Some(s) => self.text.get(s.start as usize..s.end as usize).unwrap_or("").replace("$$", "$"),
                None => ast.str(ast.str_lists[parts.start as usize + i]).to_string(),
            };
            escaped_string(&raw, false)
        };
        let mut segments = Vec::with_capacity(holes.len() + 1);
        for (i, &h) in holes.iter().enumerate() {
            let start = ast.expr_span(h).start as usize;
            let braced = bytes[..start.min(bytes.len())].iter().rev().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
            let hole = self.at_prec(GLOBAL_PREC, |s| {
                let e = s.expr(h);
                if braced { s.block_text(vec![e]) } else { e }
            });
            segments.push(Text::str(part_text(i)).cat("{").cat(hole).cat("}"));
        }
        if parts.len as usize > holes.len() {
            segments.push(Text::str(part_text(holes.len())));
        }
        Text::str(self.name(id)).cat("\"").cat(Text::join(segments, "")).cat("\"")
    }

    fn pats(&mut self, l: &[PatId], sep: &str) -> Text {
        let ts: Vec<Text> = l.iter().map(|&p| self.pat(p)).collect();
        Text::join(ts, sep)
    }

    fn pat(&mut self, p: PatId) -> Text {
        self.pat_core(p).close()
    }

    fn pat_core(&mut self, p: PatId) -> Text {
        let ast = self.ast;
        match ast.pat(p) {
            Pat::Wildcard => "_".into(),
            Pat::Bind(n, None) => self.name(n).into(),
            Pat::Bind(n, Some(inner)) => {
                let n = self.name(n);
                self.change_prec(INFIX_PREC, |s| Text::str(n).cat(" @ ").cat(s.pat(inner)))
            }
            Pat::Typed(inner, t) => self.change_prec(DOT_PREC, |s| {
                let text = s.pat(inner);
                let colon = if text.last_line().chars().last().is_some_and(is_operator_part) { " :" } else { ":" };
                text.cat(colon).cat(s.ty(t))
            }),
            Pat::Lit(e) | Pat::StableId(e) | Pat::Quote(e) => self.expr(e),
            Pat::Ctor(path, subs) => {
                let subs = ast.pat_list(subs).to_vec();
                let infix = !ast.notes.infix_pattern_ops.is_empty() && ast.notes.infix_pattern_ops.binary_search(&path).is_ok();
                match (infix, subs.as_slice(), ast.expr(path)) {
                    (true, &[l, r], Expr::Ident(op)) => {
                        let op = self.name(op);
                        self.change_prec(precedence(&op), |s| s.pat(l).cat(" ").cat(op.as_str()).cat(" ").cat(s.pat(r)))
                    }
                    _ => {
                        let fun = self.local(path);
                        let args = self.at_prec(GLOBAL_PREC, |s| s.pats(&subs, ", "));
                        fun.cat("(").cat(args).cat(")")
                    }
                }
            }
            Pat::Tuple(items) => {
                let items = ast.pat_list(items).to_vec();
                let items = self.at_prec(GLOBAL_PREC, |s| s.pats(&items, ", "));
                Text::str("(").cat(items).cat(")")
            }
            Pat::NamedField(n, inner) => Text::str(self.name(n)).cat(" = ").cat(self.pat(inner)),
            Pat::Alt(items) => {
                let items = ast.pat_list(items).to_vec();
                self.change_prec(OR_PREC, |s| s.pats(&items, " | "))
            }
            Pat::Rest(inner) => self.pat(inner).cat("*"),
            Pat::QuoteType(t) => Text::str("'[").cat(self.ty(t)).cat("]"),
            Pat::Interp(id, _, _) => Text::str(self.name(id)).cat("\"...\""),
            Pat::Error => "<error>".into(),
        }
    }

    fn global_types(&mut self, l: &[TyExprId]) -> Text {
        self.at_prec(GLOBAL_PREC, |s| {
            let ts: Vec<Text> = l.iter().map(|&t| s.ty(t)).collect();
            Text::join(ts, ", ")
        })
    }

    fn ty(&mut self, t: TyExprId) -> Text {
        self.ty_core(t).close()
    }

    fn ty_core(&mut self, t: TyExprId) -> Text {
        let ast = self.ast;
        match ast.ty(t) {
            TyExpr::Name(n) | TyExpr::TypeVar(n) => self.name(n).into(),
            TyExpr::Select(q, n) => self.at_prec(DOT_PREC, |s| s.ty(q)).cat(".").cat(self.name(n)),
            TyExpr::Project(q, n) => self.at_prec(DOT_PREC, |s| s.ty(q)).cat("#").cat(self.name(n)),
            TyExpr::Apply(f, args) => {
                let fun = self.at_prec(DOT_PREC, |s| s.ty(f));
                let args = self.global_types(ast.ty_list(args));
                fun.cat("[").cat(args).cat("]")
            }
            TyExpr::Fun(params, ret) | TyExpr::CtxFun(params, ret) => {
                let arrow = if matches!(ast.ty(t), TyExpr::CtxFun(..)) { " ?=> " } else { " => " };
                let named = ast.fun_param_names.iter().find(|&&(f, _)| f == t).map(|&(_, l)| l);
                let params = ast.ty_list(params).to_vec();
                let ts: Vec<Text> = match named {
                    Some(names) => params.iter().zip(&ast.name_lists[names.range()]).map(|(&p, &n)| Text::str(self.name(n)).cat(": ").cat(self.ty(p))).collect(),
                    None => params.iter().map(|&p| self.ty(p)).collect(),
                };
                let ret = self.ty(ret);
                Text::str("(").cat(Text::join(ts, ", ")).cat(")").cat(arrow).cat(ret)
            }
            TyExpr::Tuple(items) => Text::str("(").cat(self.global_types(ast.ty_list(items))).cat(")"),
            TyExpr::NamedTuple(names, tys) => {
                let pairs: Vec<(Name, TyExprId)> = ast.name_lists[names.range()].iter().copied().zip(ast.ty_list(tys).iter().copied()).collect();
                let ts: Vec<Text> = pairs.iter().map(|&(n, ty)| Text::str(self.name(n)).cat(" : ").cat(self.ty(ty))).collect();
                Text::str("(").cat(Text::join(ts, ", ")).cat(")")
            }
            TyExpr::Union(a, b) => self.change_prec(OR_PREC, |s| s.ty(a).cat(" | ").cat(s.ty(b))),
            TyExpr::Inter(a, b) => self.change_prec(AND_PREC, |s| s.ty(a).cat(" & ").cat(s.ty(b))),
            TyExpr::ByName(inner) => Text::str("=> ").cat(self.at_prec(DOT_PREC, |s| s.ty(inner))),
            TyExpr::Repeated(inner) => self.change_prec(INFIX_PREC, |s| s.ty(inner).cat(" *")),
            TyExpr::Wildcard => "?".into(),
            TyExpr::BoundedWildcard(lo, hi) => Text::str("? >: ").cat(self.ty(lo)).cat(" <: ").cat(self.ty(hi)),
            TyExpr::Singleton(p) => self.at_prec(DOT_PREC, |s| s.ty(p)).cat(".type"),
            TyExpr::Lit(e) => self.expr(e),
            TyExpr::Unchecked(inner) | TyExpr::UncheckedVariance(inner) | TyExpr::Annotated(inner, _) => self.ty(inner),
            TyExpr::Lambda(..) | TyExpr::PolyFun(..) | TyExpr::Refined(..) | TyExpr::Match(..) | TyExpr::MatchCase(..) | TyExpr::Resolved(_) | TyExpr::MainParam(..) | TyExpr::Error => "<type>".into(),
        }
    }
}
