//! The navigation index of the language server (`teq compiler watch --check --index`): what each name
//! of the program refers to, recorded as the typer resolves it, and the queries over it
//! (docs/TARGETS.md, "The language server").
//!
//! A record is a span of a program file with its target: a symbol, a class, a type alias or a
//! type parameter, and how the name uses it. Records go to a journal while the typer runs, so
//! that an attempt it abandons (an extension tried and rejected, an overloaded alternative, a
//! conversion) takes its records back with it (`index_mark`, `index_drop`); after the phase the
//! journal is sorted into per-file tables where the last record of a span wins.
//!
//! What a retype of a file types again (its bodies) records again; what it leaves alone (the
//! signatures, parents, aliases and imports, resolved once and cached) keeps its records, moved
//! to the new positions of the names `shape::compare_moving` pairs. A record of the file whose
//! name was not paired stood in a body and is dropped for the new one.
//!
//! Declarations are not recorded: a member's, a class's and an alias's position is in the
//! symbol table, a type parameter's here, and a local's comes with its `Def` record, made when
//! the local is, so that the locals of a body typed again replace the old ones.

use super::loader::occurrences::{DocIndex, DocTarget};
use super::resolve::{TermRef, TypeRef};
use super::Worker;
use crate::ast::{self, DefKind, Expr, ExprId, TyExprId};
use crate::index::{location, Lines};
use crate::intern::{FxMap, Name};
use crate::lsp::json::{obj, Json};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::{TExpr, TExprId, TPat, TPatId};
use crate::types::*;
use crate::watch::Positions;
use std::path::PathBuf;

/// Counts the typings the index settled in this process, so that a completion item names the
/// program it was offered from and its resolve tells whether the program changed since.
pub(super) static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Target {
    Sym(SymId),
    Class(ClassId),
    Alias(AliasId),
    TParam(TParamId),
    /// An expression node of a body, for completion's receivers (`Kind::Node`): kept apart from
    /// the records of names, in `Index::nodes`.
    Node(TExprId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A value, a local or an object named.
    Ref,
    /// A method called, with or without arguments, or an inline method expanded.
    Call,
    Type,
    /// A name of an import or export clause.
    Import,
    /// The class or extractor of a constructor pattern.
    Pattern,
    NamedArg,
    /// The declaration of a local.
    Def,
    /// An expression typed in a body, at its span: the receiver a completion finds by its span.
    Node,
    /// A name typed in a body where a function is expected, at the name's span, its target the
    /// node: what a completion of the name inserts without arguments.
    Function,
}

/// Where the index's journal and the unused-import marks stand (`Worker::index_mark`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Recorded {
    records: usize,
    marks: usize,
}

/// What an attempt recorded, taken out (`Worker::index_take`).
pub(super) struct Taken {
    records: Vec<(FileId, Record)>,
    marks: Vec<(super::unused::Sel, u8)>,
}

#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub span: Span,
    pub target: Target,
    pub kind: Kind,
}

#[derive(Default)]
pub struct Index {
    journal: Vec<(FileId, Record)>,
    /// Per program file its records, by span.
    files: Vec<Vec<Record>>,
    /// Per program file the expression nodes its bodies made (`Kind::Node`), by span, the last
    /// of a span the latest: a retype of the file replaces them.
    nodes: Vec<Vec<(Span, TExprId)>>,
    /// Per program file the names its bodies typed where a function was expected
    /// (`Kind::Function`), by span: a retype of the file replaces them.
    functions: Vec<Vec<Span>>,
    /// Per file, as a bit per expression of its tree, the receivers of its selections: the nodes
    /// recorded, made on the first node a typing of the file records, dropped when the typing is
    /// settled.
    receivers: Vec<Option<Vec<u64>>>,
    /// Where each type parameter of the program is declared.
    tparams: FxMap<TParamId, (FileId, Span)>,
    /// The inline expansions by the expression each gave: the call of the inline method.
    pub(super) expansions: FxMap<TExprId, SymId>,
    /// The implicit conversions the typer applied, by the node of each application: a
    /// conversion no source wrote, as against a written call of the conversion's method.
    pub(super) conversions: FxMap<TExprId, ()>,
    /// The prefix operators the typer read as a builtin's identity (`+n` of a number), by file
    /// and start: their node is the operand's, which `index_expr` would take for a member's call.
    pub(super) identity_prefixes: FxMap<(FileId, u32), ()>,
    /// Per inline method what the check of its definition recorded of its body, which each
    /// expansion by substitution records again as a retyped expansion records its own.
    bodies: FxMap<SymId, Vec<(FileId, Record)>>,
    /// Per file the first symbol, class and alias its latest typing made: the locals before
    /// them belong to bodies typed again since.
    fresh: FxMap<FileId, (u32, u32, u32)>,
    /// Where each local and parameter is declared, from its `Def` record, which a retype moves.
    locals: FxMap<SymId, (FileId, Span)>,
    /// The parameters of an extension group's methods that stand for the group's one parameter,
    /// each with the first of them.
    same_param: FxMap<SymId, SymId>,
    /// The method parameters by the position they are declared at, while their file is typed.
    params_at: FxMap<(FileId, u32), SymId>,
    /// Per member the members that override it, as the override check verified them.
    overrides: FxMap<SymId, Vec<SymId>>,
    /// Built on the first query after a build: records by target.
    inverse: Option<FxMap<Target, Vec<(FileId, Record)>>>,
    /// The names of the std and the class path a completion offers with an import, built on the
    /// first completion that wants them (`typer::complete`).
    pub(super) catalogue: Option<Box<super::complete::Catalogue>>,
    /// The imports of the blocks of program files as completion resolved them, by the file and
    /// the start of the statement: resolved once per typing of the file, since resolving one
    /// enters the values and exclusions it names.
    pub(super) block_imports: FxMap<(FileId, u32), Vec<super::ResolvedImport>>,
    /// The files recorded into since the tables were last ordered.
    unordered: Vec<FileId>,
    /// The std files whose documents a query reached, by slot: their bodies typed and recorded
    /// (`Worker::demand_std_document`), once per typing of the program.
    demanded: Vec<bool>,
    /// Per file how its names are recorded (`Worker::recording`): 0 not known yet, 1 with the
    /// expressions completion reads (a program file, and in a language server's session a std
    /// file, whose document completes as a program file does), 2 not at all (a library body).
    program: Vec<u8>,
}

impl Index {
    /// Another worker's index at the fork: the tables its typing reads as this one has them (the
    /// type parameters', the locals' and the parameters' declarations, the extension groups'
    /// parameters, the inline methods' recorded bodies, the overrides, which files are the
    /// program's), its journal its own: what it records goes to the merge (`absorb`).
    pub fn attach(&self) -> Index {
        Index {
            journal: Vec::new(),
            files: Vec::new(),
            nodes: Vec::new(),
            functions: Vec::new(),
            receivers: Vec::new(),
            tparams: self.tparams.clone(),
            expansions: self.expansions.clone(),
            conversions: self.conversions.clone(),
            identity_prefixes: FxMap::default(),
            bodies: self.bodies.clone(),
            fresh: self.fresh.clone(),
            locals: self.locals.clone(),
            same_param: self.same_param.clone(),
            params_at: self.params_at.clone(),
            overrides: self.overrides.clone(),
            inverse: None,
            catalogue: None,
            block_imports: FxMap::default(),
            unordered: Vec::new(),
            demanded: Vec::new(),
            program: self.program.clone(),
        }
    }

    pub fn journal_len(&self) -> u32 {
        self.journal.len() as u32
    }

    /// How many std files a query demanded (`Worker::demand_std_document`).
    pub fn demanded(&self) -> usize {
        self.demanded.iter().filter(|&&d| d).count()
    }

    /// The records of the journal, for the merge to place by the work items that made them.
    pub fn take_journal(&mut self) -> Vec<(FileId, Record)> {
        std::mem::take(&mut self.journal)
    }

    pub fn set_journal(&mut self, journal: Vec<(FileId, Record)>) {
        self.journal = journal;
    }

    /// Takes over another worker's tables at the merge, where this one has no entry of its own;
    /// its records come with the work items (`merge_workers`).
    pub fn absorb(&mut self, o: Index) {
        fn union<K: Eq + std::hash::Hash, V>(mine: &mut FxMap<K, V>, theirs: FxMap<K, V>) {
            for (k, v) in theirs {
                mine.entry(k).or_insert(v);
            }
        }
        union(&mut self.tparams, o.tparams);
        union(&mut self.expansions, o.expansions);
        union(&mut self.conversions, o.conversions);
        union(&mut self.bodies, o.bodies);
        union(&mut self.fresh, o.fresh);
        union(&mut self.locals, o.locals);
        union(&mut self.same_param, o.same_param);
        union(&mut self.params_at, o.params_at);
        for (base, overriding) in o.overrides {
            let mine = self.overrides.entry(base).or_default();
            for m in overriding {
                if !mine.contains(&m) {
                    mine.push(m);
                }
            }
        }
        if self.program.len() < o.program.len() {
            self.program.resize(o.program.len(), 0);
        }
        for (mine, theirs) in self.program.iter_mut().zip(o.program) {
            if *mine == 0 {
                *mine = theirs;
            }
        }
    }

    /// The index under the merge's ids (`merge.rs`): every record's target and every table's
    /// symbols, classes, aliases, type parameters and expressions.
    pub fn remap(&mut self, sym: &dyn Fn(SymId) -> SymId, class: &dyn Fn(ClassId) -> ClassId, alias: &dyn Fn(AliasId) -> AliasId, tparam: &dyn Fn(TParamId) -> TParamId, expr: &dyn Fn(TExprId) -> TExprId) {
        let target = |t: Target| match t {
            Target::Sym(s) => Target::Sym(sym(s)),
            Target::Class(c) => Target::Class(class(c)),
            Target::Alias(a) => Target::Alias(alias(a)),
            Target::TParam(p) => Target::TParam(tparam(p)),
            Target::Node(e) => Target::Node(expr(e)),
        };
        let records = |rs: &mut [Record]| {
            for r in rs {
                r.target = target(r.target);
            }
        };
        for (_, r) in self.journal.iter_mut() {
            r.target = target(r.target);
        }
        for f in self.files.iter_mut() {
            records(f);
        }
        for f in self.nodes.iter_mut() {
            for (_, e) in f.iter_mut() {
                *e = expr(*e);
            }
        }
        self.tparams = std::mem::take(&mut self.tparams).into_iter().map(|(p, v)| (tparam(p), v)).collect();
        self.expansions = std::mem::take(&mut self.expansions).into_iter().map(|(e, s)| (expr(e), sym(s))).collect();
        self.conversions = std::mem::take(&mut self.conversions).into_iter().map(|(e, ())| (expr(e), ())).collect();
        self.bodies = std::mem::take(&mut self.bodies)
            .into_iter()
            .map(|(s, mut rs)| {
                for (_, r) in rs.iter_mut() {
                    r.target = target(r.target);
                }
                (sym(s), rs)
            })
            .collect();
        self.locals = std::mem::take(&mut self.locals).into_iter().map(|(s, v)| (sym(s), v)).collect();
        self.same_param = std::mem::take(&mut self.same_param).into_iter().map(|(a, b)| (sym(a), sym(b))).collect();
        for s in self.params_at.values_mut() {
            *s = sym(*s);
        }
        self.overrides = std::mem::take(&mut self.overrides).into_iter().map(|(s, ms)| (sym(s), ms.into_iter().map(sym).collect())).collect();
        self.inverse = None;
    }

    /// The locals and type parameters declared in `file`, each by the start of its name, and the
    /// first symbol and class of the file's latest typing.
    pub(super) fn declarations_in(&self, file: FileId, locals: &mut Vec<(u32, SymId)>, tparams: &mut Vec<(u32, TParamId)>) -> (u32, u32) {
        locals.extend(self.locals.iter().filter(|(_, (f, _))| *f == file).map(|(&s, &(_, span))| (span.start, s)));
        tparams.extend(self.tparams.iter().filter(|(_, (f, _))| *f == file).map(|(&p, &(_, span))| (span.start, p)));
        self.fresh.get(&file).map_or((0, 0), |&(s, c, _)| (s, c))
    }

    /// The bytes the index's tables hold.
    pub fn held(&self) -> usize {
        use crate::held::{array, owners, table};
        array(&self.journal)
            + owners(&self.files, array)
            + owners(&self.nodes, array)
            + owners(&self.functions, array)
            + array(&self.receivers)
            + self.receivers.iter().flatten().map(array).sum::<usize>()
            + table(&self.tparams)
            + table(&self.expansions)
            + table(&self.conversions)
            + table(&self.bodies)
            + self.bodies.values().map(array).sum::<usize>()
            + table(&self.fresh)
            + table(&self.locals)
            + table(&self.same_param)
            + table(&self.params_at)
            + table(&self.overrides)
            + self.overrides.values().map(array).sum::<usize>()
            + self.inverse.as_ref().map_or(0, |inverse| table(inverse) + inverse.values().map(array).sum::<usize>())
            + array(&self.unordered)
            + array(&self.demanded)
            + array(&self.program)
            + self.catalogue.as_ref().map_or(0, |c| c.held())
            + table(&self.block_imports)
            + self.block_imports.values().map(array).sum::<usize>()
    }
}

/// The session's view of the program files for the queries: their canonical paths by file, a
/// std file its document's (`attach::std_document_path`), library files having none.
pub struct Files {
    pub paths: Vec<Option<PathBuf>>,
    pub positions: Positions,
}

impl Files {
    pub(super) fn ids_of(&self, path: &std::path::Path) -> Vec<FileId> {
        self.paths.iter().enumerate().filter(|(_, p)| p.as_deref() == Some(path)).map(|(i, _)| FileId(i as u32)).collect()
    }

    pub(super) fn path(&self, f: FileId) -> Option<&PathBuf> {
        self.paths.get(f.0 as usize).and_then(Option::as_ref)
    }
}

/// The offset a query at a position asks about, of the queries a library's document answers.
fn document_offset(q: &crate::index::Query) -> Option<u32> {
    use crate::index::Query;
    match q {
        Query::Definition(_, o) | Query::References(_, o, _) | Query::Hover(_, o) | Query::Implementation(_, o) => Some(*o),
        Query::PrepareCall(_, o) | Query::Incoming(_, o) | Query::Outgoing(_, o) => Some(*o),
        _ => None,
    }
}

/// The names an expression's value comes out of: itself where it is one, else its branches'
/// (an `if`'s, a `match`'s cases', a `try`'s body and cases', a block's result, inside
/// parentheses or `: @unchecked`).
pub(super) fn result_names(ast: &ast::Ast, e: ExprId, out: &mut Vec<ExprId>) {
    match ast.expr(e) {
        Expr::Ident(_) | Expr::Select(..) => out.push(e),
        Expr::Parens(x) | Expr::Unchecked(x) => result_names(ast, x, out),
        Expr::Try(i) => {
            let t = ast.try_expr(i);
            result_names(ast, t.body, out);
            for c in ast.case_list(t.cases) {
                result_names(ast, c.body, out);
            }
        }
        Expr::If(_, t, Some(f)) | Expr::InlineIf(_, t, Some(f)) => {
            result_names(ast, t, out);
            result_names(ast, f, out);
        }
        Expr::Match(_, cases) | Expr::InlineMatch(_, cases) => {
            for c in ast.case_list(cases) {
                result_names(ast, c.body, out);
            }
        }
        Expr::Block(stmts) => {
            if let Some(&ast::Stmt::Expr(r)) = ast.stmt_list(stmts).last() {
                result_names(ast, r, out);
            }
        }
        _ => {}
    }
}

/// Lines of texts, built as the answer needs them.
struct LineCache<'t> {
    by_file: FxMap<FileId, Lines<'t>>,
}

impl Recorded {
    /// Where the index's journal stood.
    pub(super) fn records(&self) -> usize {
        self.records
    }
}

impl<'a> Worker<'a> {
    /// Takes back what was recorded since `mark`, but the records at the positions `kept`
    /// (ascending), which stay in their order: what work done on demand for an older definition
    /// recorded inside the attempt (`state.rs`).
    pub(super) fn index_drop_except(&mut self, mark: Recorded, kept: &[usize]) {
        if let Some(ix) = self.index.as_mut() {
            let mut k = mark.records;
            for &i in kept {
                if i >= mark.records && i < ix.journal.len() {
                    ix.journal.swap(k, i);
                    k += 1;
                }
            }
            ix.journal.truncate(k);
        }
        self.unused.drop_since(mark.marks);
    }

    /// `index_take` but for the records at the positions `kept`, which stay.
    pub(super) fn index_take_except(&mut self, mark: Recorded, kept: &[usize]) -> Taken {
        let records = match self.index.as_mut() {
            Some(ix) if mark.records < ix.journal.len() => {
                let tail = ix.journal.split_off(mark.records);
                let mut out = Vec::new();
                for (i, r) in tail.into_iter().enumerate() {
                    if kept.binary_search(&(mark.records + i)).is_ok() {
                        ix.journal.push(r);
                    } else {
                        out.push(r);
                    }
                }
                out
            }
            _ => Vec::new(),
        };
        Taken { records, marks: self.unused.take_since(mark.marks) }
    }

    /// Where the journal stands, and the unused-import marks' (`unused.rs`), which an
    /// abandoned attempt takes back with its records.
    #[inline]
    pub(super) fn index_mark(&self) -> Recorded {
        Recorded { records: self.index.as_ref().map_or(0, |ix| ix.journal.len()), marks: self.unused.journal_len() }
    }

    /// Takes back what was recorded since `mark`: the attempt was abandoned.
    #[inline]
    pub(super) fn index_drop(&mut self, mark: Recorded) {
        if let Some(ix) = self.index.as_mut() {
            ix.journal.truncate(mark.records);
        }
        self.unused.drop_since(mark.marks);
    }

    /// Takes out what was recorded since `mark`, for `index_put_back` to restore should the
    /// attempt be kept after all.
    pub(super) fn index_take(&mut self, mark: Recorded) -> Taken {
        let records = self.index.as_mut().map_or_else(Vec::new, |ix| ix.journal.split_off(mark.records));
        Taken { records, marks: self.unused.take_since(mark.marks) }
    }

    /// Takes back the records made since `mark` at `span` of `file`, keeping the marks and what
    /// was recorded elsewhere meanwhile: what an annotation typed for the marks alone recorded
    /// (`Worker::mark_annotation`), and not a definition it completed on the way.
    pub(super) fn index_drop_records_within(&mut self, mark: Recorded, file: FileId, span: Span) {
        if let Some(ix) = self.index.as_mut() {
            let within = |(f, r): &(FileId, Record)| *f == file && r.span.start >= span.start && r.span.end <= span.end;
            let mut k = mark.records;
            for i in mark.records..ix.journal.len() {
                if !within(&ix.journal[i]) {
                    ix.journal.swap(k, i);
                    k += 1;
                }
            }
            ix.journal.truncate(k);
        }
    }

    /// Puts back, of `taken` (what `index_take(base)` took out), what was recorded between
    /// `from` and `to`: the typings a member's retry reuses keep their records.
    pub(super) fn index_put_back_between(&mut self, taken: &Taken, base: Recorded, from: Recorded, to: Recorded) {
        if let Some(ix) = self.index.as_mut() {
            ix.journal.extend_from_slice(&taken.records[from.records - base.records..to.records - base.records]);
        }
        self.unused.put_back(taken.marks[from.marks - base.marks..to.marks - base.marks].to_vec());
    }

    /// Whether what was recorded between `from` and `to` lies within `taken`, what
    /// `index_take(base)` took out.
    pub(super) fn index_holds(&self, taken: &Taken, base: Recorded, from: Recorded, to: Recorded) -> bool {
        from.records >= base.records
            && from.records <= to.records
            && to.records - base.records <= taken.records.len()
            && from.marks >= base.marks
            && from.marks <= to.marks
            && to.marks - base.marks <= taken.marks.len()
    }

    pub(super) fn index_put_back(&mut self, taken: Taken) {
        if let Some(ix) = self.index.as_mut() {
            ix.journal.extend(taken.records);
        }
        self.unused.put_back(taken.marks);
    }

    /// Keeps what was recorded since `mark` by the check of `sym`'s definition, for its
    /// expansions by substitution.
    pub(super) fn index_keep_body(&mut self, sym: SymId, mark: Recorded) {
        if let Some(ix) = self.index.as_mut() {
            // The body's nodes are the definition's own, which its expansions do not repeat.
            let (nodes, records): (Vec<_>, Vec<_>) = ix.journal.split_off(mark.records).into_iter().partition(|(_, r)| matches!(r.kind, Kind::Node | Kind::Function));
            ix.journal.extend(nodes);
            ix.bodies.insert(sym, records);
        }
    }

    pub(super) fn index_expanded_body(&mut self, sym: SymId) {
        if let Some(ix) = self.index.as_deref_mut() {
            if let Some(records) = ix.bodies.get(&sym) {
                ix.journal.extend_from_slice(records);
            }
        }
    }

    pub(super) fn index_forget_bodies(&mut self, file: FileId) {
        let syms = &self.syms;
        if let Some(ix) = self.index.as_mut() {
            ix.bodies.retain(|s, _| syms.sym(*s).file != file);
        }
    }

    /// How the names of `file` are recorded: 1 with the expressions and names a completion
    /// reads, a program file's and in a language server's session a std file's (`navigable`); 2
    /// not at all.
    fn recording(&mut self, file: FileId) -> u8 {
        let f = file.0 as usize;
        let known = self.index.as_ref().and_then(|ix| ix.program.get(f).copied()).unwrap_or(0);
        if known != 0 {
            return known;
        }
        let how = if self.navigable(file) { 1 } else { 2 };
        if let Some(ix) = self.index.as_mut() {
            if ix.program.len() <= f {
                ix.program.resize(f + 1, 0);
            }
            ix.program[f] = how;
        }
        how
    }

    /// Whether names of `file` are recorded: a program file's, or a std file's in a language
    /// server's session.
    fn indexed(&mut self, file: FileId) -> bool {
        self.recording(file) != 2
    }

    /// Whether the expressions of `file` are recorded for completion: a program file's, and in a
    /// language server's session a std file's.
    fn completes(&mut self, file: FileId) -> bool {
        self.recording(file) == 1
    }

    fn index_record(&mut self, file: FileId, span: Span, target: Target, kind: Kind) {
        if span.end <= span.start || !self.indexed(file) {
            return;
        }
        let target = self.normal_target(target);
        if let Target::Sym(s) = target {
            if self.name_ref(self.syms.sym(s).name).contains('$') {
                return;
            }
        }
        if let Some(ix) = self.index.as_mut() {
            ix.journal.push((file, Record { span, target, kind }));
        }
    }

    /// An object is its class, whichever of its two symbols names it.
    /// An extension group's parameter is its first method's.
    pub(super) fn normal_target(&self, t: Target) -> Target {
        match t {
            Target::Sym(s) => match self.syms.sym(s).kind {
                SymKind::Object(c) => Target::Class(c),
                _ => Target::Sym(self.index.as_ref().and_then(|ix| ix.same_param.get(&s).copied()).unwrap_or(s)),
            },
            _ => t,
        }
    }

    /// The span of `name` at `at`: the name's own length where the text there spells it,
    /// `None` where it does not (a name the compiler made, a span that covers more).
    fn name_at(&self, file: FileId, at: u32, name: Name) -> Option<Span> {
        let text = &self.source(file).text;
        let n = self.name_ref(name);
        let start = at as usize;
        if text.get(start..start + n.len()) == Some(n) {
            return Some(Span::new(at, at + n.len() as u32));
        }
        // `` `type` ``: the name inside its backquotes.
        if text.get(start..start + 1) == Some("`") && text.get(start + 1..start + 1 + n.len()) == Some(n) {
            return Some(Span::new(at + 1, at + 1 + n.len() as u32));
        }
        None
    }

    /// Records what the expression `e`, typed as `te`, names: an identifier, a selection, an
    /// operator, or the head of an application.
    #[cold]
    #[inline(never)]
    pub(super) fn index_expr(&mut self, e: ExprId, te: TExprId) {
        let file = self.env.file;
        if !self.indexed(file) {
            return;
        }
        let ast = self.cur_ast();
        let (head, lists) = {
            let mut head = e;
            let mut lists = 0u32;
            loop {
                match ast.expr(head) {
                    Expr::Apply(f, _) | Expr::UsingApply(f, _) => {
                        lists += 1;
                        head = f;
                    }
                    Expr::TypeApply(f, _) => head = f,
                    _ => break,
                }
            }
            (head, lists)
        };
        let applied = lists > 0;
        // In an inline method's body a parameter stands for the call's argument, which the body
        // does not name: the name is the parameter's.
        if let (true, Expr::Ident(n)) = (self.inline.depth > 0, ast.expr(head)) {
            let names_a_parameter = self.inline.param_proxies.keys().any(|&p| self.syms.sym(p).name == n);
            if let (true, Some(TermRef::Local(proxy))) = (names_a_parameter, names_a_parameter.then(|| self.lookup_term(n)).flatten()) {
                if let Some((&param, _)) = self.inline.param_proxies.iter().find(|&(_, &p)| p == proxy) {
                    let kind = if applied { Kind::Call } else { Kind::Ref };
                    self.index_record(file, ast.expr_span(head), Target::Sym(param), kind);
                    return;
                }
            }
        }
        let (name, span) = match ast.expr(head) {
            Expr::Ident(n) => (n, ast.expr_span(head)),
            Expr::Select(_, n) | Expr::Infix(_, n, _) => match ast.name_span(head) {
                Some(s) => (n, s),
                None => return,
            },
            // `-v`: the `unary_-` it calls, at the operator; a builtin's (`!b`, `-n`, `+n`) calls
            // none, its node a primitive's or, for `+n`, the operand's.
            Expr::Prefix(n, _) => match (ast.name_span(head), super::prims::unary_of(n)) {
                _ if self.index.as_ref().is_some_and(|ix| ix.identity_prefixes.contains_key(&(file, ast.expr_span(head).start))) => return,
                (Some(s), Some(n)) => (n, s),
                _ => return,
            },
            Expr::Assign(lhs, _) => {
                match self.prog.expr(te) {
                    TExpr::Assign(l, _) => self.index_expr(lhs, l),
                    // An assignment through an abstract var's setter names the var.
                    TExpr::CallMethod(_, setter, _) => {
                        let var = super::setters::abstract_var_of_setter(&self.syms, self.interner, setter);
                        let span = match ast.expr(lhs) {
                            Expr::Ident(_) => Some(ast.expr_span(lhs)),
                            Expr::Select(..) => ast.name_span(lhs),
                            _ => None,
                        };
                        if let (Some(var), Some(span)) = (var, span) {
                            self.index_record(file, span, Target::Sym(var), Kind::Ref);
                        }
                    }
                    _ => {}
                }
                return;
            }
            _ => return,
        };
        // `apply.apply(1)`: the `apply` the application implies is written, after a qualifier
        // spelled as it, which is no candidate for the name.
        let implied = applied && !(name == crate::names::APPLY && self.selects_own_name(head, name));
        // A written `apply` that a closure call stands for is the function class's, before any
        // walk down the lists (`Cfg.cfg(c).apply(x)` reads no `Conversion.apply` of `Cfg.cfg(c)`).
        let Some(mut target) = self.written_function_apply(te, name, lists, head).or_else(|| self.named_target(te, name, lists, implied, head)) else { return };
        // `(Obj: Base).f`: the member the ascribed type names, as scalac's tree does, not the
        // object's override that runs (which `definition` adds: `ascribed_override`).
        if let (Target::Sym(o), Some(_)) = (target, self.ascribed_qualifier(self.cur_ast(), head)) {
            let node = if implied || lists == 0 { self.applied_head(te, lists).map(|(n, _)| n) } else { Some(te) };
            if let Some(TExpr::Field(r, m) | TExpr::CallMethod(r, m, _)) = node.map(|n| self.prog.expr(self.peel(n))) {
                if m != o && self.static_override(r, m) == o {
                    target = Target::Sym(m);
                }
            }
        }
        let kind = match target {
            Target::Sym(s) if matches!(self.syms.sym(s).kind, SymKind::Def) => Kind::Call,
            _ => Kind::Ref,
        };
        self.index_record(file, span, target, kind);
        // `Obj.m`: the object, which a static path resolves without typing it.
        if let Expr::Select(q, _) = ast.expr(head) {
            self.index_static_prefix(file, q, te);
        }
    }

    /// The node `te` the expression `e` of a recorded body (`completes`) was typed as, outside an
    /// inline expansion, where `e` is the receiver of a selection: what a completion after `e.`
    /// reads the type of.
    #[cold]
    #[inline(never)]
    pub(super) fn index_node(&mut self, e: ExprId, te: TExprId) {
        let file = self.env.file;
        if self.inline.depth > 0 || !self.completes(file) || !self.receives(file, e) {
            return;
        }
        let span = self.cur_ast().expr_span(e);
        if span.end <= span.start {
            return;
        }
        if let Some(ix) = self.index.as_mut() {
            ix.journal.push((file, Record { span, target: Target::Node(te), kind: Kind::Node }));
        }
    }

    /// The name the expression `e` of a recorded body (`completes`) is, typed as `te` where
    /// `expected` was expected: recorded where a function is expected (`expects_function`), which
    /// the name alone gives.
    #[cold]
    #[inline(never)]
    pub(super) fn index_expected(&mut self, e: ExprId, te: TExprId, expected: TypeId) {
        if matches!(self.cur_ast().expr(e), Expr::Ident(_) | Expr::Select(..)) && self.expects_function(expected, 4) {
            self.index_function_name(e, te);
        }
    }

    /// Whether a value of type `t` may be a function, which a method's name alone gives: a
    /// function type through aliases and the opaque types the site sees through, a union or an
    /// intersection with one, an unsolved variable or a type parameter bounded by one, `depth`
    /// levels down.
    pub(super) fn expects_function(&mut self, t: TypeId, depth: u32) -> bool {
        let t = self.deref_alias(t);
        if self.as_function(t).is_some() {
            return true;
        }
        match self.types.get(t) {
            // A union's or an intersection's parts are finite, unlike a chain of bounds.
            Type::Union(a, b) | Type::Inter(a, b) => self.expects_function(a, depth) || self.expects_function(b, depth),
            _ if depth == 0 => false,
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                self.expects_function(upper, depth - 1)
            }
            Type::Var(v) => {
                let bounds: Vec<TypeId> = self.tvars[v].upper.iter().chain(&self.tvars[v].lower).copied().collect();
                bounds.into_iter().any(|b| self.expects_function(b, depth - 1))
            }
            _ => match self.opaque_underlying(t) {
                Some(u) => self.expects_function(u, depth - 1),
                None => false,
            },
        }
    }

    /// Records the name the expression `e` of a recorded body (`completes`) is, typed as `te`,
    /// outside an inline expansion, as one a function is expected of (`Kind::Function`), which a
    /// completion of the name reads: an identifier's span, a selection's name's, or where the
    /// name of a selection is missing (`o.` before a delimiter) the empty span after its dot.
    #[cold]
    #[inline(never)]
    pub(super) fn index_function_name(&mut self, e: ExprId, te: TExprId) {
        let file = self.env.file;
        if self.inline.depth > 0 || !self.completes(file) {
            return;
        }
        let ast = self.cur_ast();
        let span = match ast.expr(e) {
            Expr::Ident(_) => ast.expr_span(e),
            Expr::Select(_, n) if n == crate::names::EMPTY => {
                let end = ast.expr_span(e).end;
                Span::new(end, end)
            }
            Expr::Select(..) => match ast.name_span(e) {
                Some(s) if s.end > s.start => s,
                _ => return,
            },
            _ => return,
        };
        if let Some(ix) = self.index.as_mut() {
            ix.journal.push((file, Record { span, target: Target::Node(te), kind: Kind::Function }));
        }
    }

    /// Whether the latest typing of the files `files` typed the name at `name` (the whole of its
    /// token, backquotes included; empty after a dot) where a function was expected.
    pub(super) fn index_expects_function(&self, files: &[FileId], name: Span) -> bool {
        let Some(ix) = self.index.as_ref() else { return false };
        files.iter().any(|f| {
            let Some(spans) = ix.functions.get(f.0 as usize) else { return false };
            let from = spans.partition_point(|s| s.start < name.start);
            spans[from..].iter().take_while(|s| s.start <= name.start + 1).any(|s| name.start <= s.start && s.end <= name.end)
        })
    }

    /// Whether the expression `e` of the tree of `file` is the receiver of a selection, or the
    /// callee of an application or the left operand of an operator applied to a parenthesised
    /// list (`a op (b, c)`), whose types signature help reads (`typer::signature`).
    fn receives(&mut self, file: FileId, e: ExprId) -> bool {
        let ast = self.cur_ast();
        let sources: &'a crate::source::Sources = self.files;
        let text: &'a str = &sources[file.0 as usize].text;
        let interner: &'a crate::intern::Interner = self.interner;
        let Some(ix) = self.index.as_mut() else { return false };
        let f = file.0 as usize;
        if ix.receivers.len() <= f {
            ix.receivers.resize_with(f + 1, || None);
        }
        let bits = ix.receivers[f].get_or_insert_with(|| {
            let mut bits = vec![0u64; ast.exprs.len().div_ceil(64)];
            for (i, x) in ast.exprs.iter().enumerate() {
                let q = match *x {
                    Expr::Select(q, _) | Expr::Apply(q, _) | Expr::UsingApply(q, _) => q,
                    // A left-associative operator whose right operand stands in parentheses.
                    Expr::Infix(l, op, r) if !interner.get(op).ends_with(':') => {
                        let op_end = ast.name_span(ExprId(i as u32)).map_or(ast.expr_span(l).end, |s| s.end);
                        if !parenthesised(text, op_end, ast.expr_span(r).start) {
                            continue;
                        }
                        l
                    }
                    _ => continue,
                };
                let mut q = q;
                bits[q.idx() / 64] |= 1 << (q.idx() % 64);
                // A receiver in parentheses is typed inside them.
                while let Expr::Parens(inner) = ast.expr(q) {
                    q = inner;
                    bits[q.idx() / 64] |= 1 << (q.idx() % 64);
                }
            }
            bits
        });
        bits.get(e.idx() / 64).map_or(false, |w| w & (1 << (e.idx() % 64)) != 0)
    }

    /// The value `te` of type `ty` that the expression at `span` of the file being typed stands
    /// for where no typing of that expression makes a node of it: an application's result that
    /// a further argument list applies (`factory()(x)`), an argument of a call that failed (what
    /// signature help filters the alternatives by), an extractor's result in a pattern (the
    /// pattern's span). Recorded with the receivers, outside an inline expansion.
    #[cold]
    #[inline(never)]
    pub(super) fn index_value(&mut self, span: Span, te: TExprId, ty: TypeId) {
        let file = self.env.file;
        if self.inline.depth > 0 || span.end <= span.start || !self.completes(file) {
            return;
        }
        let te = self.prog.typed(te, ty);
        if let Some(ix) = self.index.as_mut() {
            ix.journal.push((file, Record { span, target: Target::Node(te), kind: Kind::Node }));
        }
    }

    /// The target and kind of the record of one of `files` whose span is `span`.
    pub(super) fn index_record_at(&self, files: &[FileId], span: Span) -> Option<(Target, Kind)> {
        let ix = self.index.as_ref()?;
        files.iter().find_map(|f| {
            let records = ix.files.get(f.0 as usize)?;
            let i = records.partition_point(|r| (r.span.start, r.span.end) < (span.start, span.end));
            records.get(i).filter(|r| r.span == span).map(|r| (r.target, r.kind))
        })
    }

    /// The node of the file's latest typing whose span is `span`.
    pub(super) fn index_node_at(&self, files: &[FileId], span: Span) -> Option<TExprId> {
        let ix = self.index.as_ref()?;
        files.iter().find_map(|f| {
            let nodes = ix.nodes.get(f.0 as usize)?;
            let i = nodes.partition_point(|(s, _)| (s.start, s.end) <= (span.start, span.end));
            nodes[..i].last().filter(|(s, _)| *s == span).map(|&(_, e)| e)
        })
    }

    /// What `te` stands for where the source names `name` with `lists` argument lists: its
    /// callee, value or class, through a block's result, an eta-expansion or an inline expansion;
    /// the receiver of an `apply` the name stands for (`f(x)`, `Foo(x)`, `m(0)(1)`), the object
    /// of an object's `apply` whose call reads none. Where the application implies a member the
    /// source does not write (`implied`), or applies nothing (a by-name parameter's forcing), what
    /// the node of the written head stands for (`applied_head`): the function value a closure call
    /// applies (`f(x)`, `f(1)(2)`, `o.f(x)`, `mk()(x)`), the tuple an element is read of
    /// (`fs(0)(x)`). An import renames the
    /// target of a head `written` as an identifier, or an extension method; a member selected
    /// otherwise is named as it is written.
    fn named_target(&mut self, te: TExprId, name: Name, lists: u32, implied: bool, written: ExprId) -> Option<Target> {
        let ident = matches!(self.cur_ast().expr(written), Expr::Ident(_));
        // A name no list applies is walked too: a by-name parameter's use is its forcing.
        let (head, left) = if implied || lists == 0 { self.applied_head(te, lists)? } else { (te, lists) };
        let t = self.target_of(head)?;
        if self.index_target_name(t) == Some(name) {
            return Some(t);
        }
        if let Some(original) = self.index_target_name(t) {
            if (head == te || self.renamable(t, ident)) && self.renamed_by_import(name, original) {
                return Some(t);
            }
        }
        // The `apply` a written list calls: its receiver, down the lists before it.
        if left == 0 {
            return None;
        }
        match t {
            Target::Sym(s) if self.syms.sym(s).name == crate::names::APPLY => {
                // Down the receivers of the `apply`s the written lists call, one list each
                // (`m(0)(1)` of an array of arrays), to the one the head names.
                let (mut node, mut call, mut left) = (head, s, left);
                for _ in 0..64 {
                    // A plain inline call still pending answers as the
                    // expansion it becomes: no receiver read off it.
                    let pending = self.is_pending_call(self.peel(node));
                    let receiver = match (self.apply_receiver(self.peel(node), call).filter(|_| !pending), implied) {
                        (Some(r), true) => self.applied_head(r, left - 1),
                        (r, _) => r.map(|r| (r, 0)),
                    };
                    let Some((r, rest)) = receiver else { break };
                    if let Some(rt) = self.target_named(r, name, ident) {
                        return Some(rt);
                    }
                    // On through the `apply` of a member or of the std's intrinsics (an array's), not
                    // a source extension's, which dotty's lookup names itself (`ex(1)(2)`).
                    match self.target_of(r) {
                        Some(Target::Sym(next)) if rest > 0 && self.syms.sym(next).name == crate::names::APPLY && (!self.syms.sym(next).is_extension || matches!(self.prog.expr(self.peel(r)), TExpr::Js(..))) => {
                            (node, call, left) = (r, next, rest)
                        }
                        _ => break,
                    }
                }
                // The `apply` of an object named as it, whose call reads no receiver (an inline
                // one's, pending or expanded): the case class for a made-up one, else the object,
                // as scalac's tree names it.
                let Owner::Class(module) = self.syms.sym(s).owner else { return None };
                if self.syms.class(module).name != name {
                    return None;
                }
                match (self.syms.sym(s).def, self.syms.class(module).companion) {
                    (None, Some(c)) => Some(Target::Class(c)),
                    _ => Some(Target::Class(module)),
                }
            }
            _ => None,
        }
    }

    /// The `apply` written after a function value (`h.apply(3)`, `h apply 4`), which its closure
    /// call stands for: the function type's `apply` (`Function1.apply`), as scalac names it. The
    /// call is the closure call of the list after the name, the lists after it walked down as
    /// `applied_head` walks them.
    fn written_function_apply(&mut self, te: TExprId, name: Name, lists: u32, written: ExprId) -> Option<Target> {
        if name != crate::names::APPLY {
            return None;
        }
        let after = match self.cur_ast().expr(written) {
            Expr::Select(..) => lists.checked_sub(1)?,
            Expr::Infix(..) if lists == 0 => 0,
            _ => return None,
        };
        let mut node = te;
        for _ in 0..after {
            node = match (self.target_of(node), self.prog.expr(self.peel(node))) {
                (None, TExpr::CallClosure(f, _)) => f,
                (Some(Target::Sym(s)), TExpr::Field(r, _)) if matches!(self.syms.sym(s).owner, Owner::Class(c) if self.is_tuple_class(c)) => r,
                _ => return None,
            };
        }
        let (None, TExpr::CallClosure(f, _)) = (self.target_of(node), self.prog.expr(self.peel(node))) else { return None };
        let mut t = match self.prog.type_of(f) {
            Some(t) => t,
            None => match self.prog.expr(f) {
                TExpr::Local(v) | TExpr::Static(v) | TExpr::Field(_, v) => self.syms.sym(v).sig.as_ref()?.ret,
                _ => return None,
            },
        };
        for _ in 0..4 {
            t = self.deref_alias(t);
            match self.types.get(t) {
                Type::Class(c, _) if self.is_function_class(c) || self.is_context_function_class(c) => {
                    let apply = *self.syms.class(c).members.get(&crate::names::APPLY)?;
                    return (!matches!(self.syms.sym(apply).kind, SymKind::Overloaded(_))).then_some(Target::Sym(apply));
                }
                Type::Param(p) => t = self.syms.tparam(p).upper,
                _ => t = self.opaque_underlying(t)?,
            }
        }
        None
    }

    /// The target of `te` where its name is `name`, or an import's rename of it that `renamable`
    /// lets stand.
    fn target_named(&self, te: TExprId, name: Name, ident: bool) -> Option<Target> {
        let t = self.target_of(te)?;
        let n = self.index_target_name(t)?;
        (n == name || (self.renamable(t, ident) && self.renamed_by_import(name, n))).then_some(t)
    }

    /// Whether an import's rename can be what names `t` where the head is an identifier
    /// (`ident`), else a selection: an identifier's value, or an extension method.
    fn renamable(&self, t: Target, ident: bool) -> bool {
        ident || matches!(t, Target::Sym(s) if self.syms.sym(s).is_extension)
    }

    /// The node of `te`, an application of a written head with `lists` argument lists, that
    /// stands for the head, with the lists left to it: down the closure calls and the tuple
    /// element reads the lists made (`f(1)(2)`, `fs(0)(0)(x)`), and what no list writes besides,
    /// a by-name value's forcing and an implicit conversion of the value (`c(1)` of a `c` a given
    /// `Conversion` makes a function of, `cfg(0)(1)` of a tuple `cfg` of such values:
    /// `inserted_conversion`), not past a member the source wrote (`t._1(0)(x)`: the written
    /// `_1`); a node `target_of` names otherwise ends the walk (an inline expansion answers for
    /// its call).
    fn applied_head(&mut self, te: TExprId, lists: u32) -> Option<(TExprId, u32)> {
        let (mut te, mut lists) = (te, lists);
        for _ in 0..64 {
            let (target, node) = (self.target_of(te), self.prog.expr(self.peel(te)));
            // A conversion the typer applied, never a written call of the conversion's method
            // (`C conv v`, the eta-expanded `C.conv`), whatever lists are left.
            if let Some((_, value)) = self.conversion_call(target, node) {
                if self.inserted_conversion(te) {
                    te = value;
                    continue;
                }
            }
            match (target, node) {
                (Some(Target::Sym(s)), TExpr::Field(r, _)) if lists > 0 && matches!(self.syms.sym(s).owner, Owner::Class(c) if self.is_tuple_class(c)) => {
                    lists -= 1;
                    te = r;
                }
                (Some(_), _) => return Some((te, lists)),
                (None, TExpr::CallClosure(f, args)) if lists > 0 || args.is_empty() => {
                    lists = lists.saturating_sub(1);
                    te = f;
                }
                (None, _) => return None,
            }
        }
        None
    }

    /// An implicit conversion's application `node`, of target `target`: the conversion (an
    /// implicit method of one plain parameter, a given `Conversion`) and the value it converts.
    /// The lean std's `Conversion` is a function type, a given's closure call.
    fn conversion_call(&self, target: Option<Target>, node: TExpr) -> Option<(SymId, TExprId)> {
        let (conversion, args) = match (target, node) {
            (Some(Target::Sym(s)), TExpr::CallStatic(_, args) | TExpr::CallMethod(_, _, args)) if self.syms.sym(s).mods & crate::ast::mods::IMPLICIT != 0 => {
                let info = self.syms.sym(s);
                let clauses = &self.syms.sig(s).clauses;
                let plain = clauses.first().map_or(false, |c| !c.is_using && c.params.len() == 1 && !c.params[0].repeated) && clauses[1..].iter().all(|c| c.is_using);
                (info.kind == SymKind::Def && !info.is_extension && plain).then_some((s, args))?
            }
            (Some(Target::Sym(s)), TExpr::CallMethod(r, _, args)) if self.syms.sym(s).name == crate::names::APPLY => (self.conversion_value(r)?, args),
            (None, TExpr::CallClosure(f, args)) => (self.conversion_value(f)?, args),
            _ => return None,
        };
        Some((conversion, self.prog.expr_list(args).first().copied()?))
    }

    /// Whether the conversion's application `te` is one the typer applied (`try_conversion`), no
    /// written call of the conversion: an implicit conversion of the value the head names
    /// (`spec(0)(x)` of a tuple `spec`, `Spec.spec()(x)` of a nullary `spec` beside the
    /// conversion `spec`), stepped through, where `spec(s)(x)` and `Spec.spec(s)(x)` call it.
    fn inserted_conversion(&self, te: TExprId) -> bool {
        self.index.as_ref().is_some_and(|ix| ix.conversions.contains_key(&te) || ix.conversions.contains_key(&self.peel(te)))
    }

    fn is_conversion_class(&self, c: ClassId) -> bool {
        Some(c) == self.b.conversion || self.syms.class(c).base_types.iter().any(|&(b, _)| Some(b) == self.b.conversion)
    }

    /// The given or implicit value `te` stands for where it is a `Conversion`.
    fn conversion_value(&self, te: TExprId) -> Option<SymId> {
        let Some(Target::Sym(g)) = self.target_of(te) else { return None };
        let info = self.syms.sym(g);
        if info.kind != SymKind::Given && info.mods & crate::ast::mods::IMPLICIT == 0 {
            return None;
        }
        match self.types.get(self.syms.sig(g).ret) {
            Type::Class(c, _) if self.is_conversion_class(c) => Some(g),
            _ => None,
        }
    }

    /// The receiver of the call `te` of the member `apply`: a method call's, an extension's first
    /// argument (`ex(5)` of an `extension (e: Ex) def apply`), or the first operand of its
    /// intrinsic (`PartialFunction.apply` as `$0($1)`).
    fn apply_receiver(&self, te: TExprId, apply: SymId) -> Option<TExprId> {
        let extension = self.syms.sym(apply).is_extension;
        match self.prog.expr(te) {
            TExpr::CallStatic(m, l) | TExpr::CallMethod(_, m, l) if extension && m == apply => self.prog.expr_list(l).first().copied(),
            TExpr::CallMethod(r, m, _) if !extension && self.static_override(r, m) == apply => Some(r),
            // `$0` of a member's template is the receiver, of an extension's its parameter.
            TExpr::Js(template, l) if self.prog.template_syms.get(&template) == Some(&apply) && (extension || matches!(self.syms.sym(apply).owner, Owner::Class(_))) => {
                self.prog.expr_list(l).first().copied()
            }
            _ => None,
        }
    }

    /// Whether an import in scope binds `name` to a member called `original` (`import
    /// p.{original as name}`).
    fn renamed_by_import(&self, name: Name, original: Name) -> bool {
        let file_imports = self.file_imports.get(self.env.file.0 as usize).and_then(Option::as_ref).map_or(&[][..], |v| v.as_slice());
        self.env.imports.iter().chain(file_imports).any(|imp| {
            imp.name == Some(name)
                && matches!(imp.target, super::ImportTarget::ClassMember(_, m) | super::ImportTarget::PkgMember(_, m) | super::ImportTarget::ValueMember(_, m) if m == original)
        })
    }

    fn peel(&self, mut te: TExprId) -> TExprId {
        for _ in 0..16 {
            match self.prog.expr(te) {
                TExpr::Block(_, res) => te = res,
                TExpr::Lambda(_, body) => te = body,
                _ => break,
            }
        }
        te
    }

    /// Whether the head `q.name` or `q name r` of an application selects `name` from a qualifier
    /// spelled `name` itself, or an import's rename of it, its applications, parentheses,
    /// ascriptions and a block's statements aside (`apply.apply(1)`, `apply(0).apply(2)`,
    /// `(apply: Int => Int).apply(1)`).
    fn selects_own_name(&self, head: ExprId, name: Name) -> bool {
        let ast = self.cur_ast();
        let mut q = match ast.expr(head) {
            Expr::Select(q, _) => q,
            Expr::Infix(l, op, r) => match self.name_ref(op).ends_with(':') {
                true => r,
                false => l,
            },
            _ => return false,
        };
        for _ in 0..64 {
            match ast.expr(q) {
                Expr::Apply(f, _) | Expr::UsingApply(f, _) | Expr::TypeApply(f, _) | Expr::Parens(f) | Expr::Typed(f, _) | Expr::Unchecked(f) => q = f,
                Expr::Block(stmts) => match ast.stmt_list(stmts).last() {
                    Some(&ast::Stmt::Expr(r)) => q = r,
                    _ => return false,
                },
                Expr::Ident(m) | Expr::Select(_, m) => return m == name || self.renamed_by_import(m, name),
                _ => return false,
            }
        }
        false
    }

    fn target_of(&self, te: TExprId) -> Option<Target> {
        let ix = self.index.as_ref()?;
        let mut te = te;
        for _ in 0..16 {
            if let Some(&callee) = ix.expansions.get(&te) {
                return Some(Target::Sym(callee));
            }
            return Some(match self.prog.expr(te) {
                TExpr::Block(_, res) | TExpr::Lambda(_, res) => {
                    te = res;
                    continue;
                }
                TExpr::Field(r, s) | TExpr::CallMethod(r, s, _) => Target::Sym(self.static_override(r, s)),
                TExpr::Local(s) | TExpr::Static(s) | TExpr::CallStatic(s, _) | TExpr::NewVia(s, _) => Target::Sym(s),
                TExpr::New(c, _) | TExpr::Module(c) => Target::Class(c),
                TExpr::Js(template, _) => Target::Sym(*self.prog.template_syms.get(&template)?),
                _ => return None,
            });
        }
        None
    }

    /// The member a call of `s` on the receiver `r` runs where the receiver is an object or a
    /// class-backed given (a top-level one, or a member read through its prefix), whose own
    /// definition overrides `s`: what scalac's tree names there.
    fn static_override(&self, r: TExprId, s: SymId) -> SymId {
        let class = match self.prog.expr(r) {
            TExpr::Module(c) => c,
            TExpr::Static(g) | TExpr::Field(_, g) => match (self.syms.sym(g).impl_class, self.syms.sym(g).kind) {
                (Some(c), _) | (None, SymKind::Object(c)) => c,
                // A structural given's value is of its class.
                (None, SymKind::Given) => match self.types.get(self.syms.sig(g).ret) {
                    Type::Class(c, _) if self.syms.class(c).kind == ClassKind::GivenImpl => c,
                    _ => return s,
                },
                _ => return s,
            },
            _ => return s,
        };
        self.own_override(class, s)
    }

    /// The member of `class` itself that overrides `s`, else `s`.
    fn own_override(&self, class: ClassId, s: SymId) -> SymId {
        if self.syms.sym(s).owner == Owner::Class(class) {
            return s;
        }
        let name = self.syms.sym(s).name;
        let info = self.syms.class(class);
        let own = |m: &SymId| self.syms.sym(*m).owner == Owner::Class(class) && !matches!(self.syms.sym(*m).kind, SymKind::Overloaded(_));
        match info.members.get(&name).filter(|m| own(m)) {
            Some(&m) => m,
            // An extension method is no member: of the class's extensions the one of the name.
            None => {
                let mut exts = info.extensions.iter().filter(|&&m| self.syms.sym(m).name == name && own(&m));
                match (exts.next(), exts.next()) {
                    (Some(&m), None) => m,
                    _ => s,
                }
            }
        }
    }

    /// The path `p` of a selection `head` on `(p: T)`, through parentheses: a receiver ascribed
    /// a type.
    fn ascribed_qualifier(&self, ast: &ast::Ast, head: ExprId) -> Option<ExprId> {
        let mut q = match ast.expr(head) {
            Expr::Select(q, _) | Expr::Prefix(_, q) => q,
            Expr::Infix(l, op, r) => match self.name_ref(op).ends_with(':') {
                true => r,
                false => l,
            },
            _ => return None,
        };
        while let Expr::Parens(inner) = ast.expr(q) {
            q = inner;
        }
        let Expr::Typed(mut p, _) = ast.expr(q) else { return None };
        while let Expr::Parens(inner) = ast.expr(p) {
            p = inner;
        }
        matches!(ast.expr(p), Expr::Ident(_) | Expr::Select(..)).then_some(p)
    }

    /// Where the record of the member `s` at `span` of `file` names a selection on an object or
    /// a class-backed given ascribed a base type (`(Obj: Base).f`), the object's own member that
    /// overrides `s`, which the call runs: `definition` answers both, as dotty's lookup does.
    fn ascribed_override(&self, file: FileId, span: Span, s: SymId) -> Option<SymId> {
        let ast = self.ast(file);
        let &(head, _) = ast.name_spans.iter().find(|&&(_, at)| at == span)?;
        let path = self.ascribed_qualifier(ast, head)?;
        let at = match ast.expr(path) {
            Expr::Ident(_) => ast.expr_span(path),
            _ => ast.name_span(path)?,
        };
        let class = match self.index_record_at(&[file], at)?.0 {
            Target::Class(c) if self.syms.class(c).kind == ClassKind::Object => c,
            Target::Sym(g) => self.syms.sym(g).impl_class?,
            _ => return None,
        };
        // A member of the class's own that the override check found to override `s`, through
        // the overrides between them (an extension, which no member overrides, has none).
        let ix = self.index.as_ref()?;
        let mut queue = vec![s];
        let mut seen = 0;
        while let Some(m) = queue.pop() {
            seen += 1;
            if seen > 64 {
                break;
            }
            for &o in ix.overrides.get(&m).map_or(&[][..], |v| v.as_slice()) {
                if self.syms.sym(o).owner == Owner::Class(class) && self.fresh_sym(o) {
                    return Some(o);
                }
                queue.push(o);
            }
        }
        None
    }

    fn index_target_name(&self, t: Target) -> Option<Name> {
        Some(match t {
            Target::Sym(s) => self.syms.sym(s).name,
            Target::Class(c) => self.syms.class(c).name,
            Target::Alias(a) => self.syms.aliases[a.idx()].name,
            Target::TParam(p) => self.syms.tparam(p).name,
            Target::Node(_) => return None,
        })
    }

    /// The object `q` of a selection `q.m` whose receiver came out as the object.
    fn index_static_prefix(&mut self, file: FileId, q: ExprId, te: TExprId) {
        let recv = match self.prog.expr(self.peel(te)) {
            TExpr::Field(r, _) | TExpr::CallMethod(r, _, _) => r,
            TExpr::Static(s) | TExpr::CallStatic(s, _) => match self.syms.sym(s).owner {
                Owner::Class(c) if self.syms.class(c).kind == ClassKind::Object => {
                    return self.index_object_path(file, q, c);
                }
                _ => return,
            },
            _ => return,
        };
        if let TExpr::Module(c) = self.prog.expr(recv) {
            self.index_object_path(file, q, c);
        }
    }

    fn index_object_path(&mut self, file: FileId, q: ExprId, c: ClassId) {
        let ast = self.ast(file);
        let (name, span) = match ast.expr(q) {
            Expr::Ident(n) => (n, ast.expr_span(q)),
            Expr::Select(_, n) => match ast.name_span(q) {
                Some(s) => (n, s),
                None => return,
            },
            _ => return,
        };
        if self.syms.class(c).name == name {
            self.index_record(file, span, Target::Class(c), Kind::Ref);
        }
    }

    /// A type name resolved to a class, an alias or a type parameter.
    #[cold]
    #[inline(never)]
    pub(super) fn index_type(&mut self, ty: TyExprId, r: TypeRef) {
        let file = self.env.file;
        if !self.indexed(file) {
            return;
        }
        let ast = self.cur_ast();
        let span = match ast.ty(ty) {
            ast::TyExpr::Name(_) => ast.ty_spans[ty.idx()],
            ast::TyExpr::Select(..) => match ast.ty_name_span(ty) {
                Some(s) => s,
                None => return,
            },
            _ => return,
        };
        let target = match r {
            TypeRef::Class(c) => Target::Class(c),
            TypeRef::Alias(a) => Target::Alias(a),
            TypeRef::Param(p) => Target::TParam(p),
            _ => return,
        };
        self.index_record(file, span, target, Kind::Type);
    }

    #[cold]
    #[inline(never)]
    pub(super) fn index_tparams(&mut self, tps: &[ast::TypeParam], ids: &[(Name, TParamId)]) {
        let file = self.env.file;
        if !self.indexed(file) {
            return;
        }
        let spans: Vec<(TParamId, Span)> = tps.iter().zip(ids).filter_map(|(tp, &(n, p))| Some((p, self.name_at(file, tp.span.start, n)?))).collect();
        if let Some(ix) = self.index.as_mut() {
            for (p, span) in spans {
                ix.tparams.insert(p, (file, span));
            }
        }
    }

    /// The type parameters of a polymorphic function literal, `[A, B] => ...` at `span`, where
    /// they are written.
    #[cold]
    #[inline(never)]
    pub(super) fn index_poly_tparams(&mut self, span: Span, names: &[Name], ids: &[TParamId]) {
        let file = self.env.file;
        if !self.indexed(file) {
            return;
        }
        let text = &self.source(file).text;
        let mut at = span.start as usize;
        let mut found = Vec::new();
        for (&n, &p) in names.iter().zip(ids) {
            let name = self.name_ref(n);
            let Some(i) = text.get(at..span.end as usize).and_then(|t| t.find(name)) else { break };
            let start = at + i;
            found.push((p, Span::new(start as u32, (start + name.len()) as u32)));
            at = start + name.len();
        }
        if let Some(ix) = self.index.as_mut() {
            for (p, s) in found {
                ix.tparams.insert(p, (file, s));
            }
        }
    }

    /// The declaration of a local.
    #[cold]
    #[inline(never)]
    pub(super) fn index_local(&mut self, sym: SymId) {
        let info = self.syms.sym(sym);
        let (file, at, name) = (info.file, info.span.start, info.name);
        if !self.indexed(file) {
            return;
        }
        if let Some(span) = self.name_at(file, at, name) {
            self.index_record(file, span, Target::Sym(sym), Kind::Def);
            if let Some(ix) = self.index.as_mut() {
                ix.locals.insert(sym, (file, span));
            }
        }
    }

    /// The declaration of a local the typer made at another place than its name's: a binder of
    /// a `for`, whose local stands at the comprehension, declared at its pattern.
    #[cold]
    #[inline(never)]
    pub(super) fn index_local_at(&mut self, sym: SymId, pat: Span) {
        let (file, name) = (self.syms.sym(sym).file, self.syms.sym(sym).name);
        if !self.indexed(file) {
            return;
        }
        let Some(span) = self.name_at(file, pat.start, name) else { return };
        match self.index.as_ref().and_then(|ix| ix.locals.get(&sym)) {
            Some(&(_, at)) if at == span => return,
            // A name the comprehension's own text spells (`f` and `fo` at `for`): `new_local`
            // declared the binder there, which its pattern replaces.
            Some(_) => self.index_unrecord(sym),
            None => {}
        }
        self.index_record(file, span, Target::Sym(sym), Kind::Def);
        if let Some(ix) = self.index.as_mut() {
            ix.locals.insert(sym, (file, span));
        }
    }

    /// What an expansion made to stand for an argument (a parameter's proxy): no declaration of
    /// the source's, so its record goes.
    #[cold]
    #[inline(never)]
    pub(super) fn index_unrecord(&mut self, sym: SymId) {
        let Some(ix) = self.index.as_mut() else { return };
        if ix.locals.remove(&sym).is_some() {
            if let Some(i) = ix.journal.iter().rposition(|(_, r)| r.kind == Kind::Def && r.target == Target::Sym(sym)) {
                ix.journal.remove(i);
            }
        }
    }

    /// A method's parameter as it is declared: the parameters of an extension group's methods
    /// that the group's one declaration makes are one parameter.
    #[cold]
    #[inline(never)]
    pub(super) fn index_param(&mut self, sym: SymId) {
        let (file, at, name) = {
            let info = self.syms.sym(sym);
            (info.file, info.span.start, info.name)
        };
        let first = self.index.as_ref().and_then(|ix| ix.params_at.get(&(file, at)).copied());
        match first {
            Some(first) if first != sym && self.syms.sym(first).name == name => {
                if let Some(ix) = self.index.as_mut() {
                    ix.same_param.insert(sym, first);
                }
            }
            Some(_) => {}
            None => {
                if let Some(ix) = self.index.as_mut() {
                    ix.params_at.insert((file, at), sym);
                }
            }
        }
        self.index_local(sym);
    }

    /// `m` overrides `p`, as the override check found.
    #[cold]
    #[inline(never)]
    pub(super) fn index_override(&mut self, m: SymId, p: SymId) {
        if let Some(ix) = self.index.as_mut() {
            let overriding = ix.overrides.entry(p).or_default();
            if !overriding.contains(&m) {
                overriding.push(m);
            }
        }
    }

    /// The names of an import or export clause, its path resolved to `target`: the object the
    /// path ends in and the member it selects.
    #[cold]
    #[inline(never)]
    pub(super) fn index_import(&mut self, imp: &ast::Import, target: super::ImportTarget) {
        let target = match (target, &imp.sel) {
            (super::ImportTarget::PkgAll(p), ast::ImportSel::Name(n, _)) => super::ImportTarget::PkgMember(p, *n),
            (super::ImportTarget::ClassAll(c), ast::ImportSel::Name(n, _)) => super::ImportTarget::ClassMember(c, *n),
            (t, _) => t,
        };
        let file = self.env.file;
        let ast = self.cur_ast();
        let Some((_, names)) = ast.import_names.iter().find(|(s, _)| *s == imp.span) else { return };
        let names = names.clone();
        let n = imp.path.len();
        if let (super::ImportTarget::ClassAll(c) | super::ImportTarget::ClassMember(c, _), Some(&last)) = (target, n.checked_sub(1).and_then(|i| names.get(i))) {
            self.index_record(file, last, Target::Class(c), Kind::Import);
        }
        let ast::ImportSel::Name(member, _) = imp.sel else { return };
        let found = match target {
            super::ImportTarget::PkgMember(p, _) => self.syms.pkg(p).entries.get(&member).and_then(|e| {
                e.class.map(Target::Class).or(e.term.map(Target::Sym)).or(e.alias.map(Target::Alias))
            }),
            super::ImportTarget::ClassMember(c, _) => {
                let info = self.syms.class(c);
                info.members.get(&member).map(|&s| Target::Sym(s)).or(info.nested.get(&member).map(|&k| Target::Class(k))).or(info.type_aliases.get(&member).map(|&a| Target::Alias(a)))
            }
            _ => None,
        };
        let Some(found) = found else { return };
        let found = match found {
            // Of an overloaded name, the first alternative stands for all.
            Target::Sym(s) => match self.syms.alternatives(s) {
                Some(alts) => Target::Sym(*alts.first().unwrap_or(&s)),
                None => found,
            },
            t => t,
        };
        for &span in names.iter().skip(n) {
            self.index_record(file, span, found, Kind::Import);
        }
    }

    /// `name = value` passed to the parameter `param`.
    #[cold]
    #[inline(never)]
    pub(super) fn index_named_arg(&mut self, arg: ExprId, name: Name, param: SymId) {
        let file = self.env.file;
        let at = self.cur_ast().expr_span(arg).start;
        if let Some(span) = self.name_at(file, at, name) {
            self.index_record(file, span, Target::Sym(param), Kind::NamedArg);
        }
    }

    /// The class or extractor of the constructor pattern at `path`.
    #[cold]
    #[inline(never)]
    pub(super) fn index_pattern(&mut self, path: ExprId, tp: TPatId) {
        let file = self.env.file;
        if !self.indexed(file) {
            return;
        }
        let ast = self.cur_ast();
        let (name, span) = match ast.expr(path) {
            Expr::Ident(n) => (n, ast.expr_span(path)),
            Expr::Select(_, n) => match ast.name_span(path) {
                Some(s) => (n, s),
                None => return,
            },
            _ => return,
        };
        let target = match self.prog.pats[tp.idx()] {
            TPat::Class(c, ..) => Target::Class(c),
            TPat::Unapply(_, call, _) => match self.target_of(call) {
                Some(Target::Sym(s)) => match self.syms.sym(s).owner {
                    Owner::Class(c) => Target::Class(c),
                    _ => return,
                },
                _ => return,
            },
            _ => return,
        };
        let target = match target {
            Target::Class(c) if self.syms.class(c).name != name => match self.syms.class(c).companion {
                Some(k) if self.syms.class(k).name == name => Target::Class(k),
                _ => return,
            },
            t => t,
        };
        self.index_record(file, span, target, Kind::Pattern);
        if let Expr::Select(q, _) = ast.expr(path) {
            if let Target::Class(c) = target {
                if let Owner::Class(o) = self.syms.class(c).owner {
                    if self.syms.class(o).kind == ClassKind::Object {
                        self.index_object_path(file, q, o);
                    }
                }
            }
        }
    }

    /// Before a file's bodies are typed again: its records that the edit left outside bodies
    /// move with their names, the others go, and the locals made from here on are the file's.
    pub(super) fn index_move(&mut self, file: FileId, remap: &crate::shape::Remap) {
        let (n_syms, n_classes, n_aliases) = (self.syms.syms.len() as u32, self.syms.classes.len() as u32, self.syms.aliases.len() as u32);
        let of_file: Vec<bool> = self.syms.syms.iter().map(|s| s.file == file).collect();
        let Some(ix) = self.index.as_mut() else { return };
        ix.fresh.insert(file, (n_syms, n_classes, n_aliases));
        // The overrides the file's classes are checked for again.
        for overriding in ix.overrides.values_mut() {
            overriding.retain(|m| !of_file[m.idx()]);
        }
        ix.overrides.retain(|_, v| !v.is_empty());
        ix.params_at.retain(|(f, _), _| *f != file);
        let moved = |span: Span| remap.moved.get(&span).copied();
        // Its nodes are its bodies', which are typed again, of its tree parsed again.
        if let Some(nodes) = ix.nodes.get_mut(file.0 as usize) {
            nodes.clear();
        }
        if let Some(functions) = ix.functions.get_mut(file.0 as usize) {
            functions.clear();
        }
        if let Some(receivers) = ix.receivers.get_mut(file.0 as usize) {
            *receivers = None;
        }
        ix.block_imports.retain(|&(f, _), _| f != file);
        if let Some(records) = ix.files.get_mut(file.0 as usize) {
            records.retain_mut(|r| match moved(r.span) {
                Some(s) => {
                    r.span = s;
                    true
                }
                None => false,
            });
        }
        let mut gone = Vec::new();
        ix.locals.retain(|&s, (f, span)| {
            if *f != file {
                return true;
            }
            match moved(*span) {
                Some(m) => {
                    *span = m;
                    true
                }
                None => {
                    gone.push(s);
                    false
                }
            }
        });
        for s in gone {
            ix.same_param.remove(&s);
        }
        ix.tparams.retain(|_, (f, span)| {
            if *f != file {
                return true;
            }
            match moved(*span) {
                Some(s) => {
                    *span = s;
                    true
                }
                None => false,
            }
        });
    }

    /// After a phase: the journal into the per-file tables, which the first query after it
    /// orders (`index_order`), so that a build pays for the pushes alone.
    pub fn index_settle(&mut self) {
        if self.index.is_none() {
            return;
        }
        GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.index_file_journal();
    }

    /// The journal into the per-file tables.
    fn index_file_journal(&mut self) {
        let Some(ix) = self.index.as_mut() else { return };
        ix.inverse = None;
        // The expansions and conversions are read while the calls they stand for are typed, which
        // is over, and so are the receivers' marks.
        ix.expansions.clear();
        ix.conversions.clear();
        ix.identity_prefixes.clear();
        ix.receivers = Vec::new();
        let mut last = None;
        // The journal of a full build is the largest it gets: its room goes with it.
        for (file, r) in std::mem::take(&mut ix.journal) {
            if last != Some(file) {
                ix.unordered.push(file);
                last = Some(file);
            }
            let f = file.0 as usize;
            if r.kind == Kind::Function {
                if ix.functions.len() <= f {
                    ix.functions.resize_with(f + 1, Vec::new);
                }
                ix.functions[f].push(r.span);
                continue;
            }
            if let Target::Node(e) = r.target {
                if ix.nodes.len() <= f {
                    ix.nodes.resize_with(f + 1, Vec::new);
                }
                ix.nodes[f].push((r.span, e));
                continue;
            }
            if ix.files.len() <= f {
                ix.files.resize_with(f + 1, Vec::new);
            }
            ix.files[f].push(r);
        }
        // A std file's records are filed once, by the build or a demand of its document's.
        let std = &self.std;
        let std_files: Vec<FileId> = ix.unordered.iter().copied().filter(|&f| std.slot_of(f).is_some()).collect();
        if let Some(ix) = self.index.as_mut() {
            for f in std_files {
                if let Some(records) = ix.files.get_mut(f.0 as usize) {
                    records.shrink_to_fit();
                }
            }
        }
    }

    /// The tables of the files recorded into since the last query, ordered by span, the last
    /// record of a span winning.
    fn index_order(&mut self) {
        let Some(ix) = self.index.as_mut() else { return };
        let mut files = std::mem::take(&mut ix.unordered);
        files.sort_unstable();
        files.dedup();
        for f in files {
            if let Some(nodes) = ix.nodes.get_mut(f.0 as usize) {
                // Stable: of the nodes of one span the last is the latest.
                nodes.sort_by_key(|(s, _)| (s.start, s.end));
                nodes.shrink_to_fit();
            }
            if let Some(functions) = ix.functions.get_mut(f.0 as usize) {
                functions.sort_unstable_by_key(|s| (s.start, s.end));
                functions.dedup();
            }
            let Some(records) = ix.files.get_mut(f.0 as usize) else { continue };
            // Stable: of the records of one span the last is the latest.
            records.sort_by_key(|r| (r.span.start, r.span.end));
            records.dedup_by(|later, kept| {
                let same = later.span == kept.span;
                if same {
                    *kept = *later;
                }
                same
            });
        }
    }

    /// Records by target, built once per build.
    fn inverse(&mut self) -> &FxMap<Target, Vec<(FileId, Record)>> {
        let ix = self.index.as_mut().expect("the index is kept");
        if ix.inverse.is_none() {
            let mut inv: FxMap<Target, Vec<(FileId, Record)>> = FxMap::default();
            for (f, records) in ix.files.iter().enumerate() {
                let f = FileId(f as u32);
                for r in records {
                    inv.entry(r.target).or_default().push((f, *r));
                }
            }
            for v in inv.values_mut() {
                v.sort_by_key(|(f, r)| (*f, r.span.start));
            }
            ix.inverse = Some(inv);
        }
        ix.inverse.as_ref().unwrap()
    }

    // ---- declarations ----

    /// Whether a local of `file` belongs to the file's latest typing.
    fn fresh_sym(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        if info.owner != Owner::Local {
            return match info.owner {
                Owner::Class(c) => self.fresh_class(c),
                _ => true,
            };
        }
        let fresh = self.index.as_ref().and_then(|ix| ix.fresh.get(&info.file)).map_or(0, |&(s, _, _)| s);
        s.0 >= fresh
    }

    fn fresh_class(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        if info.owner != Owner::Local {
            return true;
        }
        let fresh = self.index.as_ref().and_then(|ix| ix.fresh.get(&info.file)).map_or(0, |&(_, k, _)| k);
        c.0 >= fresh
    }

    fn fresh_alias(&self, a: AliasId) -> bool {
        let info = &self.syms.aliases[a.idx()];
        if info.owner != Owner::Local {
            return true;
        }
        let fresh = self.index.as_ref().and_then(|ix| ix.fresh.get(&info.file)).map_or(0, |&(_, _, a)| a);
        a.0 >= fresh
    }

    /// Whether the names of `file` are recorded and its declarations located in it: a program
    /// file's, and in a language server's session (the one that keeps the index) a std file's, in
    /// the file's document (`attach::std_document_path`). `program_source` and `is_std` keep
    /// their meaning for the rest of the typer (the unused imports, the `Predef` fallback,
    /// completion's entries).
    pub(super) fn navigable(&self, file: FileId) -> bool {
        self.program_source(file) || self.index.is_some() && self.std_file(file)
    }

    /// Whether `file` is one of the embedded std's (a slot or a `package p:` block of one), not
    /// a jar's pseudo file or a library body, which are `is_std` as well.
    pub(super) fn std_file(&self, file: FileId) -> bool {
        self.std.slot_of(file).is_some()
    }

    /// Where a target is declared: its file and the span of its name there.
    fn declaration(&self, t: Target) -> Option<(FileId, Span)> {
        let (file, at, name) = match t {
            Target::Sym(s) => {
                if let Some(&(file, span)) = self.index.as_ref().and_then(|ix| ix.locals.get(&s)) {
                    return self.navigable(file).then_some((file, span));
                }
                let info = self.syms.sym(s);
                (info.file, info.span.start, info.name)
            }
            Target::Class(c) => {
                let info = self.syms.class(c);
                (info.file, info.span.start, info.name)
            }
            Target::Alias(a) => {
                let info = &self.syms.aliases[a.idx()];
                let d = info.def?;
                (info.file, self.ast(info.file).def(d).span.start, info.name)
            }
            Target::TParam(p) => return self.index.as_ref()?.tparams.get(&p).copied(),
            Target::Node(_) => return None,
        };
        if !self.navigable(file) {
            return None;
        }
        Some((file, self.name_at(file, at, name)?))
    }

    /// The declaration whose name is at `offset` in one of `files`.
    pub(super) fn declaration_at(&self, files: &[FileId], offset: u32) -> Option<(Span, Target)> {
        let within = |span: Span| span.start <= offset && offset <= span.end;
        let mut best: Option<(Span, Target)> = None;
        let mut consider = |span: Span, t: Target| {
            if within(span) && best.map_or(true, |(b, _)| span.end - span.start < b.end - b.start) {
                best = Some((span, t));
            }
        };
        for (i, info) in self.syms.syms.iter().enumerate() {
            // A local's declaration is its `Def` record.
            if !files.contains(&info.file) || info.owner == Owner::Local {
                continue;
            }
            let s = SymId(i as u32);
            if matches!(info.kind, SymKind::Overloaded(_)) || !self.fresh_sym(s) {
                continue;
            }
            if let Some(span) = self.name_at(info.file, info.span.start, info.name) {
                consider(span, self.normal_target(Target::Sym(s)));
            }
        }
        for (i, info) in self.syms.classes.iter().enumerate() {
            if !files.contains(&info.file) || info.def.is_none() || !self.fresh_class(ClassId(i as u32)) {
                continue;
            }
            if let Some(span) = self.name_at(info.file, info.span.start, info.name) {
                consider(span, Target::Class(ClassId(i as u32)));
            }
        }
        for (i, info) in self.syms.aliases.iter().enumerate() {
            if !files.contains(&info.file) || !self.fresh_alias(AliasId(i as u32)) {
                continue;
            }
            if let Some((_, span)) = self.declaration(Target::Alias(AliasId(i as u32))) {
                consider(span, Target::Alias(AliasId(i as u32)));
            }
        }
        if let Some(ix) = self.index.as_ref() {
            for (&p, &(f, span)) in &ix.tparams {
                if files.contains(&f) {
                    consider(span, Target::TParam(p));
                }
            }
        }
        best
    }

    /// The record or declaration at `offset`: the innermost, a record before a declaration.
    fn target_at(&self, files: &[FileId], offset: u32) -> Option<(FileId, Span, Target, Kind)> {
        let ix = self.index.as_ref()?;
        let mut best: Option<(FileId, Span, Target, Kind)> = None;
        for &f in files {
            let Some(records) = ix.files.get(f.0 as usize) else { continue };
            let upto = records.partition_point(|r| r.span.start <= offset);
            for r in records[..upto].iter().rev().take(64) {
                if r.span.end < offset {
                    continue;
                }
                if best.map_or(true, |(_, b, _, _)| r.span.end - r.span.start < b.end - b.start) {
                    best = Some((f, r.span, r.target, r.kind));
                }
            }
        }
        if let Some((span, t)) = self.declaration_at(files, offset) {
            if best.map_or(true, |(_, b, _, _)| span.end - span.start < b.end - b.start) {
                let file = self.declaration(t).map_or(files[0], |(f, _)| f);
                best = Some((file, span, t, Kind::Def));
            }
        }
        best
    }

    // ---- queries ----

    /// The answer to a query, and for a query about a target (references, implementations,
    /// callers) where the target is declared: the other sessions whose programs hold that file
    /// are asked there too.
    pub fn index_query(&mut self, q: &crate::index::Query, files: &Files) -> (Json, Option<(PathBuf, u32)>) {
        use crate::index::Query;
        let demanded = self.index.as_ref().map_or(0, |ix| ix.demanded());
        if let Some(p) = q.file() {
            let ids = files.ids_of(p);
            if ids.first().map_or(false, |&f| self.std_file(f)) {
                // The offsets are in the text the client holds, the file's as the server read it:
                // a std document whose file holds another text than the embedded one is made whole
                // and the query on it declined, as a library document's is (`document_target_at`).
                if !self.std_document_whole(ids[0], p) {
                    let declined = match q {
                        Query::Complete(..) => super::complete::list(false, Vec::new()),
                        Query::Resolve(..) => obj([("edits", Json::Arr(Vec::new()))]),
                        _ => Json::Null,
                    };
                    return (declined, None);
                }
                self.demand_std_document(&ids);
            }
        }
        self.index_order();
        // A query at a library's document: its occurrence looked up once, so that a lookup that
        // declined (the document had to be made whole, `document_target_at`) declines the query.
        let at_document = match q.file() {
            Some(p) if files.ids_of(p).is_empty() => Some(document_offset(q).and_then(|o| self.document_target_at(p, o))),
            _ => None,
        };
        let shared = |found: &Option<(Span, DocTarget, std::sync::Arc<DocIndex>)>| match found {
            Some((_, DocTarget::Shared(t), _)) => Some(*t),
            _ => None,
        };
        let target = match (q, &at_document) {
            (Query::References(..) | Query::Implementation(..), Some(found)) => shared(found),
            (Query::Incoming(..), Some(found)) => shared(found).filter(|&t| self.is_callable(t)),
            (Query::References(p, offset, _) | Query::Implementation(p, offset), None) => self.target_at(&files.ids_of(p), *offset).map(|(_, _, t, _)| t),
            (Query::Incoming(p, offset), None) => self.callable_at(&files.ids_of(p), *offset),
            _ => None,
        };
        // Who uses, extends or calls what a std body can name: every std file's records, not
        // only those of the documents demanded so far.
        if target.map_or(false, |t| self.reaches_std(t)) {
            self.demand_all_std();
            self.index_order();
        }
        // What the stores outgrew while a demand typed, freed as a retype frees it: between two
        // queries no reference into it is alive.
        if self.index.as_ref().map_or(0, |ix| ix.demanded()) > demanded {
            self.types.release_retired();
            self.interner.release_retired();
        }
        let declared = match q {
            Query::References(..) | Query::Implementation(..) | Query::Incoming(..) => target.and_then(|t| self.declared_at(t, files)),
            _ => None,
        };
        let answer = match q {
            Query::Complete(p, offset, flags) => return (self.complete(p, *offset, *flags, files), None),
            Query::Resolve(p, offset, generation, name) => return (self.complete_resolve(p, *offset, generation, name, files), None),
            Query::Signature(p, offset) => return (self.signature(p, *offset, files), None),
            _ => match at_document {
                Some(found) => self.document_answer(q, found, files),
                None => self.answer(q, files),
            },
        };
        (answer, declared)
    }

    /// Whether the document at `path` of the std file `file` holds the embedded text, compared
    /// before any repair; where it does not, it is made whole again for the next query
    /// (`attach::write_document`), what is no regular file left as it is.
    fn std_document_whole(&mut self, file: FileId, path: &std::path::Path) -> bool {
        let Some(slot) = self.std.slot_of(file) else { return true };
        let text = self.std.slots[slot].text;
        let held = super::loader::attach::document_bytes(path);
        if held.as_deref() == Some(text.as_bytes()) {
            return true;
        }
        if held.is_some() || std::fs::symlink_metadata(path).is_err() {
            super::loader::attach::write_document(path, text);
        }
        false
    }

    /// The first query on a std file's document (`ids` the file and its `package p:` blocks):
    /// the file entered where the program never entered it, and what of it a check session
    /// leaves untyped typed now with its names recorded: its classes checked as emission's reach
    /// checks a std class it meets (`check_std_class`: parents, constructors, fields), every
    /// member's signature and every alias completed, the bodies of its methods (`defers_body`)
    /// typed, and its inline methods' bodies typed as their definition check would
    /// (`record_std_inline_body`), those the program never calls or expands included. What the
    /// std reports there is no build's: a query leaves the diagnostics as they were.
    fn demand_std_document(&mut self, ids: &[FileId]) {
        let Some(slot) = ids.first().and_then(|&f| self.std.slot_of(f)) else { return };
        let Some(ix) = self.index.as_mut() else { return };
        if ix.demanded.get(slot) == Some(&true) {
            return;
        }
        if ix.demanded.len() <= slot {
            ix.demanded.resize(slot + 1, false);
        }
        ix.demanded[slot] = true;
        let diagnostics = self.diags.items.len();
        self.enter_std_slot(slot);
        let classes: Vec<ClassId> = (0..self.syms.classes.len() as u32)
            .map(ClassId)
            .filter(|&c| ids.contains(&self.syms.class(c).file) && self.syms.class(c).owner != Owner::Local && self.syms.class(c).def.is_some() && !self.std.shadowed_classes.contains_key(&c))
            .collect();
        for c in classes {
            self.check_std_class(c);
        }
        let aliases: Vec<AliasId> = (0..self.syms.aliases.len() as u32).map(AliasId).filter(|&a| ids.contains(&self.syms.aliases[a.idx()].file) && self.syms.aliases[a.idx()].owner != Owner::Local).collect();
        for a in aliases {
            self.complete_alias(a);
        }
        let members: Vec<SymId> = (0..self.syms.syms.len() as u32)
            .map(SymId)
            .filter(|&s| {
                let info = self.syms.sym(s);
                ids.contains(&info.file) && info.owner != Owner::Local && info.def.is_some() && !matches!(info.kind, SymKind::Overloaded(_) | SymKind::Param)
            })
            .collect();
        for &s in &members {
            self.sig_arc(s);
        }
        for s in members {
            // A constructor is its class's, which `check_std_class` typed.
            if self.syms.sym(s).name == crate::names::INIT {
                continue;
            }
            if self.is_inline_callee(s) && !self.is_retained_inline(s) {
                self.record_std_inline_body(s);
            } else if self.defers_body(s) {
                self.deferred_body(s);
            }
        }
        self.drop_reported_since(diagnostics);
        self.index_file_journal();
    }

    /// Whether a std body can name `t`, so that its uses, overrides and calls stand in std files
    /// as well: a std definition that is no local, or the program's own in a package the std
    /// has members in, which a std body names where it shadows the std's (`scala.printlnImpl`).
    fn reaches_std(&self, t: Target) -> bool {
        let (file, mut owner) = match t {
            Target::Sym(s) => (self.syms.sym(s).file, self.syms.sym(s).owner),
            Target::Class(c) => (self.syms.class(c).file, self.syms.class(c).owner),
            Target::Alias(a) => (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].owner),
            Target::TParam(_) | Target::Node(_) => return false,
        };
        if matches!(t, Target::Sym(s) if self.syms.sym(s).kind == SymKind::Param) {
            return false;
        }
        loop {
            match owner {
                Owner::Local => return false,
                Owner::Class(c) => owner = self.syms.class(c).owner,
                Owner::Package(_) if self.std_file(file) => return true,
                Owner::Package(p) => {
                    let path = self.pkg_path(p);
                    return self.program_source(file) && self.std.slots.iter().any(|slot| slot.index.packages.contains(&path.as_str()));
                }
            }
        }
    }

    /// Every std file of the session's std demanded as its document's first query demands it
    /// (`demand_std_document`), once per typing of the program: the files of the Scala.js layer
    /// once it is unlocked, by the program or by a std body the demand types, which a later
    /// round demands. Measured on the application corpus's frontend by `bench/std-references.py`.
    fn demand_all_std(&mut self) {
        loop {
            let demanded = self.index.as_ref().map_or(&[][..], |ix| &ix.demanded[..]);
            let slots: Vec<Vec<FileId>> = (0..self.std.slots.len())
                .filter(|&i| demanded.get(i) != Some(&true) && (self.std.slots[i].layer == super::stdlib::Layer::Base || self.std.scalajs_unlocked()))
                .map(|i| std::iter::once(self.std.slots[i].file).chain(self.std.slots[i].blocks.iter().copied()).collect())
                .collect();
            if slots.is_empty() || self.index.is_none() {
                return;
            }
            for ids in slots {
                self.demand_std_document(&ids);
            }
        }
    }

    /// Where the other sessions are asked a query about `t` again: its declaration's file and
    /// offset, a library document's included.
    fn declared_at(&mut self, t: Target, files: &Files) -> Option<(PathBuf, u32)> {
        match self.declaration(t) {
            Some((f, span)) => Some((files.path(f)?.clone(), span.start)),
            None => self.library_declaration(t).map(|(doc, span)| (doc.path, span.start)),
        }
    }

    /// The occurrence at `offset` of a library's document: the innermost record of the
    /// document's index whose span holds the offset, a
    /// use or a declaration, with the index.
    fn document_target_at(&mut self, path: &std::path::Path, offset: u32) -> Option<(Span, DocTarget, std::sync::Arc<DocIndex>)> {
        // The offset is in the text the client holds, which is the file's as the server read it:
        // a file made whole by this query held another text, and the query is declined.
        let held = super::loader::attach::document_bytes(path)?;
        let ix = self.document_index(path)?;
        if held != ix.text.as_bytes() {
            return None;
        }
        let r = ix.at(offset)?;
        Some((r.span, r.target, ix))
    }

    /// The documents indexed so far, each as `document_index` keeps it: made whole again where
    /// its file was truncated or altered since, so that a cached occurrence is answered only in
    /// the text its span was found in, and left out where it cannot be.
    fn whole_document_indexes(&mut self) -> Vec<std::sync::Arc<DocIndex>> {
        let paths: Vec<PathBuf> = self.document_indexes().iter().map(|ix| ix.path.clone()).collect();
        paths.iter().filter_map(|p| self.document_index(p)).collect()
    }

    /// The locations of a target's occurrences in every document indexed so far, of the kinds
    /// `wanted` (every kind but a declaration where `wanted` is empty).
    fn document_use_locations(&mut self, t: DocTarget, wanted: &[Kind], files: &Files) -> Vec<Json> {
        let mut out = Vec::new();
        for ix in self.whole_document_indexes() {
            let Some(found) = ix.uses.get(&t) else { continue };
            let lines = Lines::new(&ix.text);
            for &i in found {
                let r = ix.records[i as usize];
                if !wanted.is_empty() && !wanted.contains(&r.kind) {
                    continue;
                }
                let loc = location(&ix.path, &lines, r.span, files.positions);
                if !out.contains(&loc) {
                    out.push(loc);
                }
            }
        }
        out
    }

    /// The location of a document-local definition: its name in its document.
    fn local_location(&mut self, file: u32, addr: crate::tasty::tree::Addr, files: &Files) -> Option<Json> {
        let (doc, span) = self.local_declaration(file, addr)?;
        Some(location(&doc.path, &Lines::new(&doc.text), span, files.positions))
    }

    /// Where a document target is declared.
    fn doc_decl_location(&mut self, t: DocTarget, files: &Files, cache: &mut LineCache<'a>) -> Option<Json> {
        match t {
            DocTarget::Shared(t) => self.decl_location(t, files, cache),
            DocTarget::Local(file, addr) => self.local_location(file, addr, files),
        }
    }

    /// A call hierarchy item for a document target: a shared one's as `call_item` makes it, a
    /// local method's in its document, its range the name's.
    fn doc_call_item(&mut self, t: DocTarget, files: &Files, cache: &mut LineCache<'a>) -> Option<Json> {
        match t {
            DocTarget::Shared(t) => self.call_item(t, files, cache),
            DocTarget::Local(file, addr) => {
                if !self.local_is_method(file, addr) {
                    return None;
                }
                let (doc, span) = self.local_declaration(file, addr)?;
                let name = doc.text.get(span.start as usize..span.end as usize)?.to_string();
                let range = Lines::new(&doc.text).range(span, files.positions);
                let detail = self.describe_local(file, addr);
                let mut fields = vec![("name".to_string(), Json::from(name)), ("kind".to_string(), 12u32.into())];
                if let Some(d) = detail {
                    fields.push(("detail".to_string(), d.into()));
                }
                fields.push(("uri".to_string(), crate::lsp::uri::from_path(&doc.path).into()));
                fields.push(("range".to_string(), range.clone()));
                fields.push(("selectionRange".to_string(), range));
                Some(Json::Obj(fields))
            }
        }
    }

    fn doc_is_callable(&self, t: DocTarget) -> bool {
        match t {
            DocTarget::Shared(t) => self.is_callable(t),
            DocTarget::Local(file, addr) => self.local_is_method(file, addr),
        }
    }

    /// The calls of a target in the indexed documents, grouped by the definition each stands
    /// in: the callers' entries of the call hierarchy.
    fn document_incoming(&mut self, t: DocTarget, files: &Files, cache: &mut LineCache<'a>) -> Vec<Json> {
        let mut out = Vec::new();
        for ix in self.whole_document_indexes() {
            let Some(found) = ix.uses.get(&t) else { continue };
            let mut groups: Vec<(DocTarget, Vec<Span>)> = Vec::new();
            for &i in found {
                let r = ix.records[i as usize];
                if r.kind != Kind::Call {
                    continue;
                }
                let Some(owner) = r.owner else { continue };
                match groups.iter_mut().find(|(o, _)| *o == owner) {
                    Some((_, spans)) => spans.push(r.span),
                    None => groups.push((owner, vec![r.span])),
                }
            }
            let lines = Lines::new(&ix.text);
            for (owner, spans) in groups {
                let Some(from) = self.doc_call_item(owner, files, cache) else { continue };
                let ranges: Vec<Json> = spans.iter().map(|&s| lines.range(s, files.positions)).collect();
                out.push(obj([("from", from), ("fromRanges", Json::Arr(ranges))]));
            }
        }
        out
    }

    /// The calls a library definition makes, from the index of its document: the callees'
    /// entries of the call hierarchy.
    fn document_outgoing(&mut self, t: DocTarget, files: &Files, cache: &mut LineCache<'a>) -> Vec<Json> {
        let ix = match t {
            DocTarget::Local(file, _) => self.document_index_of_file(file),
            DocTarget::Shared(t) => self.library_definition(t).and_then(|(file, _, _)| self.document_index_of_file(file)),
        };
        let Some(ix) = ix else { return Vec::new() };
        let mut groups: Vec<(DocTarget, Vec<Span>)> = Vec::new();
        for r in &ix.records {
            if r.kind != Kind::Call || r.owner != Some(t) {
                continue;
            }
            match groups.iter_mut().find(|(c, _)| *c == r.target) {
                Some((_, spans)) => spans.push(r.span),
                None => groups.push((r.target, vec![r.span])),
            }
        }
        let lines = Lines::new(&ix.text);
        let mut out = Vec::new();
        for (callee, spans) in groups {
            let Some(to) = self.doc_call_item(callee, files, cache) else { continue };
            let ranges: Vec<Json> = spans.iter().map(|&s| lines.range(s, files.positions)).collect();
            out.push(obj([("to", to), ("fromRanges", Json::Arr(ranges))]));
        }
        out
    }

    /// The classes declared in the indexed documents that extend `c`.
    fn document_implementations(&mut self, c: ClassId) -> Vec<Target> {
        let mut out = Vec::new();
        for ix in self.document_indexes() {
            for t in ix.defs.keys() {
                let DocTarget::Shared(Target::Class(k)) = *t else { continue };
                if k != c && self.syms.class(k).base_types.iter().any(|&(b, _)| b == c) && !out.contains(&Target::Class(k)) {
                    out.push(Target::Class(k));
                }
            }
        }
        out
    }

    /// A query on a file that is none of the program's: a library's document, whose extent is
    /// its occurrence index, `found` its occurrence at the query's offset
    /// (`document_target_at`); `null` for any other file and where none is found.
    fn document_answer(&mut self, q: &crate::index::Query, found: Option<(Span, DocTarget, std::sync::Arc<DocIndex>)>, files: &Files) -> Json {
        use crate::index::Query;
        let mut lines = LineCache { by_file: FxMap::default() };
        let Some((span, dt, ix)) = found else { return Json::Null };
        self.index_order();
        match q {
            Query::Definition(..) => self.doc_decl_location(dt, files, &mut lines).map_or(Json::Null, |l| Json::Arr(vec![l])),
            Query::References(_, _, declaration) => {
                let mut out = Vec::new();
                if *declaration {
                    out.extend(self.doc_decl_location(dt, files, &mut lines));
                }
                if let DocTarget::Shared(t) = dt {
                    let found: Vec<(FileId, Record)> = self.inverse().get(&t).cloned().unwrap_or_default();
                    for (f, r) in found {
                        if r.kind == Kind::Def {
                            continue;
                        }
                        if let Some(loc) = self.location_in(f, r.span, files, &mut lines) {
                            if !out.contains(&loc) {
                                out.push(loc);
                            }
                        }
                    }
                }
                for loc in self.document_use_locations(dt, &[], files) {
                    if !out.contains(&loc) {
                        out.push(loc);
                    }
                }
                Json::Arr(out)
            }
            Query::Hover(..) => {
                let shown = match dt {
                    DocTarget::Shared(t) => self.describe(t),
                    DocTarget::Local(file, addr) => match self.describe_local(file, addr) {
                        Some(s) => s,
                        None => return Json::Null,
                    },
                };
                let range = Lines::new(&ix.text).range(span, files.positions);
                obj([("contents", obj([("kind", "markdown".into()), ("value", format!("```scala\n{}\n```", shown).into())])), ("range", range)])
            }
            Query::Implementation(..) => {
                let DocTarget::Shared(t) = dt else { return Json::Arr(Vec::new()) };
                let mut found = self.implementations(t);
                if let Target::Class(c) = t {
                    found.extend(self.document_implementations(c));
                }
                Json::Arr(found.into_iter().filter_map(|t| self.decl_location(t, files, &mut lines)).collect())
            }
            Query::PrepareCall(..) if self.doc_is_callable(dt) => self.doc_call_item(dt, files, &mut lines).map_or(Json::Null, |i| Json::Arr(vec![i])),
            Query::Incoming(..) if self.doc_is_callable(dt) => {
                let mut out = match dt {
                    DocTarget::Shared(t) => self.incoming(t, files, &mut lines).arr().to_vec(),
                    DocTarget::Local(..) => Vec::new(),
                };
                out.extend(self.document_incoming(dt, files, &mut lines));
                Json::Arr(out)
            }
            Query::Outgoing(..) if self.doc_is_callable(dt) => Json::Arr(self.document_outgoing(dt, files, &mut lines)),
            _ => Json::Null,
        }
    }

    fn answer(&mut self, q: &crate::index::Query, files: &Files) -> Json {
        use crate::index::Query;
        let ids = q.file().map(|p| files.ids_of(p)).unwrap_or_default();
        if q.file().is_some() && ids.is_empty() {
            return Json::Null;
        }
        let mut lines = LineCache { by_file: FxMap::default() };
        match q {
            Query::Definition(_, offset) => {
                let Some((file, span, t, kind)) = self.target_at(&ids, *offset) else { return Json::Null };
                let mut out: Vec<Json> = self.decl_location(t, files, &mut lines).into_iter().collect();
                if let (Target::Sym(s), true) = (t, kind != Kind::Def) {
                    if let Some(own) = self.ascribed_override(file, span, s) {
                        out.extend(self.decl_location(Target::Sym(own), files, &mut lines));
                    }
                }
                match out.is_empty() {
                    true => Json::Null,
                    false => Json::Arr(out),
                }
            }
            Query::References(_, offset, declaration) => {
                let Some((_, _, t, _)) = self.target_at(&ids, *offset) else { return Json::Null };
                let mut out = Vec::new();
                if *declaration {
                    out.extend(self.decl_location(t, files, &mut lines));
                }
                let found: Vec<(FileId, Record)> = self.inverse().get(&t).cloned().unwrap_or_default();
                for (f, r) in found {
                    if r.kind == Kind::Def {
                        continue;
                    }
                    if let Some(loc) = self.location_in(f, r.span, files, &mut lines) {
                        if !out.contains(&loc) {
                            out.push(loc);
                        }
                    }
                }
                for loc in self.document_use_locations(DocTarget::Shared(t), &[], files) {
                    if !out.contains(&loc) {
                        out.push(loc);
                    }
                }
                Json::Arr(out)
            }
            Query::Hover(_, offset) => {
                let Some((_, span, t, _)) = self.target_at(&ids, *offset) else { return Json::Null };
                let text = self.describe(t);
                let range = ids.first().and_then(|&f| self.lines(f, files, &mut lines).map(|l| l.range(span, files.positions)));
                let mut fields = vec![("contents".to_string(), obj([("kind", "markdown".into()), ("value", format!("```scala\n{}\n```", text).into())]))];
                if let Some(r) = range {
                    fields.push(("range".to_string(), r));
                }
                Json::Obj(fields)
            }
            Query::Symbols(_) => self.document_symbols(&ids, files, &mut lines),
            Query::WorkspaceSymbols(query) => self.workspace_symbols(query, files, &mut lines),
            Query::Implementation(_, offset) => {
                let Some((_, _, t, _)) = self.target_at(&ids, *offset) else { return Json::Null };
                let mut found = self.implementations(t);
                if let Target::Class(c) = t {
                    found.extend(self.document_implementations(c));
                }
                Json::Arr(found.into_iter().filter_map(|t| self.decl_location(t, files, &mut lines)).collect())
            }
            Query::PrepareCall(_, offset) => {
                let Some(t) = self.callable_at(&ids, *offset) else { return Json::Null };
                match self.call_item(t, files, &mut lines) {
                    Some(item) => Json::Arr(vec![item]),
                    None => Json::Null,
                }
            }
            Query::Incoming(_, offset) => {
                let Some(t) = self.callable_at(&ids, *offset) else { return Json::Null };
                let mut out = self.incoming(t, files, &mut lines).arr().to_vec();
                out.extend(self.document_incoming(DocTarget::Shared(t), files, &mut lines));
                Json::Arr(out)
            }
            Query::Outgoing(_, offset) => {
                let Some(t) = self.callable_at(&ids, *offset) else { return Json::Null };
                match self.declaration(t) {
                    Some(_) => self.outgoing(t, files, &mut lines),
                    // A library method's calls, from its document's index.
                    None => Json::Arr(self.document_outgoing(DocTarget::Shared(t), files, &mut lines)),
                }
            }
            Query::Complete(..) | Query::Resolve(..) | Query::Signature(..) => Json::Null,
            Query::Stats => {
                let (library, std) = self.records_outside_sources();
                let Some(ix) = self.index.as_ref() else { return Json::Null };
                obj([
                    ("libraryRecords", library.into()),
                    ("stdRecords", std.into()),
                    ("records", ix.files.iter().map(Vec::len).sum::<usize>().into()),
                    ("nodes", ix.nodes.iter().map(Vec::len).sum::<usize>().into()),
                    ("catalogue", ix.catalogue.as_ref().map_or(0, |c| c.len()).into()),
                    ("journal", ix.journal.len().into()),
                    ("expansions", ix.expansions.len().into()),
                    ("bodies", ix.bodies.values().map(Vec::len).sum::<usize>().into()),
                    ("locals", ix.locals.len().into()),
                    ("sameParam", ix.same_param.len().into()),
                    ("paramsAt", ix.params_at.len().into()),
                    ("overrides", ix.overrides.values().map(Vec::len).sum::<usize>().into()),
                    ("tparams", ix.tparams.len().into()),
                    // The library documents indexed and the jars' source maps.
                    ("documents", Json::Arr(self.document_indexes().iter().map(|d| obj([
                        ("path", d.path.display().to_string().into()),
                        ("files", d.files.len().into()),
                        ("records", d.records.len().into()),
                        ("bytes", d.held().into()),
                        ("trees", d.stats.trees.into()),
                        ("candidates", d.stats.candidates.into()),
                        ("unresolved", d.stats.unresolved.into()),
                        ("unspelled", d.stats.unspelled.into()),
                        ("buildMs", Json::Num(d.stats.build_ms)),
                    ])).collect())),
                    ("sourceMaps", Json::Arr(self.loaded.as_ref().map(|l| l.source_map_stats.iter().map(|&(jar, files, bytes, ms)| obj([
                        ("jar", l.cp.paths[jar as usize].clone().into()),
                        ("files", files.into()),
                        ("bytes", bytes.into()),
                        ("sources", l.source_maps.get(&jar).map_or(0, |m| m.len()).into()),
                        ("ms", Json::Num(ms)),
                    ])).collect()).unwrap_or_default())),
                ])
            }
        }
    }

    /// How many records name a target declared in a jar (a class, a member, a library body's
    /// definition) and how many one of the std: what `definition` cannot locate in a source.
    fn records_outside_sources(&self) -> (usize, usize) {
        let Some(ix) = self.index.as_ref() else { return (0, 0) };
        let (mut library, mut std) = (0, 0);
        for r in ix.files.iter().flatten() {
            let file = match r.target {
                Target::Sym(s) => self.syms.sym(s).file,
                Target::Class(c) => self.syms.class(c).file,
                Target::Alias(a) => self.syms.aliases[a.idx()].file,
                Target::TParam(_) | Target::Node(_) => continue,
            };
            if self.in_jar(file) || self.is_body_file(file) {
                library += 1;
            } else if self.source(file).is_std {
                std += 1;
            }
        }
        (library, std)
    }

    /// The lines of a file a location is answered in: a std file's document made to hold its
    /// text then (written the first time, replaced where it was truncated or altered since,
    /// `attach::write_document`), else no location.
    fn lines<'l>(&self, f: FileId, files: &Files, cache: &'l mut LineCache<'a>) -> Option<&'l Lines<'a>> {
        let path = files.path(f)?;
        let sources: &'a crate::source::Sources = self.files;
        let text: &'a str = &sources[f.0 as usize].text;
        if !cache.by_file.contains_key(&f) && self.std_file(f) && !super::loader::attach::write_document(path, text) {
            return None;
        }
        Some(cache.by_file.entry(f).or_insert_with(|| Lines::new(text)))
    }

    fn location_in(&self, f: FileId, span: Span, files: &Files, cache: &mut LineCache<'a>) -> Option<Json> {
        let path = files.path(f)?.clone();
        let lines = self.lines(f, files, cache)?;
        Some(location(&path, lines, span, files.positions))
    }

    /// The location of a target's declaration: in a program file, else in its library's
    /// document, `None` where it has neither.
    fn decl_location(&mut self, t: Target, files: &Files, cache: &mut LineCache<'a>) -> Option<Json> {
        match self.declaration(t) {
            Some((f, span)) => self.location_in(f, span, files, cache),
            None => {
                let (doc, span) = self.library_declaration(t)?;
                Some(location(&doc.path, &Lines::new(&doc.text), span, files.positions))
            }
        }
    }

    /// What hover shows of a target: its kind and qualified name with its signature, a class's
    /// parents, an alias's right-hand side.
    pub(super) fn describe(&mut self, t: Target) -> String {
        match t {
            Target::Sym(s) => {
                let info = self.syms.sym(s).info.clone();
                let name = self.name_str(info.name);
                let qualified = match info.owner {
                    Owner::Class(c) => format!("{}.{}", self.class_path(c), name),
                    Owner::Package(p) if p != ROOT_PKG => format!("{}.{}", self.pkg_description(p), name),
                    _ => name,
                };
                let keyword = match info.kind {
                    SymKind::Def => "def",
                    SymKind::Var => "var",
                    SymKind::Given => "given",
                    SymKind::EnumValue(_) => "case",
                    SymKind::Param => "",
                    _ if info.mods & crate::ast::mods::LAZY != 0 => "lazy val",
                    _ => "val",
                };
                let sig = self.sig_arc(s);
                let sig = if matches!(info.kind, SymKind::Def | SymKind::Given) { self.sig_text(&sig) } else { format!(": {}", self.show(sig.ret)) };
                let sig = if sig.starts_with(": ") || sig.starts_with('[') || sig.starts_with('(') { sig } else { format!(": {}", sig) };
                let keyword = if keyword.is_empty() { String::new() } else { format!("{} ", keyword) };
                format!("{}{}{}", keyword, qualified, sig)
            }
            Target::Class(c) => {
                let info = self.syms.class(c).info.clone();
                let keyword = match info.kind {
                    ClassKind::Trait => "trait",
                    ClassKind::Object => "object",
                    ClassKind::Enum => "enum",
                    ClassKind::EnumCase => "case",
                    _ if info.mods & crate::ast::mods::CASE != 0 => "case class",
                    _ => "class",
                };
                let mut out = format!("{} {}", keyword, self.class_path(c));
                let tparams: Vec<String> = info.own_tparams().iter().map(|&p| self.name_str(self.syms.tparam(p).name)).collect();
                if !tparams.is_empty() {
                    out.push_str(&format!("[{}]", tparams.join(", ")));
                }
                let parents: Vec<String> = info.parents.iter().map(|&p| self.show(p)).filter(|p| p != "Object" && p != "AnyRef" && p != "Any").collect();
                if !parents.is_empty() {
                    out.push_str(&format!(" extends {}", parents.join(", ")));
                }
                out
            }
            Target::Alias(a) => {
                let (name, rhs, bounds) = {
                    let info = &self.syms.aliases[a.idx()];
                    (info.name, info.rhs, info.bounds)
                };
                match bounds {
                    Some((lo, hi)) => {
                        let (lo, hi) = (self.show(lo), self.show(hi));
                        format!("type {} >: {} <: {}", self.name_str(name), lo, hi)
                    }
                    None => format!("type {} = {}", self.name_str(name), self.show(rhs)),
                }
            }
            Target::TParam(p) => {
                let (name, upper) = (self.syms.tparam(p).name, self.syms.tparam(p).upper);
                if upper == ANY {
                    format!("type {}", self.name_str(name))
                } else {
                    format!("type {} <: {}", self.name_str(name), self.show(upper))
                }
            }
            Target::Node(_) => String::new(),
        }
    }

    fn document_symbols(&mut self, ids: &[FileId], files: &Files, cache: &mut LineCache<'a>) -> Json {
        let mut out = Vec::new();
        for &f in ids {
            let ast = self.ast(f);
            for &d in &ast.top_level {
                out.extend(self.document_symbol(f, d, true, files, cache));
            }
        }
        Json::Arr(out)
    }

    fn document_symbol(&mut self, f: FileId, d: ast::DefId, top: bool, files: &Files, cache: &mut LineCache<'a>) -> Option<Json> {
        let ast = self.ast(f);
        let def = ast.def(d);
        let name = self.name_str(def.name);
        if name.contains('$') || ast.def_ranges.get(d.idx()).map_or(true, |&r| r == ast::NO_RANGE) {
            return None;
        }
        let selection = self.name_at(f, def.span.start, def.name).unwrap_or(def.span);
        let range = ast.def_range(d);
        let range = Span::new(range.start.min(selection.start), range.end.max(selection.end));
        let (kind, body): (u32, &[ast::Stmt]) = match &def.kind {
            DefKind::Class(c) => (
                match c.kind {
                    ast::ClassKind::Trait => 11,
                    ast::ClassKind::Object => 2,
                    ast::ClassKind::Enum => 10,
                    ast::ClassKind::EnumCase => 22,
                    ast::ClassKind::Class => 5,
                },
                &c.body,
            ),
            DefKind::Given(g) => (19, &g.body),
            DefKind::Fun(_) if def.name == crate::names::INIT => (9, &[]),
            DefKind::Fun(_) => (if top { 12 } else { 6 }, &[]),
            DefKind::Val { .. } if def.mods & crate::ast::mods::MUTABLE != 0 => (13, &[]),
            DefKind::Val { .. } => (if top { 13 } else { 8 }, &[]),
            DefKind::TypeAlias { .. } => (26, &[]),
        };
        let detail = match (&def.kind, self.def_syms.get(f.0 as usize, &d).copied()) {
            (DefKind::Fun(_), Some(s)) => {
                let sig = self.sig_arc(s);
                Some(self.sig_text(&sig))
            }
            (DefKind::Val { .. }, Some(s)) => {
                let sig = self.sig_arc(s);
                Some(self.show(sig.ret))
            }
            _ => None,
        };
        let mut children = Vec::new();
        for stmt in body {
            if let ast::Stmt::Def(c) = *stmt {
                children.extend(self.document_symbol(f, c, false, files, cache));
            }
        }
        let lines = self.lines(f, files, cache)?;
        let mut fields = vec![("name".to_string(), Json::from(name))];
        if let Some(detail) = detail {
            fields.push(("detail".to_string(), detail.into()));
        }
        fields.push(("kind".to_string(), kind.into()));
        fields.push(("range".to_string(), lines.range(range, files.positions)));
        fields.push(("selectionRange".to_string(), lines.range(selection, files.positions)));
        if !children.is_empty() {
            fields.push(("children".to_string(), Json::Arr(children)));
        }
        Some(Json::Obj(fields))
    }

    fn workspace_symbols(&mut self, query: &str, files: &Files, cache: &mut LineCache<'a>) -> Json {
        let query = query.to_lowercase();
        let mut found: Vec<(bool, String, Target)> = Vec::new();
        let matches = |name: &str| {
            let lower = name.to_lowercase();
            lower.contains(&query).then(|| !lower.starts_with(&query))
        };
        // The program's: a std file has a path for its document, and is no program's.
        let program = |f: FileId| files.path(f).is_some() && !self.std_file(f);
        for (i, info) in self.syms.classes.iter().enumerate() {
            if info.def.is_none() || info.owner == Owner::Local || !program(info.file) || info.kind == ClassKind::Anon || !self.fresh_class(ClassId(i as u32)) {
                continue;
            }
            let name = self.name_ref(info.name);
            if name.contains('$') {
                continue;
            }
            if let Some(later) = matches(name) {
                found.push((later, name.to_string(), Target::Class(ClassId(i as u32))));
            }
        }
        for (i, info) in self.syms.syms.iter().enumerate() {
            if info.def.is_none() || info.owner == Owner::Local || !program(info.file) || !self.fresh_sym(SymId(i as u32)) {
                continue;
            }
            if matches!(info.kind, SymKind::Param | SymKind::Object(_) | SymKind::Overloaded(_)) {
                continue;
            }
            let name = self.name_ref(info.name);
            if name.contains('$') || name == "<init>" {
                continue;
            }
            if let Some(later) = matches(name) {
                found.push((later, name.to_string(), Target::Sym(SymId(i as u32))));
            }
        }
        for (i, info) in self.syms.aliases.iter().enumerate() {
            if info.def.is_none() || info.owner == Owner::Local || !program(info.file) || !self.fresh_alias(AliasId(i as u32)) {
                continue;
            }
            let name = self.name_ref(info.name);
            if let Some(later) = matches(name) {
                found.push((later, name.to_string(), Target::Alias(AliasId(i as u32))));
            }
        }
        found.sort_by(|a, b| (a.0, a.1.to_lowercase()).cmp(&(b.0, b.1.to_lowercase())));
        let mut out = Vec::new();
        for (_, name, t) in found {
            if out.len() >= 200 {
                break;
            }
            let Some(loc) = self.decl_location(t, files, cache) else { continue };
            let (kind, container) = match t {
                Target::Class(c) => {
                    let info = self.syms.class(c);
                    let kind = match info.kind {
                        ClassKind::Trait => 11u32,
                        ClassKind::Object => 2,
                        ClassKind::Enum => 10,
                        ClassKind::EnumCase => 22,
                        _ => 5,
                    };
                    (kind, self.owner_name(info.owner))
                }
                Target::Sym(s) => {
                    let info = self.syms.sym(s);
                    let kind = match info.kind {
                        SymKind::Def => 6u32,
                        SymKind::EnumValue(_) => 22,
                        SymKind::Var => 13,
                        _ => 8,
                    };
                    (kind, self.owner_name(info.owner))
                }
                Target::Alias(a) => (26, self.owner_name(self.syms.aliases[a.idx()].owner)),
                Target::TParam(_) | Target::Node(_) => continue,
            };
            let mut fields = vec![("name".to_string(), Json::from(name)), ("kind".to_string(), kind.into()), ("location".to_string(), loc)];
            if let Some(c) = container {
                fields.push(("containerName".to_string(), c.into()));
            }
            out.push(Json::Obj(fields));
        }
        Json::Arr(out)
    }

    fn owner_name(&self, owner: Owner) -> Option<String> {
        match owner {
            Owner::Class(c) => Some(self.class_path(c)),
            Owner::Package(p) if p != ROOT_PKG => Some(self.pkg_description(p)),
            _ => None,
        }
    }

    /// A class's descendants in the program, or the members that override a member.
    fn implementations(&self, t: Target) -> Vec<Target> {
        let descendants = |w: &Self, c: ClassId| -> Vec<ClassId> {
            (0..w.syms.classes.len() as u32)
                .map(ClassId)
                .filter(|&d| d != c && w.syms.class(d).def.is_some() && w.fresh_class(d) && w.syms.class(d).base_types.iter().any(|&(b, _)| b == c))
                .collect()
        };
        match t {
            Target::Class(c) => descendants(self, c).into_iter().map(Target::Class).collect(),
            // The members the override check found to override it, in classes of the program
            // as they now stand.
            Target::Sym(m) => {
                let found = self.index.as_ref().and_then(|ix| ix.overrides.get(&m)).cloned().unwrap_or_default();
                found.into_iter().filter(|&o| self.fresh_sym(o)).map(Target::Sym).collect()
            }
            _ => Vec::new(),
        }
    }

    // ---- the call hierarchy ----

    /// The method at `offset` (declared or called there), else the innermost definition that
    /// encloses it.
    fn callable_at(&self, ids: &[FileId], offset: u32) -> Option<Target> {
        if let Some((_, _, t, _)) = self.target_at(ids, offset) {
            if self.is_callable(t) {
                return Some(t);
            }
        }
        for &f in ids {
            if let Some(t) = self.owner_at(f, Span::new(offset, offset)) {
                return Some(t);
            }
        }
        None
    }

    fn is_callable(&self, t: Target) -> bool {
        matches!(t, Target::Sym(s) if self.syms.sym(s).kind == SymKind::Def)
    }

    /// The innermost definition of `file` whose range holds `span` and that has a symbol: a
    /// method, a val, or a class whose initialiser it is part of.
    fn owner_at(&self, file: FileId, span: Span) -> Option<Target> {
        let ast = self.ast(file);
        let mut best: Option<(Span, Target)> = None;
        for (i, &range) in ast.def_ranges.iter().enumerate() {
            if range == ast::NO_RANGE || range.start > span.start || range.end < span.end {
                continue;
            }
            if best.map_or(false, |(b, _)| range.end - range.start >= b.end - b.start) {
                continue;
            }
            let d = ast::DefId(i as u32);
            let t = match (self.def_syms.get(file.0 as usize, &d), self.def_classes.get(file.0 as usize, &d)) {
                (Some(&s), _) if !matches!(self.syms.sym(s).kind, SymKind::Object(_)) => Target::Sym(s),
                (_, Some(&c)) => Target::Class(c),
                (Some(&s), None) => self.normal_target(Target::Sym(s)),
                _ => continue,
            };
            best = Some((range, t));
        }
        best.map(|(_, t)| t)
    }

    /// A call hierarchy item: the target's definition in a program file, its range the
    /// definition's, else its declaration in a library's document, its range
    /// the name's.
    fn call_item(&mut self, t: Target, files: &Files, cache: &mut LineCache<'a>) -> Option<Json> {
        let (name, kind, detail) = match t {
            Target::Sym(s) => {
                let info = self.syms.sym(s).info.clone();
                let kind = match (info.kind, info.owner) {
                    (SymKind::Def, Owner::Class(_)) => 6u32,
                    (SymKind::Def, _) => 12,
                    _ => 13,
                };
                (self.name_str(info.name), kind, self.owner_name(info.owner))
            }
            Target::Class(c) => {
                let info = self.syms.class(c);
                (self.name_str(info.name), 5, self.owner_name(info.owner))
            }
            _ => return None,
        };
        let (uri, range, selection) = match self.declaration(t) {
            Some((f, selection)) => {
                let range = match t {
                    Target::Sym(s) => self.syms.sym(s).def.map(|d| self.ast(f).def_range(d)),
                    Target::Class(c) => self.syms.class(c).def.map(|d| self.ast(f).def_range(d)),
                    _ => None,
                }
                .unwrap_or(selection);
                let range = Span::new(range.start.min(selection.start), range.end.max(selection.end));
                let path = files.path(f)?.clone();
                let lines = self.lines(f, files, cache)?;
                (crate::lsp::uri::from_path(&path), lines.range(range, files.positions), lines.range(selection, files.positions))
            }
            None => {
                let (doc, selection) = self.library_declaration(t)?;
                let lines = Lines::new(&doc.text);
                let range = lines.range(selection, files.positions);
                (crate::lsp::uri::from_path(&doc.path), range.clone(), range)
            }
        };
        let mut fields = vec![("name".to_string(), Json::from(name)), ("kind".to_string(), kind.into())];
        if let Some(d) = detail {
            fields.push(("detail".to_string(), d.into()));
        }
        fields.push(("uri".to_string(), uri.into()));
        fields.push(("range".to_string(), range));
        fields.push(("selectionRange".to_string(), selection));
        Some(Json::Obj(fields))
    }

    fn incoming(&mut self, t: Target, files: &Files, cache: &mut LineCache<'a>) -> Json {
        let calls: Vec<(FileId, Record)> = self.inverse().get(&t).cloned().unwrap_or_default();
        let mut groups: Vec<(Target, FileId, Vec<Span>)> = Vec::new();
        for (f, r) in calls {
            if r.kind != Kind::Call {
                continue;
            }
            let Some(owner) = self.owner_at(f, r.span) else { continue };
            match groups.iter_mut().find(|(o, _, _)| *o == owner) {
                Some((_, _, spans)) => spans.push(r.span),
                None => groups.push((owner, f, vec![r.span])),
            }
        }
        let mut out = Vec::new();
        for (owner, f, spans) in groups {
            let Some(from) = self.call_item(owner, files, cache) else { continue };
            let Some(lines) = self.lines(f, files, cache) else { continue };
            let ranges: Vec<Json> = spans.iter().map(|&s| lines.range(s, files.positions)).collect();
            out.push(obj([("from", from), ("fromRanges", Json::Arr(ranges))]));
        }
        Json::Arr(out)
    }

    fn outgoing(&mut self, t: Target, files: &Files, cache: &mut LineCache<'a>) -> Json {
        let Some((f, _)) = self.declaration(t) else { return Json::Arr(Vec::new()) };
        let range = match t {
            Target::Sym(s) => self.syms.sym(s).def.map(|d| self.ast(f).def_range(d)),
            Target::Class(c) => self.syms.class(c).def.map(|d| self.ast(f).def_range(d)),
            _ => None,
        };
        let Some(range) = range else { return Json::Arr(Vec::new()) };
        let records: Vec<Record> = self.index.as_ref().and_then(|ix| ix.files.get(f.0 as usize)).map_or(Vec::new(), |r| {
            r.iter().filter(|r| r.kind == Kind::Call && r.span.start >= range.start && r.span.end <= range.end).copied().collect()
        });
        let mut groups: Vec<(Target, Vec<Span>)> = Vec::new();
        for r in records {
            if self.owner_at(f, r.span) != Some(t) {
                continue;
            }
            match groups.iter_mut().find(|(c, _)| *c == r.target) {
                Some((_, spans)) => spans.push(r.span),
                None => groups.push((r.target, vec![r.span])),
            }
        }
        let mut out = Vec::new();
        for (callee, spans) in groups {
            let Some(to) = self.call_item(callee, files, cache) else { continue };
            let Some(lines) = self.lines(f, files, cache) else { continue };
            let ranges: Vec<Json> = spans.iter().map(|&s| lines.range(s, files.positions)).collect();
            out.push(obj([("to", to), ("fromRanges", Json::Arr(ranges))]));
        }
        Json::Arr(out)
    }
}

/// Whether an operator's right operand, which starts at `start` of `text`, stands in
/// parentheses that open after the operator's name, which ends at `op_end`: blanks and comments
/// between, as signature help reads them (`typer::signature`), or the operand's own span opening
/// with them (a tuple's).
fn parenthesised(text: &str, op_end: u32, start: u32) -> bool {
    let open = super::signature::skip_trivia(text, op_end, start);
    text.as_bytes().get(open as usize) == Some(&b'(') && (open == start || super::signature::blank(text, open + 1, start))
}
