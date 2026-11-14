//! scalac's `ForwardDepChecks`: a value of a block defined at or before a definition of that
//! block which a reference standing at or before the value names is a forward reference
//! extending over the value's definition (E039). The check runs after the inlining phase, over
//! the final trees, as dotty's runs after `Inlining`: a call the phase expanded is no reference,
//! what its expansion names is one.
//!
//! A level is a block's statement list. What stands where in it is read from the syntax tree,
//! which every build keeps: the typed block is matched to its syntax block by the name position
//! and the name of one definition it holds, and the syntax block gives each statement's ordinal
//! and kind (a class, a trait, an enum and an inline method leave no typed statement). What a
//! statement NAMES is read from the typed trees: the expansions in the place of their calls, a
//! folded `if`'s taken branch, a default's getter, a creation's class or the type alias of a block
//! it is written by (never what the class captures: scalac's tree is lifted long after its check).
//! A class's trees are read in the order of scalac's template: the parents' calls, the body as the
//! source has it, the parameters' defaults where the class's companion stands. A typed block that
//! finds no syntax block (an expansion's bindings, the lowering's temporaries, a tupled lambda's
//! parameters, a `for`'s values) is no level: it is walked for what it names in the levels around
//! it and tests nothing; a class it declares stands where it is first made.
//!
//! The place an error is told at is the reference's, which the typed trees do not keep: the
//! typing notes it (`Program::forward`) for the nodes that can decide, those that name a
//! definition of an open block from at or before that definition's statement.

use super::Worker;
use crate::ast::{self, mods, DefKind as DefKindAst, Expr, Stmt};
use crate::intern::FxMap;
use crate::source::{DiagOwner, FileId, Span, PHASE_FORWARD_DEP_CHECKS};
use crate::symbols::{ClassKind, Owner, SymKind};
use crate::tir::*;
use crate::types::{ClassId, SymId};
use std::sync::Arc;

/// Whether the check writes what it decides (`FORWARD`), each deciding reference it has no
/// place for (`NOPLACE`: none, a reference without a place naming nothing), each node without
/// a place that reads a level's definition (`UNNOTED`), each decision in a unit no note named
/// (`OFFUNIT`) and each typed block
/// that holds a value and finds no syntax block (`UNMATCHED`), and walks every unit for it.
pub(super) fn forward_dump() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("TEQ_FORWARD_DUMP").is_some())
}

/// A definition a level holds, as a reference names it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Named {
    Term(SymId),
    Class(ClassId),
    /// A type alias of a block, by its definition.
    Alias(FileId, ast::DefId),
}

/// The reference that reached a level's farthest definition first.
#[derive(Clone, Copy)]
struct Reference {
    named: Named,
    /// The parameter's place, from one, where the reference is a default's getter.
    getter: u32,
    kind: &'static str,
    place: Option<(FileId, Span)>,
}

struct Level {
    /// The farthest ordinal a reference named so far (dotty's `maxIndex`).
    max: Option<Ordinal>,
    by: Option<Reference>,
    /// The syntax block stands in a quote, or in an inline method's body.
    quoted: bool,
    inline_body: bool,
    /// The syntax block.
    syntax: (FileId, ast::ListRef),
}

/// A definition among a syntax block's statements: where its name (a pattern definition's
/// pattern) stands, the block and the statement's place in it.
struct SyntaxDef {
    start: u32,
    end: u32,
    block: ast::ListRef,
    at: u32,
    def: ast::DefId,
    quoted: bool,
    inline_body: bool,
}

struct Walk {
    /// Per file its blocks' definitions by position, made when the file's first block asks.
    syntax: FxMap<FileId, Vec<SyntaxDef>>,
    levels: Vec<Level>,
    /// The levels under way, innermost last.
    open: Vec<u32>,
    /// Each definition of a level under way or done: its level and its ordinal there.
    named: FxMap<Named, (u32, Ordinal)>,
    /// The classes whose trees were walked.
    walked: FxMap<ClassId, ()>,
    /// The stored bodies under walk, whose classes are their own.
    stored: Vec<Arc<InlineDefinition>>,
    /// The expansions the walk stands in, outermost first.
    expansions: Vec<TExprId>,
    /// The places an error was told at: scalac hides a second message at an offset one took,
    /// a place's end among them (`UniqueMessagePositions`).
    told: Vec<(FileId, Span)>,
    /// The file of the unit under walk.
    unit: FileId,
    lines: Vec<String>,
}

impl Walk {
    fn syntax_at(&self, file: FileId, pos: u32) -> Option<&SyntaxDef> {
        let defs = self.syntax.get(&file)?;
        let i = defs.partition_point(|d| d.start <= pos);
        let d = defs.get(i.checked_sub(1)?)?;
        (pos < d.end).then_some(d)
    }
}

/// Where a definition stands in its level: its syntax statement's place, then its place among
/// the typed statements that statement gave (a tuple pattern taken apart gives one a name).
type Ordinal = (u32, u32);

/// The names the terms of a written type start with: a singleton's path (`x.type`) and the
/// prefix of a selection (`x.T`), wherever they stand in it.
fn singleton_heads(ast: &ast::Ast, t: ast::TyExprId, out: &mut Vec<crate::intern::Name>) {
    use ast::TyExpr;
    let head = |mut p: ast::TyExprId, out: &mut Vec<crate::intern::Name>| loop {
        match ast.ty(p) {
            TyExpr::Name(n) => {
                out.push(n);
                return;
            }
            TyExpr::Select(q, _) => p = q,
            _ => return,
        }
    };
    match ast.ty(t) {
        TyExpr::Singleton(p) => head(p, out),
        TyExpr::Select(q, _) | TyExpr::Project(q, _) => head(q, out),
        TyExpr::Apply(c, args) => {
            singleton_heads(ast, c, out);
            for &a in ast.ty_list(args) {
                singleton_heads(ast, a, out);
            }
        }
        TyExpr::Tuple(l) => {
            for &a in ast.ty_list(l) {
                singleton_heads(ast, a, out);
            }
        }
        TyExpr::Fun(l, r) | TyExpr::CtxFun(l, r) => {
            for &a in ast.ty_list(l) {
                singleton_heads(ast, a, out);
            }
            singleton_heads(ast, r, out);
        }
        TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
            singleton_heads(ast, a, out);
            singleton_heads(ast, b, out);
        }
        TyExpr::ByName(a) | TyExpr::Repeated(a) | TyExpr::Refined(a, _) => singleton_heads(ast, a, out),
        _ => {}
    }
}

/// The typing's side: the notes.
impl<'a> Worker<'a> {
    /// Whether the definition named at `at` in `file` is a statement of an open block that the
    /// block's typing has not passed: the statement under way, or one after it.
    fn forward_ahead(&self, file: FileId, at: u32) -> bool {
        for b in self.local_blocks.iter().rev() {
            if b.file != file {
                continue;
            }
            let ast = self.ast(file);
            let stmts = ast.stmt_list(b.stmts);
            for st in stmts.iter().skip(b.next) {
                if let Stmt::Def(d) = *st {
                    let span = ast.def(d).span;
                    if span.start <= at && at < span.end.max(span.start + 1) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The note of `node`, standing in the unit under way.
    fn forward_put(&mut self, node: TExprId, span: Span, parts: Vec<SymId>) {
        let file = self.env.file;
        self.prog.forward.places.insert(node, ForwardPlace { file, span, parts, alias: None });
        self.forward_unit();
    }

    /// The unit a node made now stands in: the outermost call's where an expansion is under way.
    fn forward_unit(&mut self) {
        for file in [self.env.file, self.inline.sites.first().map_or(self.env.file, |s| s.file)] {
            if !self.prog.forward.units.contains(&file) {
                self.prog.forward.units.push(file);
            }
        }
    }

    /// The identifier written at `at`: `name`, in backquotes or not; else nothing but the point.
    fn forward_ident_span(&self, at: u32, name: crate::intern::Name) -> Span {
        let text = self.source(self.env.file).text.as_bytes();
        let name = self.name_str(name);
        let from = at as usize;
        let quoted = text.get(from) == Some(&b'`');
        let start = from + quoted as usize;
        if text.get(start..start + name.len()) == Some(name.as_bytes()) {
            return Span { start: at, end: (start + name.len() + quoted as usize) as u32 };
        }
        Span { start: at, end: at }
    }

    /// The created type of the creation written at `span`: what follows `new`, to the end of
    /// its last type argument; a constructor proxy's is its identifier.
    fn forward_created_span(&self, span: Span) -> Span {
        let text = self.source(self.env.file).text.as_bytes();
        let mut i = span.start as usize;
        let ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80;
        if text.get(i..i + 3) == Some(b"new") && text.get(i + 3).is_some_and(|&b| !ident(b)) {
            i += 3;
            while text.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
                i += 1;
            }
        }
        let start = i;
        let mut depth = 0usize;
        while let Some(&b) = text.get(i) {
            match b {
                b'[' => depth += 1,
                b']' if depth > 0 => depth -= 1,
                _ if depth > 0 => {}
                b'.' | b'`' | b'#' => {}
                _ if ident(b) => {}
                _ => break,
            }
            i += 1;
        }
        Span { start: start as u32, end: (i.max(start)) as u32 }
    }

    /// A value read by its name (`term_callee`): a lazy value, an object's or a given's, named
    /// before its statement is passed; a value named while its own initialiser is typed
    /// (`SymInfo::initialising`).
    #[inline]
    pub(super) fn forward_note_value(&mut self, node: TExprId, s: SymId, span: Span, lazy: bool, initialising: bool) {
        if lazy {
            self.forward_note_term(node, s, span);
        } else if initialising {
            self.forward_put(node, span, Vec::new());
        }
    }

    #[cold]
    #[inline(never)]
    fn forward_note_term(&mut self, node: TExprId, s: SymId, span: Span) {
        let info = self.syms.sym(s);
        let (owner, file, at) = (info.owner, info.file, info.span.start);
        if owner == Owner::Local && self.forward_ahead(file, at) {
            self.forward_put(node, span, Vec::new());
        }
    }

    /// The call of a method written at `span`: a block's method named before its statement is
    /// passed is named by the identifier the application starts with.
    #[inline]
    pub(super) fn forward_note_call(&mut self, node: TExprId, callee: SymId, span: Span) {
        if self.syms.sym(callee).owner == Owner::Local {
            self.forward_note_call_of(node, callee, span);
        }
    }

    #[cold]
    #[inline(never)]
    fn forward_note_call_of(&mut self, node: TExprId, callee: SymId, span: Span) {
        if !matches!(self.prog.expr(node), TExpr::CallStatic(s, _) if s == callee) {
            return;
        }
        let info = self.syms.sym(callee);
        let (file, at, name) = (info.file, info.span.start, info.name);
        if self.forward_ahead(file, at) {
            let mut place = self.forward_ident_span(span.start, name);
            // An extension method applied to its receiver is named after it.
            if place.end == place.start && self.syms.sym(callee).is_extension {
                if let Some(written) = self.forward_name_within(span, name) {
                    place = written;
                }
            }
            self.forward_put(node, place, Vec::new());
        }
    }

    /// Where `name` is first written within `span` as a whole name.
    fn forward_name_within(&self, span: Span, name: crate::intern::Name) -> Option<Span> {
        let text = self.source(self.env.file).text.as_bytes();
        let name = self.name_str(name);
        let n = name.as_bytes();
        let part = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80;
        let symbolic = n.first().is_some_and(|&b| !part(b));
        let joins = |b: u8| match symbolic {
            true => !part(b) && !b.is_ascii_whitespace() && !matches!(b, b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'.' | b',' | b';' | b'`' | b'"' | b'\''),
            false => part(b),
        };
        let (from, to) = (span.start as usize, (span.end as usize).min(text.len()));
        let last = to.checked_sub(n.len())?;
        (from..=last)
            .find(|&i| text.get(i..i + n.len()) == Some(n) && (i == from || !joins(text[i - 1])) && text.get(i + n.len()).map_or(true, |&b| !joins(b)))
            .map(|i| Span { start: i as u32, end: (i + n.len()) as u32 })
    }

    /// A given found for an extension method applied at `span`: the method is selected on it,
    /// where the application starts.
    pub(super) fn forward_note_extension_given(&mut self, node: TExprId, span: Span) {
        if self.prog.forward.places.is_empty() {
            return;
        }
        if let Some(note) = self.prog.forward.places.get_mut(&node) {
            note.span = Span { start: span.start, end: span.start };
        }
    }

    /// A default's getter applied in the call written at `span`: dotty's identifier of the
    /// getter stands where the call's function does.
    pub(super) fn forward_note_default(&mut self, node: TExprId, span: Span) {
        match self.prog.expr(node) {
            TExpr::Default(DefaultOf::Method(m), ..) => {
                let info = self.syms.sym(m);
                let (owner, file, at, name) = (info.owner, info.file, info.span.start, info.name);
                if owner == Owner::Local && self.forward_ahead(file, at) {
                    let place = self.forward_ident_span(span.start, name);
                    self.forward_put(node, place, Vec::new());
                }
            }
            // A constructor's getter is selected on the class's companion, a block class's on
            // its object of the block: scalac's tree names it where the created type starts.
            TExpr::Default(DefaultOf::Ctor(_), at, args) if at & DEFAULT_RECEIVER != 0 => {
                let Some(&recv) = self.prog.expr_list(args).first() else { return };
                let TExpr::Local(v) = self.prog.expr(recv) else { return };
                let info = self.syms.sym(v);
                let (owner, file, pos) = (info.owner, info.file, info.span.start);
                if owner == Owner::Local && !self.prog.forward.places.contains_key(&recv) && self.forward_ahead(file, pos) {
                    let start = self.forward_created_place(span, self.forward_written_type(span)).start;
                    self.forward_put(recv, Span { start, end: start }, Vec::new());
                }
            }
            _ => {}
        }
    }

    /// A given the search found for the application written at `span`: scalac places an inferred
    /// argument at the end of what it completes.
    #[inline]
    pub(super) fn forward_note_given(&mut self, node: TExprId, g: SymId, span: Span) {
        if self.syms.sym(g).owner == Owner::Local {
            self.forward_note_given_of(node, g, span);
        }
    }

    #[cold]
    #[inline(never)]
    fn forward_note_given_of(&mut self, node: TExprId, g: SymId, span: Span) {
        let info = self.syms.sym(g);
        let (file, at) = (info.file, info.span.start);
        if self.forward_ahead(file, at) {
            self.forward_put(node, Span { start: span.end, end: span.end }, Vec::new());
        }
    }

    /// A creation written at `span`: of a block's class, or of a member of one, named before the
    /// block class's statement is passed: its place is the created type's.
    #[inline]
    pub(super) fn forward_note_creation(&mut self, node: TExprId, c: ClassId, span: Span) {
        let mut k = c;
        loop {
            match self.syms.class(k).owner {
                Owner::Local => break,
                Owner::Class(outer) => k = outer,
                _ => return,
            }
        }
        self.forward_note_creation_of(node, c, k, span);
    }

    #[cold]
    #[inline(never)]
    fn forward_note_creation_of(&mut self, node: TExprId, c: ClassId, k: ClassId, span: Span) {
        let info = self.syms.class(k);
        let (kind, file, at) = (info.kind, info.file, info.span.start);
        if kind != ClassKind::Anon && self.forward_ahead(file, at) && !self.prog.forward.places.contains_key(&node) {
            let written = self.forward_written_type(span);
            let place = self.forward_created_place(span, written);
            let alias = written.and_then(|ty| self.forward_resolved_alias(ty, c));
            self.forward_put(node, place, Vec::new());
            if let Some(note) = self.prog.forward.places.get_mut(&node) {
                note.alias = alias;
            }
        }
    }

    /// Where the creation at `span` writes its created type: dotty's `New` has its type tree's
    /// span, a creation without `new` its name's.
    fn forward_created_place(&self, span: Span, written: Option<ast::TyExprId>) -> Span {
        match written {
            Some(ty) => self.cur_ast().ty_spans[ty.idx()],
            None => self.forward_created_span(span),
        }
    }

    /// The type the syntax's `new` at `span` writes: its own, or the first parent of the
    /// anonymous class it makes.
    fn forward_written_type(&self, span: Span) -> Option<ast::TyExprId> {
        let ast = self.cur_ast();
        ast.exprs.iter().enumerate().find_map(|(i, x)| match x {
            Expr::New(ty, _) if ast.expr_spans[i].start == span.start => Some(*ty),
            Expr::NewAnon(d) if ast.expr_spans[i].start == span.start => match &ast.def(*d).kind {
                DefKindAst::Class(cls) => cls.parents.first().map(|p| p.ty),
                _ => None,
            },
            _ => None,
        })
    }

    /// The type alias the typer resolves the written type `ty` of a creation of `c` to, where
    /// the creation keeps it: the name the type's constructor is written by, resolved as the
    /// typing of the creation resolved it. An alias that is its class applied to its own
    /// parameters is that class where it is applied (dotty reduces it at the application), and
    /// no alias here. A type reached through a value's path (`p.T`), which no scope lookup
    /// names, is the class where it bears the class's own name and an alias of the path else.
    fn forward_resolved_alias(&mut self, ty: ast::TyExprId, c: ClassId) -> Option<Option<ast::DefId>> {
        use super::resolve::TypeRef;
        use ast::TyExpr;
        let file = self.env.file;
        let (qualifier, name) = {
            let ast = self.cur_ast();
            let mut head = ty;
            loop {
                match ast.ty(head) {
                    TyExpr::Apply(f, _) => head = f,
                    TyExpr::Annotated(t, _) => head = t,
                    TyExpr::Tuple(l) if l.len == 1 => head = ast.ty_list(l)[0],
                    _ => break,
                }
            }
            match ast.ty(head) {
                TyExpr::Name(n) => (None, n),
                TyExpr::Select(q, n) => (Some(q), n),
                _ => return None,
            }
        };
        let found = match qualifier {
            None => self.lookup_type(name),
            Some(q) => self.lookup_type_in_path(q, name),
        };
        let a = match found {
            Some(TypeRef::Alias(a)) => a,
            Some(TypeRef::Class(_)) => return None,
            _ if self.syms.class(c).name == name => return None,
            _ => return Some(None),
        };
        let info = &self.syms.aliases[a.idx()];
        let reduced = !info.tparams.is_empty()
            && match self.types.get(info.rhs) {
                crate::types::Type::Class(_, args) => {
                    let args = self.types.items(args);
                    args.len() == info.tparams.len() && args.iter().zip(&info.tparams).all(|(&t, &p)| matches!(self.types.get(t), crate::types::Type::Param(q) if q == p))
                }
                _ => false,
            };
        if reduced {
            return None;
        }
        Some(if info.owner == Owner::Local && info.file == file { info.def } else { None })
    }

    /// The definitions of open blocks that the type arguments written for the creation `node`
    /// name as singletons (`new Box[later.type]`), which the trees do not hold: the creation's
    /// note takes them, at the created type's place.
    #[cold]
    #[inline(never)]
    pub(super) fn forward_note_type_args(&mut self, node: TExprId, targs: ast::ListRef, span: Span) {
        if !matches!(self.prog.expr(node), TExpr::New(..) | TExpr::NewVia(..)) {
            return;
        }
        let ast = self.cur_ast();
        let mut names = Vec::new();
        for &t in ast.ty_list(targs) {
            singleton_heads(ast, t, &mut names);
        }
        let mut parts = Vec::new();
        for name in names {
            if let Some(super::resolve::TermRef::Local(s)) = self.lookup_term(name) {
                let info = self.syms.sym(s);
                let (owner, file, at) = (info.owner, info.file, info.span.start);
                if owner == Owner::Local && self.forward_ahead(file, at) && !parts.contains(&s) {
                    parts.push(s);
                }
            }
        }
        if parts.is_empty() {
            return;
        }
        match self.prog.forward.places.get_mut(&node) {
            Some(place) => place.parts = parts,
            None => {
                let place = self.forward_created_place(span, self.forward_written_type(span));
                self.forward_put(node, place, parts);
            }
        }
    }

    /// A constructor pattern written at `span`: of a block's class named before its statement
    /// is passed: the pattern selects the class's companion by the identifier it starts with.
    #[inline]
    pub(super) fn forward_note_pattern(&mut self, p: TPatId, c: ClassId, span: Span) {
        match self.syms.class(c).owner {
            Owner::Local => self.forward_note_pattern_of(p, c, span),
            Owner::Class(_) => self.forward_note_member_pattern(p, c, span),
            _ => {}
        }
    }

    /// The value of the block's object that holds the member class `c`, through the classes
    /// between; none where a class of the way is no object of a block.
    fn forward_holder(&self, c: ClassId) -> Option<SymId> {
        let mut k = c;
        loop {
            match self.syms.class(k).owner {
                Owner::Class(outer) => k = outer,
                Owner::Local if k != c => return self.syms.class(k).local_module,
                _ => return None,
            }
        }
    }

    /// A constructor pattern of a member class of a block's object, named before the object's
    /// statement is passed: the pattern selects the class's companion through the path it
    /// writes, so the object is named where that path starts at the object's own value
    /// (`Later.Inner(v)`, `Later.held.Inner(v)`) or the name was imported from it, at the
    /// pattern's start. A path through another value (`l.Inner(v)`, `l` a parameter of the
    /// object's type) and a bare name inside the object's own body name nothing.
    #[cold]
    #[inline(never)]
    fn forward_note_member_pattern(&mut self, p: TPatId, c: ClassId, span: Span) {
        let Some(m) = self.forward_holder(c) else { return };
        let info = self.syms.sym(m);
        let (file, at, name) = (info.file, info.span.start, info.name);
        if self.forward_ahead(file, at) && self.forward_pattern_head(c, span) == Some(m) {
            let place = self.forward_ident_span(span.start, name);
            let here = self.env.file;
            self.prog.forward.patterns.insert(p, (here, place));
            self.forward_unit();
        }
    }

    /// The value a constructor pattern of the class `c` written at `span` selects its class
    /// through: the head of the path it writes (`a` of `a.b.C(..)`), or of the import its bare
    /// name came by (`import a.b.*; C(..)`), as the scope resolves it where the pattern is typed.
    fn forward_pattern_head(&mut self, c: ClassId, span: Span) -> Option<SymId> {
        use super::resolve::TermRef;
        use crate::types::Type;
        let ast = self.cur_ast();
        let class_name = self.syms.class(c).name;
        let last = |path: ast::ExprId| match ast.expr(path) {
            Expr::Select(_, n) | Expr::Ident(n) => Some(n),
            _ => None,
        };
        // An infix pattern starts where its left operand does: the pattern of this class first.
        let mut paths = ast.pats.iter().enumerate().filter_map(|(i, p)| match *p {
            ast::Pat::Ctor(path, _) if ast.pat_spans[i].start == span.start => Some(path),
            _ => None,
        });
        let first = paths.next()?;
        let path = if last(first) == Some(class_name) { first } else { paths.find(|&q| last(q) == Some(class_name)).unwrap_or(first) };
        let mut head = path;
        while let Expr::Select(q, _) = ast.expr(head) {
            head = q;
        }
        let Expr::Ident(name) = ast.expr(head) else { return None };
        match self.lookup_term(name)? {
            TermRef::Local(s) => {
                // A name an import of a local value's member made stands for that value's path.
                let through = self.syms.sym(s).sig.as_ref().map(|sig| sig.ret).and_then(|ret| match self.types.get(ret) {
                    Type::Nested(prefix, _) => Some(prefix),
                    _ => None,
                });
                match through {
                    Some(mut t) => loop {
                        match self.types.get(t) {
                            Type::Term(v) => break Some(v),
                            Type::Select(q, _) => t = q,
                            _ => break None,
                        }
                    },
                    None => Some(s),
                }
            }
            TermRef::ValueMember(mut value, _) | TermRef::ValueClass(mut value, _) => loop {
                let (v, _, through) = self.import_values[value.0 as usize];
                match through {
                    Some(previous) => value = previous,
                    None => break Some(v),
                }
            },
            // A member of an object: by an import of the object (`import a.*`, `import a.b.C`)
            // the block's object the import's path starts at; inside that object the name is
            // `this`'s member and no path is written.
            TermRef::ModuleMember(k, _) | TermRef::ModuleClass(k, _) => {
                let mut k = k;
                loop {
                    if self.env.frames.iter().any(|f| matches!(f, super::Frame::Class(x) if *x == k)) {
                        break None;
                    }
                    match self.syms.class(k).owner {
                        Owner::Class(outer) => k = outer,
                        Owner::Local => break self.syms.class(k).local_module,
                        _ => break None,
                    }
                }
            }
            _ => None,
        }
    }

    #[cold]
    #[inline(never)]
    fn forward_note_pattern_of(&mut self, p: TPatId, c: ClassId, span: Span) {
        let info = self.syms.class(c);
        let (file, at, name) = (info.file, info.span.start, info.name);
        if self.forward_ahead(file, at) {
            let place = self.forward_ident_span(span.start, name);
            let here = self.env.file;
            self.prog.forward.patterns.insert(p, (here, place));
            self.forward_unit();
        }
    }

    /// A copy of the node `from` is `to`: it names what `from` names, from the same place, and
    /// stands in the unit the copy is made for.
    #[cold]
    #[inline(never)]
    pub(super) fn forward_copy(&mut self, from: TExprId, to: TExprId) {
        if let Some(place) = self.prog.forward.places.get(&from).cloned() {
            self.prog.forward.places.insert(to, place);
            self.forward_unit();
        }
    }

    /// An expansion's root `from` gives its value to its call's node `to`: the node names what
    /// the root named, from the root's place, and no longer the callee the call named.
    #[cold]
    #[inline(never)]
    pub(super) fn forward_move(&mut self, from: TExprId, to: TExprId) {
        match self.prog.forward.places.remove(&from) {
            Some(place) => {
                self.prog.forward.places.insert(to, place);
            }
            None => {
                self.prog.forward.places.remove(&to);
            }
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn forward_copy_pattern(&mut self, from: TPatId, to: TPatId) {
        if let Some(place) = self.prog.forward.patterns.get(&from).copied() {
            self.prog.forward.patterns.insert(to, place);
            self.forward_unit();
        }
    }
}

/// The check.
impl<'a> Worker<'a> {
    /// The walk over the units a noted node stands in: `only`, the files a retype typed again,
    /// or every one.
    pub(super) fn forward_references(&mut self, only: Option<&[FileId]>) {
        let probe = forward_dump();
        if self.diags.typer_failed() || (self.prog.forward.units.is_empty() && !probe) {
            return;
        }
        let noted: Vec<FileId> = self.prog.forward.units.clone();
        let files: Vec<FileId> = (0..self.asts.len() as u32)
            .map(FileId)
            .filter(|&f| self.program_file(f) && (probe || noted.contains(&f)) && only.map_or(true, |o| o.contains(&f)))
            .collect();
        if files.is_empty() {
            return;
        }
        let mut w = Walk {
            syntax: FxMap::default(),
            levels: Vec::new(),
            open: Vec::new(),
            named: FxMap::default(),
            walked: FxMap::default(),
            stored: Vec::new(),
            expansions: Vec::new(),
            told: Vec::new(),
            unit: FileId(0),
            lines: Vec::new(),
        };
        let (outer_bodies, outer_owner) = (self.diags.of_bodies, self.diags.owner);
        self.diags.of_bodies = true;
        self.diags.owner = DiagOwner::NONE;
        // An inline method's body as its definition stores it, which scalac's tree still holds;
        // a block's own is walked where its statement stands.
        let stored: Vec<(SymId, Arc<InlineDefinition>)> = self.inline_definitions.iter().map(|(&s, d)| (s, d.clone())).collect();
        for (sym, def) in stored {
            let info = self.syms.sym(sym);
            let (file, owner) = (info.file, info.owner);
            let in_block = match owner {
                Owner::Local => true,
                Owner::Class(c) => self.forward_in_block(c),
                _ => false,
            };
            if files.contains(&file) && !in_block {
                w.unit = file;
                self.forward_stored(&mut w, &def);
            }
        }
        // The classes that are no block's: each with its members. A block's class is walked
        // where its statement stands, an anonymous class where it is made.
        let classes: Vec<ClassId> = self.prog.classes.iter().map(|tc| tc.id).collect();
        for &c in &classes {
            let file = self.syms.class(c).file;
            if files.contains(&file) && !self.forward_in_block(c) {
                w.unit = file;
                self.forward_class(&mut w, c, true);
            }
        }
        let tops: Vec<FunId> = self.prog.top_funs.all().collect();
        for f in tops {
            let (sym, roots) = self.forward_fun_roots(f);
            let file = self.syms.sym(sym).file;
            if files.contains(&file) {
                w.unit = file;
                for r in roots {
                    self.forward_expr(&mut w, r);
                }
            }
        }
        let vals: Vec<(SymId, TExprId)> = self.prog.top_vals.all().collect();
        for (sym, init) in vals {
            let file = self.syms.sym(sym).file;
            if files.contains(&file) {
                w.unit = file;
                self.forward_expr(&mut w, init);
            }
        }
        // The classes of blocks no level held (a block with no syntax block, a class nothing
        // creates): their own levels, with nothing around them left to test.
        for &c in &classes {
            let file = self.syms.class(c).file;
            if files.contains(&file) && !w.walked.contains_key(&c) {
                w.unit = file;
                self.forward_class(&mut w, c, true);
            }
        }
        self.diags.of_bodies = outer_bodies;
        self.diags.owner = outer_owner;
        if probe {
            for l in &w.lines {
                eprintln!("{}", l);
            }
            // The notes left on a node that names nothing: a kept call's own, where its
            // expansion's root took the node.
            let stale = self.prog.forward.places.keys().filter(|&&e| !matches!(self.prog.expr(e), TExpr::Local(_) | TExpr::CallStatic(..) | TExpr::New(..) | TExpr::NewVia(..) | TExpr::Default(..))).count();
            eprintln!("FORWARD-NOTES\tnotes={}\ton-no-reference={}", self.prog.forward.places.len(), stale);
        }
    }

    /// Whether the class is a block's, or made by an expression, or a member of such a class.
    fn forward_in_block(&self, c: ClassId) -> bool {
        let mut c = c;
        loop {
            let info = self.syms.class(c);
            if info.kind == ClassKind::Anon {
                return true;
            }
            match info.owner {
                Owner::Local => return true,
                Owner::Class(outer) => c = outer,
                _ => return false,
            }
        }
    }

    fn forward_fun_roots(&self, f: FunId) -> (SymId, Vec<TExprId>) {
        let fun = &self.prog.funs[f.idx()];
        (fun.sym, fun.body.iter().chain(fun.defaults.iter().flatten()).copied().collect())
    }

    fn forward_stored(&mut self, w: &mut Walk, def: &Arc<InlineDefinition>) {
        w.stored.push(def.clone());
        let roots: Vec<TExprId> = def.body.iter().chain(def.defaults.iter().flatten()).copied().collect();
        for r in roots {
            self.forward_expr(w, r);
        }
        w.stored.pop();
    }

    /// The body of the class, the program's or a stored body's own.
    fn forward_class_body(&mut self, w: &Walk, c: ClassId) -> Option<TClass> {
        for def in w.stored.iter().rev() {
            if let Some(tc) = def.classes.iter().rfind(|t| t.id == c) {
                return Some(tc.clone());
            }
        }
        let i = self.prog_index.class(&self.prog, c)?;
        Some(self.prog.classes[i].clone())
    }

    /// What the class's definition names where it constructs its superclass `parent`, and
    /// where. A parent written by that class's name: the class, at the parent's type. A parent
    /// written by a type alias of a level that stands for the class: the alias, which scalac's
    /// `New` names in the class's place. Else the constructor call a trait parent brought
    /// (`class D extends T`, `trait T extends Base`): the class, at the definition's own name.
    fn forward_parent(&self, w: &Walk, c: ClassId, parent: ClassId) -> (Named, Option<(FileId, Span)>) {
        let class = Named::Class(parent);
        let info = self.syms.class(c);
        let (file, own) = (info.file, info.span);
        let Some(d) = info.def else { return (class, None) };
        if (file.0 as usize) >= self.asts.len() || !self.program_file(file) {
            return (class, None);
        }
        let ast = self.ast(file);
        let parents = match &ast.def(d).kind {
            DefKindAst::Class(cls) => &cls.parents,
            DefKindAst::Given(g) => &g.parents,
            _ => return (class, None),
        };
        let name = self.syms.class(parent).name;
        if let Some(p) = parents.iter().find(|p| Self::forward_type_head(ast, p.ty) == Some(name)) {
            return (class, Some((file, ast.ty_spans[p.ty.idx()])));
        }
        for p in parents {
            if let ast::TyExpr::Name(n) = ast.ty(p.ty) {
                if let Some(alias) = self.forward_alias_for(w, file, n, name) {
                    return (alias, Some((file, ast.ty_spans[p.ty.idx()])));
                }
            }
        }
        (class, Some((file, own)))
    }

    /// The name a written type's constructor ends with.
    fn forward_type_head(ast: &ast::Ast, t: ast::TyExprId) -> Option<crate::intern::Name> {
        match ast.ty(t) {
            ast::TyExpr::Name(n) | ast::TyExpr::Select(_, n) | ast::TyExpr::Project(_, n) => Some(n),
            ast::TyExpr::Apply(c, _) => Self::forward_type_head(ast, c),
            _ => None,
        }
    }

    /// The alias named `n` of the innermost level under way in `file` that has one, where it
    /// stands for the class named `target`, by itself or through further aliases of the levels.
    fn forward_alias_for(&self, w: &Walk, file: FileId, n: crate::intern::Name, target: crate::intern::Name) -> Option<Named> {
        let ast = self.ast(file);
        let of = |n: crate::intern::Name| {
            w.named
                .iter()
                .filter_map(|(&k, &(level, _))| match k {
                    Named::Alias(f, d) if f == file && w.open.contains(&level) && ast.def(d).name == n => Some((level, d)),
                    _ => None,
                })
                .max_by_key(|&(level, _)| level)
                .map(|(_, d)| d)
        };
        let first = of(n)?;
        let mut seen = vec![first];
        let mut d = first;
        loop {
            let DefKindAst::TypeAlias { rhs: Some(r), .. } = &ast.def(d).kind else { return None };
            let head = Self::forward_type_head(ast, *r)?;
            if head == target {
                return Some(Named::Alias(file, first));
            }
            d = of(head)?;
            if seen.contains(&d) {
                return None;
            }
            seen.push(d);
        }
    }

    /// The arguments of the creation `r` where `r.f` is a projection the inliner reduces: a
    /// creation of a class without initialisation (`isNoInitsRealClass`), `f` a parameter of it.
    fn forward_projection(&mut self, r: TExprId, f: SymId) -> Option<ast::ListRef> {
        let (c, args) = match self.prog.expr(r) {
            TExpr::New(c, args) => (c, args),
            TExpr::NewVia(ctor, args) => match self.syms.sym(ctor).owner {
                Owner::Class(c) => (c, args),
                _ => return None,
            },
            _ => return None,
        };
        (self.is_own_param_accessor(c, f) && self.no_inits(c)).then_some(args)
    }

    /// What the class of a block captures.
    fn forward_captures(&mut self, w: &Walk, c: ClassId) -> Vec<SymId> {
        let captured = |tc: &TClass| tc.ctor_params[..tc.captures.min(tc.ctor_params.len())].to_vec();
        match self.anon_captures.get(&c) {
            Some(held) => held.clone(),
            None => match w.stored.iter().rev().find_map(|def| def.classes.iter().rfind(|t| t.id == c)) {
                Some(tc) => captured(tc),
                None => match self.prog_index.class(&self.prog, c) {
                    Some(i) => captured(&self.prog.classes[i]),
                    None => Vec::new(),
                },
            },
        }
    }

    /// Whether the argument `x` of a creation passes one of the class's `captures`: a read of
    /// the captured definition's own symbol that no code wrote.
    fn forward_capture_arg(&self, captures: &[SymId], x: TExprId) -> bool {
        matches!(self.prog.expr(x), TExpr::Local(s) if captures.contains(&s) && !self.prog.forward.places.contains_key(&x))
    }

    /// Whether scalac's tree holds the block as its constant, and so nothing of it for the
    /// check: `FirstTransform.transformBlock` replaces by its literal (`TreeInfo.constToLiteral`)
    /// a tree of a constant type that is pure (`isPureExpr`). The type is the result's: a
    /// constant as typed (`fold_constant_as_typed`) that neither an ascription nor a plain inline
    /// call's declared type widened (`widened_constant`). The purity is dotty's `exprPurity` of
    /// the block (`expr_purity`; its statements by `stat_purity`: a method, a class, a value
    /// that is no `var` whose initialiser is pure). The transform goes from the leaves up: a
    /// result that is such a block is its literal by then, an `if` its taken branch
    /// (`forward_folded_if`), in a statement's initialiser too.
    fn forward_folded_block(&mut self, w: &Walk, stmts: ast::ListRef, value: TExprId) -> bool {
        use super::elaborate::Purity;
        let res = self.forward_folded_if(value);
        let constant = !self.widened_constant(res)
            && match self.prog.expr(res) {
                TExpr::Block(inner, r) => self.forward_folded_block(w, inner, r),
                TExpr::If(..) => false,
                _ => self.fold_constant_as_typed(res).is_some() && self.expr_purity(res).at_least(Purity::PURE),
            };
        if !constant {
            return false;
        }
        let stats: Vec<TStmt> = self.prog.stmt_list(stmts).to_vec();
        stats.into_iter().all(|st| match st {
            TStmt::Val(v, x) if self.syms.sym(v).kind != SymKind::Var && self.syms.sym(v).mods & mods::MUTABLE == 0 => {
                let x = self.forward_folded_if(x);
                match self.prog.expr(x) {
                    // A creation of a block's class: scalac's tree passes it no capture.
                    TExpr::New(c, args) if self.forward_in_block(c) => {
                        let captures = self.forward_captures(w, c);
                        let args: Vec<TExprId> = self.prog.expr_list(args).to_vec();
                        self.no_inits(c) && args.into_iter().all(|a| self.forward_capture_arg(&captures, a) || self.expr_purity(a).at_least(Purity::PURE))
                    }
                    _ => self.expr_purity(x).at_least(Purity::PURE),
                }
            }
            _ => self.stat_purity(st).at_least(Purity::PURE),
        })
    }

    /// The tree `e`, or what scalac's `FirstTransform.transformIf` leaves of it: the taken
    /// branch of an `if` whose condition is a pure tree of a constant type, which a plain inline
    /// call's result, typed at the method's declared type, is not.
    fn forward_folded_if(&mut self, mut e: TExprId) -> TExprId {
        while let TExpr::If(c, t, els) = self.prog.expr(e) {
            if self.widened_constant(c) {
                break;
            }
            match (self.folded_condition(c), els) {
                (Some(true), _) => e = t,
                (Some(false), Some(x)) => e = x,
                _ => break,
            }
        }
        e
    }

    /// The defaults of a class's parameters. scalac's getters of them are members of the
    /// class's companion: they stand where a written companion stands, else after the class.
    fn forward_ctor_defaults(&mut self, w: &mut Walk, c: ClassId) {
        let Some(tc) = self.forward_class_body(w, c) else { return };
        for d in tc.ctor_defaults.iter().flatten() {
            self.forward_expr(w, *d);
        }
    }

    /// The trees of a class in the order of scalac's template: its parents' calls (each a
    /// reference to what it constructs), then its body's fields, statements, methods,
    /// constructors and classes as the source has them; with `defaults`, its parameters'
    /// defaults after it.
    fn forward_class(&mut self, w: &mut Walk, c: ClassId, defaults: bool) {
        if w.walked.insert(c, ()).is_some() {
            return;
        }
        let Some(tc) = self.forward_class_body(w, c) else { return };
        let prelude: Vec<TStmt> = self.prog.stmt_list(tc.parent_prelude).to_vec();
        for st in prelude {
            self.forward_plain_stmt(w, st);
        }
        // A class's parent is always constructed (`new P(..)`), a trait's where it is given arguments.
        let superclass = self.syms.class(c).superclass;
        if let Some(p) = superclass {
            if w.named.contains_key(&Named::Class(p)) {
                let (named, place) = self.forward_parent(w, c, p);
                self.forward_name(w, named, 0, "parent", place);
            }
        }
        if let Some(l) = tc.parent_args {
            self.forward_list(w, l);
        }
        for init in &tc.init {
            if let TInit::Parent(p, call) = *init {
                let prelude: Vec<TStmt> = self.prog.stmt_list(call.prelude).to_vec();
                for st in prelude {
                    self.forward_plain_stmt(w, st);
                }
                // A trait is constructed where it is given arguments: more than what its
                // initialiser takes of the block (the passing `LambdaLift` makes, after the check).
                let captured = self.forward_class_body(w, p).map_or(0, |t| t.captures);
                if call.args.len as usize > captured && w.named.contains_key(&Named::Class(p)) {
                    let (named, place) = self.forward_parent(w, c, p);
                    self.forward_name(w, named, 0, "parent", place);
                }
                self.forward_list(w, call.args);
            }
        }
        // The body's parts by where each stands; a statement stands with the field before it.
        enum Part {
            Init(TExprId),
            Fun(FunId),
            Class(ClassId),
        }
        let mut parts: Vec<(u32, Part)> = Vec::new();
        let mut at = 0;
        for init in &tc.init {
            match *init {
                TInit::Parent(..) => {}
                TInit::Field(s, e) => {
                    at = self.syms.sym(s).span.start;
                    parts.push((at, Part::Init(e)));
                }
                TInit::Stmt(e) => parts.push((at, Part::Init(e))),
            }
        }
        for &f in tc.methods.iter().chain(&tc.ctors) {
            let s = self.prog.funs[f.idx()].sym;
            parts.push((self.syms.sym(s).span.start, Part::Fun(f)));
        }
        let mut nested: Vec<(u32, ClassId)> = self.syms.class(c).nested.values().map(|&n| (self.syms.class(n).span.start, n)).collect();
        nested.sort();
        for (at, n) in nested {
            if self.syms.class(n).owner == Owner::Class(c) {
                parts.push((at, Part::Class(n)));
            }
        }
        parts.sort_by_key(|p| p.0);
        for (_, part) in parts {
            match part {
                Part::Init(e) => self.forward_expr(w, e),
                Part::Fun(f) => {
                    for r in self.forward_fun_roots(f).1 {
                        self.forward_expr(w, r);
                    }
                }
                Part::Class(n) => self.forward_class(w, n, true),
            }
        }
        if defaults {
            for d in tc.ctor_defaults.iter().flatten() {
                self.forward_expr(w, *d);
            }
        }
    }

    /// A reference to a definition: dotty's `enterReference`.
    fn forward_name(&mut self, w: &mut Walk, named: Named, getter: u32, kind: &'static str, place: Option<(FileId, Span)>) {
        let Some(&(level, at)) = w.named.get(&named) else { return };
        // A reference that code wrote has a place: the typing noted it, or its definition's
        // syntax gives it (a parent's). A node without one is a passing the lowering made (what
        // a class captures, the free arguments of a lifted trait's member, a mixer's forwarder,
        // a trait initialiser's call), which scalac's tree does not hold before `LambdaLift`
        // and `Mixin`: it names nothing.
        let Some(place) = place else {
            if forward_dump() {
                w.lines.push(format!("UNNOTED\tnamed={:?}\tkind={}\tunit={}", named, kind, self.source(w.unit).path));
            }
            return;
        };
        // A default's getter stands after its method, the later parameter's the farther.
        let at = (at.0, at.1 + getter);
        let l = &mut w.levels[level as usize];
        if l.max.map_or(true, |m| m < at) {
            l.max = Some(at);
            l.by = Some(Reference { named, getter, kind, place: Some(place) });
        }
    }

    fn forward_node_place(&self, e: TExprId) -> Option<(FileId, Span)> {
        self.prog.forward.places.get(&e).map(|p| (p.file, p.span))
    }

    fn forward_list(&mut self, w: &mut Walk, l: ast::ListRef) {
        let items: Vec<TExprId> = self.prog.expr_list(l).to_vec();
        for x in items {
            self.forward_expr(w, x);
        }
    }

    /// A statement's trees, nothing tested.
    fn forward_plain_stmt(&mut self, w: &mut Walk, st: TStmt) {
        match st {
            TStmt::Expr(x) | TStmt::Val(_, x) => self.forward_expr(w, x),
            TStmt::Pat(p, x) => {
                self.forward_expr(w, x);
                self.forward_pat(w, p);
            }
            TStmt::Fun(f) => {
                for r in self.forward_fun_roots(f).1 {
                    self.forward_expr(w, r);
                }
            }
        }
    }

    fn forward_cases(&mut self, w: &mut Walk, l: ast::ListRef) {
        let cases: Vec<TCase> = self.prog.case_list(l).to_vec();
        for case in cases {
            self.forward_pat(w, case.pat);
            if let Some(g) = case.guard {
                self.forward_expr(w, g);
            }
            self.forward_expr(w, case.body);
        }
    }

    fn forward_pat(&mut self, w: &mut Walk, p: TPatId) {
        match self.prog.pats[p.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(_, inner) => {
                if let Some(inner) = inner {
                    self.forward_pat(w, inner);
                }
            }
            // A typed pattern's type names nothing in scalac's tree, as a type test's (`forward_node`).
            TPat::Test(_, _, inner) => self.forward_pat(w, inner),
            TPat::Equals(e, _) => self.forward_expr(w, e),
            // A constructor pattern selects the class's companion (`C.unapply`).
            TPat::Class(c, _, _, subs) => {
                let place = self.prog.forward.patterns.get(&p).copied();
                if w.named.contains_key(&Named::Class(c)) {
                    self.forward_name(w, Named::Class(c), 0, "pattern", place);
                } else if let Some(m) = self.forward_holder(c) {
                    // A member class's companion is selected through the block's object that holds it.
                    self.forward_name(w, Named::Term(m), 0, "pattern", place);
                }
                let subs: Vec<TPatId> = self.prog.pat_list(subs).to_vec();
                for s in subs {
                    self.forward_pat(w, s);
                }
            }
            TPat::Alt(subs) => {
                let subs: Vec<TPatId> = self.prog.pat_list(subs).to_vec();
                for s in subs {
                    self.forward_pat(w, s);
                }
            }
            TPat::Seq(items, rest) => {
                let items: Vec<TPatId> = self.prog.pat_list(items).to_vec();
                for s in items {
                    self.forward_pat(w, s);
                }
                if let Some(rest) = rest {
                    self.forward_pat(w, rest);
                }
            }
            TPat::Unapply(_, call, inner) => {
                self.forward_expr(w, call);
                self.forward_pat(w, inner);
            }
        }
    }

    /// A creation: a reference to its class and to what its created type names (dotty's
    /// `transformNew`): the terms the typing noted, and the object a member class is made
    /// through, which the creation passes as its enclosing instance. An anonymous class's
    /// template stands where it is made.
    fn forward_creation(&mut self, w: &mut Walk, c: ClassId, e: TExprId, args: ast::ListRef) {
        let place = self.forward_node_place(e);
        let note = self.prog.forward.places.get(&e).map(|p| (p.file, p.alias, p.parts.clone()));
        // What the creation names is its type as the typer resolved it: an alias in the
        // class's place, a block's a definition of its level.
        match note {
            Some((file, Some(Some(d)), _)) => self.forward_name(w, Named::Alias(file, d), 0, "new", place),
            Some((_, Some(None), _)) => {}
            _ => self.forward_name(w, Named::Class(c), 0, "new", place),
        }
        if let Some((_, _, parts)) = note {
            for s in parts {
                self.forward_name(w, Named::Term(s), 0, "new", place);
            }
        }
        if let Owner::Class(k) = self.syms.class(c).owner {
            if let Some(m) = self.syms.class(k).local_module {
                if matches!(self.prog.expr_list(args).first().map(|&a| self.prog.expr(a)), Some(TExpr::Local(s)) if s == m) {
                    self.forward_name(w, Named::Term(m), 0, "new", place);
                }
            }
        }
        let info = self.syms.class(c);
        let (kind, owner, file, at, name) = (info.kind, info.owner, info.file, info.span.start, info.name);
        if kind == ClassKind::Anon {
            self.forward_class(w, c, true);
        } else if owner == Owner::Local && !w.walked.contains_key(&c) {
            // A class of a block that is no level (the block holds no other definition) stands
            // where it is first made.
            self.forward_syntax_of(w, file);
            let block = w.syntax_at(file, at).map(|d| (d.block, d.def));
            if let Some((block, d)) = block {
                let declared = self.ast(file).def(d).name == name;
                if declared && !w.open.iter().any(|&l| w.levels[l as usize].syntax == (file, block)) {
                    self.forward_class(w, c, true);
                }
            }
        }
    }

    fn forward_expr(&mut self, w: &mut Walk, e: TExprId) {
        let expansion = self.prog.is_expansion(e);
        if expansion {
            w.expansions.push(e);
        }
        self.forward_node(w, e);
        if expansion {
            w.expansions.pop();
        }
    }

    fn forward_node(&mut self, w: &mut Walk, e: TExprId) {
        match self.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::This | TExpr::Super(_) | TExpr::Static(_) | TExpr::Module(_) | TExpr::ClassOf(..) | TExpr::JsImport(_) | TExpr::JsGlobal(..) | TExpr::Null => {}
            TExpr::Local(s) => {
                if w.named.contains_key(&Named::Term(s)) {
                    let place = self.forward_node_place(e);
                    self.forward_name(w, Named::Term(s), 0, "local", place);
                }
            }
            // In the code the inliner copies scalac's tree holds the argument where a creation of
            // a class that initialises nothing is selected for a parameter of its
            // (`InlineReducer.reduceProjection`): the class is not named there, the arguments are.
            TExpr::Field(r, f) if !w.expansions.is_empty() && self.forward_projection(r, f).is_some() => {
                if let Some(args) = self.forward_projection(r, f) {
                    self.forward_list(w, args);
                }
            }
            TExpr::Field(r, _) | TExpr::Unary(_, r) | TExpr::ToStr(r, _) | TExpr::Index(r, _) | TExpr::Spread(r) | TExpr::JsSelect(r, _) | TExpr::Return(r) | TExpr::Throw(r, _) | TExpr::Lambda(_, r) | TExpr::Splice(r) => self.forward_expr(w, r),
            // A type test's and a cast's type is a type tree where scalac's check runs and names
            // nothing there; the test it is lowered to (a singleton's value, an inner class's
            // enclosing instance) is made by phases after the check (`PatternMatcher.addOuterTest`,
            // `TypeTestsCasts`, `Erasure`).
            TExpr::TypeTest(r, _) | TExpr::Cast(r, ..) => self.forward_expr(w, r),
            TExpr::CallStatic(s, args) => {
                if w.named.contains_key(&Named::Term(s)) {
                    let place = self.forward_node_place(e);
                    self.forward_name(w, Named::Term(s), 0, "call", place);
                }
                self.forward_list(w, args);
            }
            TExpr::New(c, args) => {
                self.forward_creation(w, c, e, args);
                self.forward_list(w, args);
            }
            TExpr::NewVia(ctor, args) => {
                let owner = self.syms.sym(ctor).owner;
                if let Owner::Class(c) = owner {
                    self.forward_creation(w, c, e, args);
                }
                self.forward_list(w, args);
            }
            TExpr::Default(of, n, args) => {
                let place = self.forward_node_place(e);
                match of {
                    DefaultOf::Method(m) => self.forward_name(w, Named::Term(m), (n & !DEFAULT_RECEIVER) + 1, "default", place),
                    DefaultOf::Ctor(c) => self.forward_name(w, Named::Class(c), 0, "default", place),
                }
                self.forward_list(w, args);
            }
            TExpr::StrConcat(args) | TExpr::Js(_, args) | TExpr::SeqLit(args) | TExpr::ArrayLit(args) | TExpr::ObjLit(args) => self.forward_list(w, args),
            TExpr::CallMethod(r, _, args) | TExpr::CallClosure(r, args) => {
                self.forward_expr(w, r);
                self.forward_list(w, args);
            }
            // scalac's tree holds a folded `if`'s taken branch alone (`FirstTransform`).
            TExpr::If(c, t, els) => match self.folded_condition(c) {
                Some(true) => self.forward_expr(w, t),
                Some(false) => {
                    if let Some(x) = els {
                        self.forward_expr(w, x);
                    }
                }
                None => {
                    self.forward_expr(w, c);
                    self.forward_expr(w, t);
                    if let Some(x) = els {
                        self.forward_expr(w, x);
                    }
                }
            },
            // `false && y` and `true || y` are their left operand in scalac's tree
            // (`FirstTransform.foldCondition`), where that operand is a literal: an inline call's
            // result is none there, whatever its expansion is.
            TExpr::Prim(op @ (PrimOp::BoolAnd | PrimOp::BoolOr), a, b) => {
                self.forward_expr(w, a);
                let left = if self.prog.is_expansion(a) { None } else { self.folded_condition(a) };
                if left != Some(op == PrimOp::BoolOr) {
                    self.forward_expr(w, b);
                }
            }
            TExpr::While(a, b) | TExpr::Prim(_, a, b) | TExpr::Assign(a, b) => {
                self.forward_expr(w, a);
                self.forward_expr(w, b);
            }
            // A block scalac replaces by its constant holds nothing for its check
            // (`FirstTransform.transformBlock`, `TreeInfo.constToLiteral`).
            TExpr::Block(stmts, value) => {
                if !self.forward_folded_block(w, stmts, value) {
                    self.forward_block(w, stmts, value)
                }
            }
            TExpr::Match(scrut, l) => {
                self.forward_expr(w, scrut);
                self.forward_cases(w, l);
            }
            TExpr::Try(i) => {
                let t = self.prog.tries[i as usize].clone();
                self.forward_expr(w, t.body);
                self.forward_cases(w, t.cases);
                if let Some(f) = t.finalizer {
                    self.forward_expr(w, f);
                }
            }
        }
    }

    /// The symbols a pattern binds, in its order.
    fn forward_binders(&self, p: TPatId, out: &mut Vec<SymId>) {
        match self.prog.pats[p.idx()] {
            TPat::Wildcard | TPat::Equals(..) => {}
            TPat::Bind(s, inner) => {
                out.push(s);
                if let Some(inner) = inner {
                    self.forward_binders(inner, out);
                }
            }
            TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => self.forward_binders(inner, out),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &s in self.prog.pat_list(subs) {
                    self.forward_binders(s, out);
                }
            }
            TPat::Seq(items, rest) => {
                for &s in self.prog.pat_list(items) {
                    self.forward_binders(s, out);
                }
                if let Some(rest) = rest {
                    self.forward_binders(rest, out);
                }
            }
        }
    }

    /// The first definition a typed statement holds: a temporary the typer hoisted is none of
    /// the block's.
    fn forward_stmt_symbol(&self, st: TStmt) -> Option<SymId> {
        let s = match st {
            TStmt::Val(s, _) => s,
            TStmt::Fun(f) => self.prog.funs[f.idx()].sym,
            TStmt::Pat(p, _) => {
                let mut binders = Vec::new();
                self.forward_binders(p, &mut binders);
                *binders.first()?
            }
            TStmt::Expr(_) => return None,
        };
        (self.syms.sym(s).mods & mods::LIFTED == 0).then_some(s)
    }

    /// The file's blocks' definitions by position, each with whether its block stands in a
    /// quote or in an inline method's body.
    fn forward_syntax_of(&mut self, w: &mut Walk, file: FileId) {
        if w.syntax.contains_key(&file) {
            return;
        }
        let mut defs = Vec::new();
        if (file.0 as usize) < self.asts.len() && self.program_file(file) {
            let ast = self.ast(file);
            let mut quotes: Vec<Span> = Vec::new();
            let mut inlines: Vec<Span> = Vec::new();
            for (i, x) in ast.exprs.iter().enumerate() {
                if matches!(x, Expr::Quote(..)) {
                    quotes.push(ast.expr_spans[i]);
                }
            }
            for (i, d) in ast.defs.iter().enumerate() {
                if d.mods & mods::INLINE != 0 && matches!(d.kind, DefKindAst::Fun(_) | DefKindAst::Given(_)) {
                    inlines.push(ast.def_range(ast::DefId(i as u32)));
                }
            }
            let within = |spans: &[Span], at: u32| spans.iter().any(|s| s.start <= at && at < s.end);
            for x in ast.exprs.iter() {
                if let Expr::Block(l) = *x {
                    for (at, s) in ast.stmt_list(l).iter().enumerate() {
                        if let Stmt::Def(d) = *s {
                            let span = ast.def(d).span;
                            defs.push(SyntaxDef {
                                start: span.start,
                                end: span.end.max(span.start + 1),
                                block: l,
                                at: at as u32,
                                def: d,
                                quoted: within(&quotes, span.start),
                                inline_body: within(&inlines, span.start),
                            });
                        }
                    }
                }
            }
            defs.sort_by_key(|d| d.start);
        }
        w.syntax.insert(file, defs);
    }

    /// The syntax block's definition a symbol was made of: the one at its name's position, of
    /// its name (a pattern definition holds its variables, whatever their names).
    fn forward_symbol_place(&mut self, w: &mut Walk, s: SymId) -> Option<(FileId, ast::ListRef, u32)> {
        let info = self.syms.sym(s);
        let (file, pos, name) = (info.file, info.span.start, info.name);
        self.forward_syntax_of(w, file);
        let d = w.syntax_at(file, pos)?;
        let (block, at, def) = (d.block, d.at, d.def);
        let def = self.ast(file).def(def);
        let pattern = matches!(def.kind, DefKindAst::Val { pat: Some(_), .. });
        (pattern || def.name == name).then_some((file, block, at))
    }

    fn forward_line(&self, file: FileId, pos: u32) -> usize {
        crate::source::locate(&self.source(file).text, pos as usize).0
    }

    /// A block: a level where its syntax block is found, its statements in the syntax's order,
    /// each value tested as the walk passes it.
    fn forward_block(&mut self, w: &mut Walk, stmts: ast::ListRef, value: TExprId) {
        let typed: Vec<TStmt> = self.prog.stmt_list(stmts).to_vec();
        let mut found = None;
        for &st in &typed {
            if let Some(s) = self.forward_stmt_symbol(st) {
                if let Some(place) = self.forward_symbol_place(w, s) {
                    found = Some((place, s));
                    break;
                }
            }
        }
        let Some(((file, block, _), first)) = found else {
            // No syntax block: no level.
            if forward_dump() {
                let value_stmt = typed.iter().find(|st| match **st {
                    TStmt::Val(s, _) => {
                        let info = self.syms.sym(s);
                        info.mods & (mods::LAZY | mods::LIFTED) == 0 && matches!(info.kind, SymKind::Val | SymKind::Var)
                    }
                    TStmt::Pat(p, _) => {
                        let mut binders = Vec::new();
                        self.forward_binders(p, &mut binders);
                        !binders.is_empty()
                    }
                    _ => false,
                });
                if let Some(s) = value_stmt.and_then(|&st| self.forward_stmt_symbol(st)) {
                    let info = self.syms.sym(s);
                    w.lines.push(format!("UNMATCHED\tvalue={}\tat={}:{}\tstatements={}", self.name_str(info.name), self.source(info.file).path, self.forward_line(info.file, info.span.start), typed.len()));
                }
            }
            for st in typed {
                self.forward_plain_stmt(w, st);
            }
            self.forward_expr(w, value);
            return;
        };
        let (quoted, inline_body) = {
            let info = self.syms.sym(first);
            let (f, at) = (info.file, info.span.start);
            w.syntax_at(f, at).map_or((false, false), |d| (d.quoted, d.inline_body))
        };
        let level = w.levels.len() as u32;
        w.levels.push(Level { max: None, by: None, quoted, inline_body, syntax: (file, block) });
        w.open.push(level);
        let ast = self.ast(file);
        let syntax: Vec<Stmt> = ast.stmt_list(block).to_vec();
        // Each typed statement's ordinal: its definition's place in the syntax block; an
        // expression's is the next expression statement's after the last place taken; what has
        // no place of its own (a hoisted temporary) stands with what follows it.
        let mut places: Vec<Option<u32>> = Vec::with_capacity(typed.len());
        let mut own: Vec<bool> = Vec::with_capacity(typed.len());
        let mut last: Option<u32> = None;
        for &st in &typed {
            let at = match st {
                TStmt::Expr(_) => {
                    let from = last.map_or(0, |l| l as usize + 1);
                    (from..syntax.len().saturating_sub(1)).find(|&i| matches!(syntax[i], Stmt::Expr(_))).map(|i| i as u32)
                }
                _ => match self.forward_stmt_symbol(st).and_then(|s| self.forward_symbol_place(w, s)) {
                    Some((f, b, at)) if f == file && b == block && last.map_or(true, |l| l <= at) => Some(at),
                    _ => None,
                },
            };
            if at.is_some() {
                last = at;
            }
            own.push(at.is_some() && !matches!(st, TStmt::Expr(_)));
            places.push(at);
        }
        let mut next = None;
        for p in places.iter_mut().rev() {
            match *p {
                Some(at) => next = Some(at),
                None => *p = next,
            }
        }
        // The definitions of the level. A typed statement's own symbols (an expansion's copy of
        // the block has its own), then what leaves no typed statement: an inline method, a
        // class, an object's and a given's class.
        let mut subs: FxMap<u32, u32> = FxMap::default();
        let mut ordinals: Vec<Option<Ordinal>> = Vec::with_capacity(typed.len());
        for (i, &st) in typed.iter().enumerate() {
            let Some(at) = places[i] else {
                ordinals.push(None);
                continue;
            };
            let sub = subs.entry(at).or_insert(0);
            let o = (at, *sub);
            *sub += 1;
            if !own[i] {
                ordinals.push(None);
                continue;
            }
            ordinals.push(Some(o));
            match st {
                TStmt::Val(s, _) => {
                    w.named.insert(Named::Term(s), (level, o));
                }
                TStmt::Fun(f) => {
                    let s = self.prog.funs[f.idx()].sym;
                    w.named.insert(Named::Term(s), (level, o));
                }
                TStmt::Pat(p, _) => {
                    let mut binders = Vec::new();
                    self.forward_binders(p, &mut binders);
                    for s in binders {
                        w.named.insert(Named::Term(s), (level, o));
                    }
                }
                TStmt::Expr(_) => {}
            }
        }
        let mut untyped: Vec<(u32, Option<SymId>, Option<ClassId>)> = Vec::new();
        for (at, s) in syntax.iter().enumerate() {
            let Stmt::Def(d) = *s else { continue };
            let at = at as u32;
            let (sym, class) = match &ast.def(d).kind {
                DefKindAst::Fun(_) | DefKindAst::Given(_) | DefKindAst::Class(_) => (self.def_syms.get(file.0 as usize, &d).copied(), self.def_classes.get(file.0 as usize, &d).copied()),
                // A copy of the block names the alias by the same definition: the level under
                // way takes it.
                DefKindAst::TypeAlias { .. } => {
                    if !quoted {
                        w.named.insert(Named::Alias(file, d), (level, (at, 0)));
                    }
                    (None, None)
                }
                _ => (None, None),
            };
            if let Some(s) = sym {
                w.named.entry(Named::Term(s)).or_insert((level, (at, 0)));
            }
            if let Some(c) = class {
                // In a quote's copy a creation is no reference to a class of the quote's block:
                // scalac's check does not see it there (the reason is not established).
                if !quoted {
                    w.named.entry(Named::Class(c)).or_insert((level, (at, 0)));
                }
                // An object's and a structural given's value.
                let module = self.syms.class(c).local_module;
                if let Some(m) = module {
                    w.named.entry(Named::Term(m)).or_insert((level, (at, 0)));
                }
            }
            if sym.is_some() || class.is_some() {
                untyped.push((at, sym, class));
            }
        }
        // A class of the level and the object of its name the level writes, its companion.
        let mut companions: Vec<(ClassId, ClassId)> = Vec::new();
        for &(_, _, class) in &untyped {
            let Some(c) = class else { continue };
            if self.syms.class(c).kind == ClassKind::Object {
                continue;
            }
            let companion = self.syms.class(c).companion;
            if let Some(o) = companion.filter(|o| untyped.iter().any(|u| u.2 == Some(*o))) {
                companions.push((c, o));
            }
        }
        // The walk, statement by statement.
        let mut i = 0;
        for at in 0..syntax.len() as u32 {
            while i < typed.len() && places[i].map_or(true, |p| p <= at) {
                let st = typed[i];
                self.forward_plain_stmt(w, st);
                if let (Some(o), Some(place)) = (ordinals[i], places[i]) {
                    self.forward_value(w, level, o, st, file, block, place, &syntax);
                }
                i += 1;
            }
            for &(_, sym, class) in untyped.iter().filter(|u| u.0 == at) {
                // An inline method's stored body, where the method stands.
                if let Some(s) = sym {
                    if let Some(def) = self.inline_definitions.get(&s).cloned() {
                        self.forward_stored(w, &def);
                    }
                }
                if let Some(c) = class {
                    if let Some(&(k, _)) = companions.iter().find(|p| p.1 == c) {
                        self.forward_ctor_defaults(w, k);
                    }
                    let own = !companions.iter().any(|p| p.0 == c);
                    self.forward_class(w, c, own);
                }
            }
        }
        while i < typed.len() {
            let st = typed[i];
            self.forward_plain_stmt(w, st);
            i += 1;
        }
        // The block's value stands outside the level's statements.
        self.forward_expr(w, value);
        w.open.pop();
    }

    /// The test of a value's definition as the walk passes it: dotty's `transformValDef`.
    #[allow(clippy::too_many_arguments)]
    fn forward_value(&mut self, w: &mut Walk, level: u32, o: Ordinal, st: TStmt, file: FileId, block: ast::ListRef, at: u32, syntax: &[Stmt]) {
        // A value of the block as the syntax has it: a `val` or `var` statement, strict.
        let Some(Stmt::Def(d)) = syntax.get(at as usize).copied() else { return };
        let ast = self.ast(file);
        let def = ast.def(d);
        if !matches!(def.kind, DefKindAst::Val { .. }) || def.mods & mods::LAZY != 0 {
            return;
        }
        let (value, value_at, value_sym) = match st {
            TStmt::Val(s, _) => {
                let info = self.syms.sym(s);
                if info.mods & (mods::LAZY | mods::LIFTED) != 0 || !matches!(info.kind, SymKind::Val | SymKind::Var) {
                    return;
                }
                // `val _ = e` is no definition scalac tests.
                let name = self.name_str(info.name);
                if name == "_" || name.starts_with("unnamed$") {
                    return;
                }
                let start = info.span.start;
                if self.forward_symbol_place(w, s) != Some((file, block, at)) {
                    return;
                }
                (name, start, Some(s))
            }
            TStmt::Pat(p, _) => {
                let mut binders = Vec::new();
                self.forward_binders(p, &mut binders);
                // A tuple of names and wildcards against a value has a holder whatever it binds
                // (`makePatDef`'s variables count the wildcards); any other pattern by its variables.
                let tuple = match def.kind {
                    DefKindAst::Val { pat: Some(pat), rhs: Some(rhs), ty: None } => ast.patdef_tuple(pat, rhs).is_some(),
                    _ => false,
                };
                match binders[..] {
                    [] => return,
                    // One variable: the value is the variable's.
                    [s] if !tuple => (self.name_str(self.syms.sym(s).name), self.syms.sym(s).span.start, Some(s)),
                    // Several: the holder's, which scalac names by a fresh name of its lowering.
                    _ => (self.name_str(self.syms.sym(binders[0]).name), def.span.start, None),
                }
            }
            _ => return,
        };
        let l = &w.levels[level as usize];
        let (Some(max), Some(by)) = (l.max, l.by) else { return };
        if o > max {
            return;
        }
        let (quoted, inline_body) = (l.quoted, l.inline_body);
        let (named, named_file, named_at) = match by.named {
            Named::Term(s) => {
                let info = self.syms.sym(s);
                let name = self.name_str(info.name);
                (if by.getter > 0 { format!("{}$default${}", name, by.getter) } else { name }, info.file, info.span.start)
            }
            Named::Class(c) => {
                let info = self.syms.class(c);
                (self.name_str(info.name), info.file, info.span.start)
            }
            Named::Alias(f, d) => {
                let def = self.ast(f).def(d);
                (self.name_str(def.name), f, def.span.start)
            }
        };
        let same = matches!((by.named, value_sym), (Named::Term(s), Some(v)) if s == v);
        let value_line = self.forward_line(file, value_at);
        let named_line = self.forward_line(named_file, named_at);
        if forward_dump() {
            w.lines.push(format!(
                "FORWARD\tvalue={}\tnamed={}\tvline={}\tnline={}\tsame={}\tkind={}\tplace={}\tunit={}",
                value,
                named,
                value_line,
                named_line,
                same as u8,
                by.kind,
                by.place.map_or("none".to_string(), |(f, s)| format!("{}:{}", self.source(f).path, s.start)),
                self.source(w.unit).path
            ));
            if by.place.is_none() {
                w.lines.push(format!("NOPLACE\tvalue={}\tnamed={}\tkind={}\tunit={}", value, named, by.kind, self.source(w.unit).path));
            }
            if !self.prog.forward.units.contains(&w.unit) {
                w.lines.push(format!("OFFUNIT\tvalue={}\tnamed={}\tunit={}", value, named, self.source(w.unit).path));
            }
        }
        // An inline method's body is told where its definition stores it; its copies say the same.
        if !w.expansions.is_empty() && inline_body && !quoted {
            return;
        }
        let Some((place_file, place)) = by.place else { return };
        if w.told.iter().any(|&(f, t)| f == place_file && t.start <= place.end && place.start <= t.end) {
            return;
        }
        w.told.push((place_file, place));
        let mut msg = if same {
            format!("forward reference to {} extends over the definition of {} (on line {})", named, value, value_line)
        } else {
            format!("forward reference to {} (defined on line {}) extends over the definition of {} (on line {})", named, named_line, value, value_line)
        };
        // A quote's copy has no definition that told it: at the macro's call, as scalac's
        // inline trace places it, the call's point.
        let call = match w.expansions.first() {
            Some(&root) if quoted => self.prog.span_of(root),
            _ => None,
        };
        match call {
            Some((call_file, call_span)) => {
                let path = self.source(place_file).path.clone();
                msg.push_str(&format!("\n  inlined from {}:{}", path, self.forward_line(place_file, place.start)));
                let point = self.forward_call_point(call_file, call_span);
                self.diags.late_error_of(call_file, Span { start: point, end: call_span.end.max(point) }, msg, PHASE_FORWARD_DEP_CHECKS);
                if let Some(d) = self.diags.items.last_mut() {
                    d.inlined_from = Some((place_file, place));
                }
            }
            None => self.diags.late_error_of(place_file, place, msg, PHASE_FORWARD_DEP_CHECKS),
        }
    }

    /// The point of the call written at `span`, where scalac's header places a message of its
    /// expansion: the parenthesis of its first argument list, else its last selector's name.
    fn forward_call_point(&self, file: FileId, span: Span) -> u32 {
        let text = self.source(file).text.as_bytes();
        let (from, to) = (span.start as usize, (span.end as usize).min(text.len()));
        let mut depth = 0usize;
        let mut name = from;
        for (i, &b) in text.iter().enumerate().take(to).skip(from) {
            match b {
                b'[' => depth += 1,
                b']' if depth > 0 => depth -= 1,
                b'(' if depth == 0 => return i as u32,
                b'.' if depth == 0 => name = i + 1,
                _ => {}
            }
        }
        name as u32
    }
}
