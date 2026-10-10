//! Completion over the typed program (docs/TARGETS.md, "The language server"). The site at the
//! cursor is read from a reparse of the request's text (`crate::complete`); the rest from what the
//! build keeps. The scopes around the offset are rebuilt as the environment the typer has there:
//! the enclosing classes' frames and imports as `env_at` gives them, then a frame per method,
//! block, lambda, case and `for` between the innermost class and the offset, read from the file's
//! tree, their binders' symbols from the index's declarations of the locals, which a retype of
//! the file replaces. The receiver of a selection is the index's node of the file's latest typing
//! whose span is the receiver's, and its type the one the typer recorded for it.
//!
//! Every name gathered is resolved again by the typer's own lookup in that environment, and
//! dropped where it finds nothing, finds something else or reports an ambiguity; members are
//! found by `find_member` and kept where the selection's access rules admit them from the site.
//! It all runs in query mode (`Worker::in_query`): the index is set aside, so nothing records
//! into it; what the lookups and lazy loading report is dropped; and the constraints a
//! conformance check made are rolled back, so that a completion leaves the session answering as
//! it did.

use super::apply::MethodCall;
use super::index::Files;
use super::resolve::{TermRef, TypeRef};
use super::{Env, Frame, ImportTarget, ResolvedImport, Worker};
use crate::ast::{self, Ast, DefId, DefKind, Expr, ExprId, Pat, PatId, Stmt};
use crate::complete::{Context, Site, TArg};
use crate::index::Lines;
use crate::intern::{FxMap, Name};
use crate::lsp::json::{obj, Json};
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::types::*;
use std::path::Path;

/// How many items a list holds at most; a longer one is cut and marked incomplete.
const CAP: usize = 1000;

/// The groups the items are ranked by, nearest first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) enum Rank {
    Local,
    Member,
    Import,
    Package,
    Root,
    Extension,
    Conversion,
    Keyword,
    /// A name out of the scope, offered with the import that brings it.
    Unimported,
}

/// What an item names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Cand {
    Sym(SymId),
    Class(ClassId),
    Alias(AliasId),
    Package(PkgId),
    TParam(TParamId),
    /// The `self =>` alias of an enclosing class.
    SelfAlias(ClassId),
    /// A member every value has (`toString`, a number's `toLong`), with its signature.
    Builtin(&'static str),
    Keyword,
    /// A name out of the scope (`Candidate::import`).
    Unimported,
}

/// An item before it is written: its spelling, its target, its group and what it shows.
pub(super) struct Candidate {
    pub name: String,
    pub target: Cand,
    pub rank: Rank,
    pub detail: String,
    pub kind: u32,
    /// For a name out of the scope: the path its import names and the package or object that
    /// holds it.
    pub import: Option<(String, String)>,
    /// The parameter clauses a call of the item writes: a method's, a class's constructor's
    /// after `new`; none for anything else.
    pub clauses: Vec<CallClause>,
}

/// A parameter clause of an item's call, as the item's detail prints it: a `using` (or
/// `implicit`) one, whose arguments the compiler finds; one of an extension's receiver, those
/// before its method's name, which a selection binds.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CallClause {
    pub using: bool,
    pub receiver: bool,
    pub params: Vec<CallParam>,
}

/// A parameter of a call's clause: its name, whether it is repeated, of a function type, or has
/// a default.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CallParam {
    pub name: Name,
    pub repeated: bool,
    pub function: bool,
    pub default: bool,
}

/// A name a completion may offer from outside the scope, with the import that brings it: a
/// class, an object, a type alias or another member of a package, or a class or an object nested
/// in an object, of the program, the std or the class path.
pub(super) struct Entry {
    /// The spelling in lower case, which the catalogue is ordered and searched by.
    lower: Box<str>,
    name: Box<str>,
    /// The path the import names, `java.util.UUID`, and the package or object that holds it.
    path: Box<str>,
    owner: Box<str>,
    kind: u32,
    /// 0 the program, 1 the std, 2 the class path: the order the groups rank in.
    origin: u8,
}

/// The names of the std and the class path a completion offers with an import: a sorted vector
/// searched by prefix, built once per typed program on the first completion that wants it. The
/// program's own are read from the symbol table at each completion, as its latest typing left it.
pub struct Catalogue {
    external: Vec<Entry>,
}

impl Catalogue {
    pub fn len(&self) -> usize {
        self.external.len()
    }

    pub fn held(&self) -> usize {
        crate::held::array(&self.external) + self.external.iter().map(|e| e.lower.len() + e.name.len() + e.path.len() + e.owner.len()).sum::<usize>()
    }
}

/// An entry of the catalogue; `owner` is the path of the package or object that holds it, its
/// segments `encode`d.
fn entry(name: &str, owner: &str, kind: u32, origin: u8) -> Entry {
    Entry { lower: name.to_lowercase().into(), name: name.into(), path: format!("{}.{}", owner, encode(name)).into(), owner: owner.into(), kind, origin }
}

/// A segment of an import path as `data` carries it: its `.`, its spaces and its `%` escaped, so
/// that the path splits back into its segments.
fn encode(segment: &str) -> String {
    segment.replace('%', "%25").replace('.', "%2E").replace(' ', "%20")
}

fn decode(segment: &str) -> String {
    segment.replace("%20", " ").replace("%2E", ".").replace("%25", "%")
}

/// A path as source reads it, its segments backquoted where they need to be.
fn shown_path(path: &str) -> String {
    path.split('.').map(|s| spelled(&decode(s))).collect::<Vec<_>>().join(".")
}

/// The entries of `entries` (ordered by `lower`) whose spelling starts with `prefix`, lower case.
fn with_prefix<'e>(entries: &'e [Entry], prefix: &str) -> &'e [Entry] {
    let from = entries.partition_point(|e| &*e.lower < prefix);
    let to = from + entries[from..].partition_point(|e| e.lower.starts_with(prefix));
    &entries[from..to]
}

/// The locals and type parameters the index declares in the file, by the start of their names:
/// what the binders of the tree around the offset are; and the first symbol and class of the
/// file's latest typing, before which a local one is a body's typed again since.
pub(super) struct Binders {
    locals: Vec<(u32, Name, SymId)>,
    tparams: Vec<(u32, Name, TParamId)>,
    fresh: (u32, u32),
    /// The index's block imports, resolved once (`Index::block_imports`), out of it while the
    /// query runs.
    imports: std::cell::RefCell<FxMap<(FileId, u32), Vec<ResolvedImport>>>,
}

impl Binders {
    fn local(&self, name: Name, within: Span) -> Option<SymId> {
        let from = self.locals.partition_point(|&(s, _, _)| s < within.start);
        self.locals[from..].iter().take_while(|&&(s, _, _)| s <= within.end).find(|&&(_, n, _)| n == name).map(|&(_, _, s)| s)
    }

    fn tparam(&self, name: Name, within: Span) -> Option<TParamId> {
        let from = self.tparams.partition_point(|&(s, _, _)| s < within.start);
        self.tparams[from..].iter().take_while(|&&(s, _, _)| s <= within.end).find(|&&(_, n, _)| n == name).map(|&(_, _, p)| p)
    }
}

fn holds(span: Span, offset: u32) -> bool {
    span.start <= offset && offset <= span.end
}

/// What the scope makes of a spelling (`Worker::spelling_state`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Spelling {
    Free,
    Bound,
    Ambiguous,
}

/// Where an extension method was found, which says what its owner's parameters are.
#[derive(Clone, Copy)]
enum Found {
    Lexical,
    /// In the receiver's implicit scope, through that object.
    Module(Option<super::implicits::GivenScope>),
    /// A member of a given's type, its owner's type as seen from that type.
    Given(TypeId),
}

/// The LSP kinds of the items.
mod kind {
    pub const METHOD: u32 = 2;
    pub const FUNCTION: u32 = 3;
    pub const CONSTRUCTOR: u32 = 4;
    pub const FIELD: u32 = 5;
    pub const VARIABLE: u32 = 6;
    pub const CLASS: u32 = 7;
    pub const INTERFACE: u32 = 8;
    pub const MODULE: u32 = 9;
    pub const ENUM: u32 = 13;
    pub const KEYWORD: u32 = 14;
    pub const ENUM_MEMBER: u32 = 20;
    pub const TYPE_PARAMETER: u32 = 25;
}

/// The members every value has, which the selection path adds where the class declares none,
/// with the signatures shown for them.
const UNIVERSAL: &[(&str, &str)] = &[
    ("==", "(x: Any): Boolean"),
    ("!=", "(x: Any): Boolean"),
    ("##", ": Int"),
    ("asInstanceOf", "[T]: T"),
    ("equals", "(x: Any): Boolean"),
    ("hashCode", "(): Int"),
    ("isInstanceOf", "[T]: Boolean"),
    ("toString", "(): String"),
];

/// The members of a reference type besides, and the conversions of a number.
const REFERENCE: &[(&str, &str)] = &[("eq", "(x: AnyRef): Boolean"), ("ne", "(x: AnyRef): Boolean"), ("synchronized", "[T](x: T): T")];
const NUMERIC: &[(&str, &str)] = &[
    ("toByte", ": Byte"),
    ("toChar", ": Char"),
    ("toDouble", ": Double"),
    ("toFloat", ": Float"),
    ("toInt", ": Int"),
    ("toLong", ": Long"),
    ("toShort", ": Short"),
];

impl<'a> Worker<'a> {
    /// The answer to `complete <path> <offset> <flags>`: a `CompletionList`. The flags' bit 0
    /// says the client takes insert-and-replace edits, bit 1 that it takes snippets.
    pub(super) fn complete(&mut self, path: &Path, offset: u32, flags: u32, files: &Files) -> Json {
        let ids = files.ids_of(path);
        let Some(&first) = ids.first() else { return list(false, Vec::new()) };
        let sources: &'a crate::source::Sources = self.files;
        let text: &'a str = &sources[first.0 as usize].text;
        let cx = crate::complete::context(text, offset);
        if cx.site == Site::Nothing {
            return list(false, Vec::new());
        }
        let file = self.file_at(&ids, cx.insert.start);
        // Where an item may write a call: a name not applied yet, as a term where no function is
        // expected (which the name alone gives), or a class after `new`. No name after a dot is
        // the selection's without one, recorded after its dot.
        let name = cx.dot.map_or(cx.replace, |d| Span::new(d, d));
        let calls = flags & 2 != 0
            && !cx.applied
            && match cx.site {
                Site::Scope | Site::Member { .. } => !self.index_expects_function(&ids, name),
                Site::New { .. } => true,
                _ => false,
            };
        let (candidates, function) = self.candidates(file, &ids, &cx, calls);
        self.render(candidates, &cx, text, flags, calls && !function, path, files)
    }

    /// The answer to `complete-resolve <path> <offset> <generation> <name>`: `{"modified":true}`
    /// where the program is no longer the one the item was offered from, else the edits that
    /// import `name` for the site at `offset`, none (with a warning) where the import does not
    /// resolve there.
    pub(super) fn complete_resolve(&mut self, path: &Path, offset: u32, generation: &str, name: &str, files: &Files) -> Json {
        if generation != current_generation() {
            return obj([("modified", true.into())]);
        }
        let ids = files.ids_of(path);
        // A std document is read-only: no import is written into it.
        let Some(&first) = ids.first().filter(|&&f| !self.std_file(f)) else { return obj([("edits", Json::Arr(Vec::new()))]) };
        let sources: &'a crate::source::Sources = self.files;
        let text: &'a str = &sources[first.0 as usize].text;
        let file = self.file_at(&ids, offset);
        let (at, before, after) = import_insertion(self.ast(file), self.ast(first), text, offset);
        let segments: Vec<String> = name.split('.').map(decode).collect();
        let base = Env { file, frames: Vec::new(), imports: Vec::new() };
        let checked = self.in_query(base, |w| w.import_check(&segments));
        let Some(rooted) = checked else {
            let warning = format!("completion: the import of {} does not resolve, or leaves its name ambiguous, in {}", name, path.display());
            return obj([("edits", Json::Arr(Vec::new())), ("warning", warning.into())]);
        };
        let written: Vec<String> = segments.iter().map(|s| spelled(s)).collect();
        let import = format!("{}import {}{}{}", before, if rooted { "_root_." } else { "" }, written.join("."), after);
        let range = Lines::new(text).range(Span::new(at, at), files.positions);
        obj([("edits", Json::Arr(vec![obj([("range", range), ("newText", import.into())])]))])
    }

    /// Whether the import of the path `segments` resolves at the top of the file, a package's,
    /// an object's or a member's, and whether it must be written from `_root_` there: where its
    /// first segment names something else than the top-level package. `None` where it resolves to
    /// nothing.
    fn import_check(&mut self, segments: &[String]) -> Option<bool> {
        let (head, top, last, target, ty, term) = self.import_target(segments)?;
        // The spelling with the import among the file's: it has to name what the import brings,
        // with nothing else of its name at that level to make it ambiguous.
        if !self.imported_alone(last, target, ty, term) {
            return None;
        }
        let here = self.quietly(|w| w.lookup_term_at(head, Span::new(0, 0)));
        let same = matches!((here, top), (Some(TermRef::Package(a)), TermRef::Package(b)) if a == b);
        Some(!same)
    }

    /// What an import of the path `segments` brings, from the root: its first segment and the
    /// top-level package that names, its last segment, the import's target and what it names as
    /// a type and as a term; `None` where it names nothing or passes through or to something
    /// private or protected.
    #[allow(clippy::type_complexity)]
    fn import_target(&mut self, segments: &[String]) -> Option<(Name, TermRef, Name, ImportTarget, Option<TypeRef>, Option<TermRef>)> {
        let names: Vec<Name> = segments.iter().map(|s| self.interner.intern(s)).collect();
        let (&head, rest) = names.split_first()?;
        let Some((&last, middle)) = rest.split_last() else { return None };
        let top = self.quietly(|w| w.pkg_term(ROOT_PKG, head))?;
        let mut at = top;
        for &n in middle {
            at = match at {
                TermRef::Package(p) => self.quietly(|w| w.pkg_term(p, n))?,
                other => {
                    let c = self.path_object(other)?;
                    self.quietly(|w| w.module_term(c, n))?
                }
            };
            if !self.public_term(at) {
                return None;
            }
        }
        let (ty, term) = match at {
            TermRef::Package(p) => (self.quietly(|w| w.pkg_type(p, last)), self.quietly(|w| w.pkg_term(p, last))),
            other => {
                let c = self.path_object(other)?;
                (self.quietly(|w| w.module_type(c, last)), self.quietly(|w| w.module_term(c, last)))
            }
        };
        // What the import brings must be reached from outside its owner: no private or
        // protected member or class.
        let public_type = |w: &Self, r: TypeRef| match r {
            TypeRef::Class(c) => w.syms.class(c).mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) == 0,
            TypeRef::Alias(a) => {
                let (file, def) = (w.syms.aliases[a.idx()].file, w.syms.aliases[a.idx()].def);
                def.map_or(true, |d| w.ast(file).def(d).mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) == 0)
            }
            _ => false,
        };
        let found = ty.is_some_and(|r| public_type(self, r)) || term.is_some_and(|r| self.public_term(r));
        if !found {
            return None;
        }
        let target = match at {
            TermRef::Package(p) => ImportTarget::PkgMember(p, last),
            other => ImportTarget::ClassMember(self.path_object(other)?, last),
        };
        Some((head, top, last, target, ty, term))
    }

    /// Whether `name`, with an import of `target` added as the last of the environment's tree
    /// (the file's, or a `package p:` block's), resolves without a report to what the import
    /// brings, `ty` as a type and `term` as a term, where the environment stands.
    fn imported_alone(&mut self, name: Name, target: ImportTarget, ty: Option<TypeRef>, term: Option<TermRef>) -> bool {
        let f = self.env.file.0 as usize;
        self.import_count();
        let saved = self.file_imports[f].clone();
        let mut imports: Vec<ResolvedImport> = saved.as_deref().cloned().unwrap_or_default();
        // A file's named imports come first, the latest at the front (`resolve_file_imports`).
        imports.insert(0, ResolvedImport { name: Some(name), target, hidden: ast::ListRef::EMPTY, bound: None, depth: 0, stmt: u32::MAX, unimports_predef: None, sel: super::unused::SelRef::NONE });
        self.file_imports[f] = Some(std::sync::Arc::new(imports));
        let at = Span::new(0, 0);
        let key = |r: TermRef| match r {
            TermRef::Class(c) => (0, c.0),
            TermRef::Package(p) => (1, p.0),
            other => (2, other.sym().map_or(u32::MAX, |s| s.0)),
        };
        let ty_ok = ty.map_or(true, |t| self.quietly(|w| w.lookup_type_at(name, at)) == Some(t));
        let term_ok = term.map_or(true, |t| self.quietly(|w| w.lookup_term_at(name, at)).map(key) == Some(key(t)));
        self.file_imports[f] = saved;
        ty_ok && term_ok
    }

    /// What the scope makes of a spelling, as a term and as a type: a binding found without a
    /// report binds it; else an ambiguity reported leaves it ambiguous; else it is free.
    fn spelling_state(&mut self, n: Name, at: Span) -> Spelling {
        let mut ambiguous = false;
        for types in [false, true] {
            let (found, reported) = if types {
                let (r, reported) = self.lookup_reporting(|w| w.lookup_type_at(n, at));
                (r.is_some_and(|r| !reported || self.defined_here_type(r)), reported)
            } else {
                let (r, reported) = self.lookup_reporting(|w| w.lookup_term_at(n, at));
                (r.is_some_and(|r| !reported || self.defined_here_term(r)), reported)
            };
            if found {
                return Spelling::Bound;
            }
            ambiguous |= reported;
        }
        if ambiguous { Spelling::Ambiguous } else { Spelling::Free }
    }

    /// What `f` resolves, and whether it reported anything on the way, the reports dropped.
    fn lookup_reporting<R>(&mut self, f: impl FnOnce(&mut Self) -> Option<R>) -> (Option<R>, bool) {
        let mark = self.diags.items.len();
        let r = f(self);
        let reported = self.diags.items.len() > mark;
        self.drop_reported_since(mark);
        (r, reported)
    }

    /// The term the scope binds to a spelling: what the lookup finds without a report, or a
    /// top-level definition of the site's own file it finds whatever it reported on the way: the
    /// file's own definitions come before every import, as scalac ranks them, while the lookup
    /// reports an ambiguity between wildcard imports before it meets them.
    pub(super) fn resolved_term(&mut self, n: Name, at: Span) -> Option<TermRef> {
        let (r, reported) = self.lookup_reporting(|w| w.lookup_term_at(n, at));
        r.filter(|&r| !reported || self.defined_here_term(r))
    }

    pub(super) fn resolved_type(&mut self, n: Name, at: Span) -> Option<TypeRef> {
        let (r, reported) = self.lookup_reporting(|w| w.lookup_type_at(n, at));
        r.filter(|&r| !reported || self.defined_here_type(r))
    }

    /// Whether a term is a top-level definition of the site's own file (a package's member).
    fn defined_here_term(&self, r: TermRef) -> bool {
        let (file, owner) = match r {
            TermRef::Class(c) => (self.syms.class(c).file, self.syms.class(c).owner),
            TermRef::Package(_) | TermRef::SelfAlias(_) => return false,
            other => match other.sym() {
                Some(s) => (self.syms.sym(s).file, self.syms.sym(s).owner),
                None => return false,
            },
        };
        matches!(owner, Owner::Package(_)) && self.of_site_file(file)
    }

    fn defined_here_type(&self, r: TypeRef) -> bool {
        let (file, owner) = match r {
            TypeRef::Class(c) => (self.syms.class(c).file, self.syms.class(c).owner),
            TypeRef::Alias(a) => (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].owner),
            _ => return false,
        };
        matches!(owner, Owner::Package(_)) && self.of_site_file(file)
    }

    /// Whether `f` is the site's file or one of its `package p:` blocks.
    fn of_site_file(&self, f: FileId) -> bool {
        self.source(f).path == self.source(self.env.file).path
    }

    /// Of the trees of a file (its own, one per `package p:` block), the one whose top-level
    /// definitions hold the offset.
    pub(super) fn file_at(&self, ids: &[FileId], offset: u32) -> FileId {
        for &f in ids {
            let ast = self.ast(f);
            if ast.top_level.iter().any(|&d| holds(ast.def_range(d), offset)) {
                return f;
            }
        }
        ids[0]
    }

    /// The candidates of the site, ranked and cut, each spelling's alternatives in the order of
    /// their signatures; with `calls`, a class after `new` with its constructor's clauses, and
    /// whether a function is expected where no name is written (`function_expected_here`).
    fn candidates(&mut self, file: FileId, ids: &[FileId], cx: &Context, calls: bool) -> (Vec<Candidate>, bool) {
        let binders = self.binders(file);
        // The receiver's node and type, read while the index is in place.
        let receiver = match &cx.site {
            Site::Member { recv, .. } => self.index_node_at(ids, *recv).and_then(|te| Some((te, self.prog.type_of(te)?))),
            _ => None,
        };
        // Where no name is written: the definition whose right-hand side the cursor is, and the
        // type of the receiver of the call it is an argument of, likewise.
        let term = calls && matches!(cx.site, Site::Scope | Site::Member { .. });
        // A member's from the symbol table, a local's from the index's declarations of the
        // locals, the latest of its name's position.
        let declared = cx.rhs_of.filter(|_| term).and_then(|at| match self.declaration_at(ids, at) {
            Some((_, super::index::Target::Sym(s))) => Some(s),
            _ => binders.locals.iter().filter(|&&(start, _, _)| start == at).map(|&(_, _, s)| s).max_by_key(|s| s.0),
        });
        let callee_recv = cx.arg.as_ref().filter(|_| term).and_then(|a| a.recv).and_then(|r| self.index_node_at(ids, r)).and_then(|te| self.prog.type_of(te));
        let mut function = false;
        let base = Env { file, frames: Vec::new(), imports: Vec::new() };
        let offset = cx.insert.start;
        let prefix = cx.prefix.to_lowercase();
        let mut out = self.in_query(base, |w| {
            w.enter_scope_at(file, offset, &binders);
            if term {
                function = w.function_expected_here(cx, declared, callee_recv, offset);
            }
            let mut out = Vec::new();
            match &cx.site {
                Site::Scope => {
                    w.scope_candidates(false, &prefix, offset, &mut out);
                    for k in crate::complete::KEYWORDS.iter().filter(|k| k.starts_with(&prefix)) {
                        out.push(Candidate { name: k.to_string(), target: Cand::Keyword, rank: Rank::Keyword, detail: String::new(), kind: kind::KEYWORD, import: None, clauses: Vec::new() });
                    }
                }
                Site::Type { path: None } => w.scope_candidates(true, &prefix, offset, &mut out),
                Site::New { path: None } => {
                    w.scope_candidates(true, &prefix, offset, &mut out);
                    w.keep_instantiable(&mut out, calls);
                }
                Site::Type { path: Some(p) } | Site::New { path: Some(p) } => {
                    match w.term_path(p, offset) {
                        Some(r) if matches!(r, TermRef::Package(_)) || w.path_object(r).is_some() => w.path_members(r, &prefix, true, &mut out),
                        // A stable value's types: those of its type's classes (`o.Inner`).
                        Some(r) => w.value_types(r, &prefix, &mut out),
                        None => {}
                    }
                    if matches!(cx.site, Site::New { .. }) {
                        w.keep_instantiable(&mut out, calls);
                    }
                }
                Site::Import { path } if path.is_empty() => w.scope_candidates(false, &prefix, offset, &mut out),
                Site::Import { path } => {
                    if let Some(r) = w.term_path(path, offset) {
                        w.path_members(r, &prefix, false, &mut out);
                        w.path_members(r, &prefix, true, &mut out);
                    }
                }
                Site::Member { path, .. } => match receiver {
                    Some((te, ty)) => {
                        let ty = w.zonk(ty);
                        let through_this = matches!(w.prog.expr(te), crate::tir::TExpr::This | crate::tir::TExpr::Super(_));
                        w.member_candidates(ty, through_this, &prefix, &mut out);
                    }
                    // `super` and a package have no node: the parents' members of the class the
                    // site is in, the package's by its path.
                    None if path.as_ref().is_some_and(|p| p.first().is_some_and(|s| s == "super")) => {
                        let qualifier = path.as_ref().and_then(|p| p.get(1)).cloned();
                        let class = w.env.frames.iter().rev().find_map(|f| match f {
                            Frame::Class(c) => Some(*c),
                            _ => None,
                        });
                        if let Some(c) = class {
                            let parents = w.syms.class(c).parents.clone();
                            for p in parents {
                                // `super[P]` selects in the parent `P` alone.
                                let named = qualifier.as_ref().map_or(true, |q| w.class_of(p).is_some_and(|k| w.name_ref(w.syms.class(k).name) == q));
                                if named {
                                    w.member_candidates(p, true, &prefix, &mut out);
                                }
                            }
                        }
                    }
                    None => {
                        if let Some(r @ TermRef::Package(_)) = path.as_ref().and_then(|p| w.term_path(p, offset)) {
                            w.path_members(r, &prefix, false, &mut out);
                        }
                    }
                },
                Site::Nothing => {}
            }
            out
        });
        dedup(&mut out);
        out.sort_by(|a, b| (a.rank, a.kind == kind::INTERFACE, a.name.to_lowercase(), &a.name, &a.detail).cmp(&(b.rank, b.kind == kind::INTERFACE, b.name.to_lowercase(), &b.name, &b.detail)));
        // The names out of the scope, after every in-scope group: none in a std document, which
        // takes no import.
        let types = match &cx.site {
            Site::Scope => Some(false),
            Site::Type { path: None } | Site::New { path: None } => Some(true),
            _ => None,
        };
        if let (Some(types), true, false) = (types, prefix.chars().count() >= 2, self.std_file(file)) {
            let catalogue = self.catalogue();
            let program = self.program_entries();
            let base = Env { file, frames: Vec::new(), imports: Vec::new() };
            let new = matches!(cx.site, Site::New { .. });
            let mut more = self.in_query(base, |w| {
                w.enter_scope_at(file, offset, &binders);
                let mut more = Vec::new();
                w.unimported_candidates(types, &prefix, &cx.prefix, offset, &[&program, &catalogue.external], &mut more);
                if new {
                    more.retain(|c| c.kind == kind::CLASS || c.kind == kind::INTERFACE);
                    // A class's constructor, the class resolved from the root by the path its
                    // import names, loaded where it is a jar's.
                    for c in more.iter_mut().filter(|c| c.kind == kind::CLASS) {
                        c.kind = kind::CONSTRUCTOR;
                        let Some((path, _)) = c.import.as_ref().filter(|_| calls) else { continue };
                        let segments: Vec<String> = path.split('.').map(decode).collect();
                        if let Some((_, _, _, _, Some(TypeRef::Class(k)), _)) = w.import_target(&segments) {
                            c.clauses = w.ctor_clauses(k);
                        }
                    }
                }
                more
            });
            if let Some(ix) = self.index.as_mut() {
                ix.catalogue = Some(catalogue);
            }
            out.append(&mut more);
        }
        self.keep_imports(binders);
        (out, function)
    }

    /// Whether a function is expected where no name is written: the declared type of the
    /// definition whose right-hand side the cursor is (`declared`), or the type of the parameter
    /// of the argument it is (`Context::arg`) of some alternative of the function, a member of
    /// the receiver's type (`recv`) or a name resolved in the scope (`param_expected`).
    fn function_expected_here(&mut self, cx: &Context, declared: Option<SymId>, recv: Option<TypeId>, offset: u32) -> bool {
        if let Some(s) = declared {
            let ret = self.sig_of(s).ret;
            if self.expects_function(ret, 4) {
                return true;
            }
        }
        let Some(arg) = &cx.arg else { return false };
        let at = Span::new(offset, offset);
        let n = self.interner.intern(&arg.name);
        let (found, subst) = match (arg.recv, recv) {
            (Some(_), Some(t)) => {
                let t = self.zonk(t);
                match self.find_member(t, n) {
                    Some((s, owner_ty)) => (Some(s), self.owner_subst(owner_ty)),
                    None => (None, Vec::new()),
                }
            }
            (None, _) => (self.resolved_term(n, at).and_then(|r| r.sym()), Vec::new()),
            _ => (None, Vec::new()),
        };
        let Some(s) = found else { return false };
        // The written type arguments a function type is, as a type parameter stands for them.
        let mut targs: Vec<bool> = Vec::with_capacity(arg.targs.len());
        for t in &arg.targs {
            let function = match t {
                TArg::Function => true,
                TArg::Path(path) => {
                    let ty = match self.written_type_path(path, offset) {
                        Some(TypeRef::Alias(a)) => Some(self.types.mk(Type::Alias(a, EMPTY_LIST))),
                        Some(TypeRef::Class(c)) if self.syms.class(c).tparams.is_empty() => Some(self.types.class(c, &[])),
                        _ => None,
                    };
                    ty.is_some_and(|ty| self.expects_function(ty, 4))
                }
                TArg::Other => false,
            };
            targs.push(function);
        }
        let alts = self.syms.alternatives(s).map_or_else(|| vec![s], |a| a.to_vec());
        alts.into_iter().any(|alt| self.param_expected(alt, &subst, arg, &targs))
    }

    /// Whether the parameter `arg` stands for of a call of `alt` may be a function: the argument
    /// lists written before its own and its own go, in order, to the method's clauses (a list
    /// to its next clause, a `using` clause it passes over given; a `using` list to the next
    /// clause, which has to be one), then, past them or for a value, to the layers of the
    /// function type its result or its value is (a context function's applied where a list
    /// that is no `using` one comes, its argument given). A clause's parameter is the one at the
    /// argument's position or of its name, a type parameter standing for the type argument
    /// written (`targs`, whether each is a function type).
    fn param_expected(&mut self, alt: SymId, subst: &Subst, arg: &crate::complete::ArgOf, targs: &[bool]) -> bool {
        let sig = self.sig_arc(alt);
        let mut clauses = sig.clauses.iter().peekable();
        let mut value: Option<TypeId> = None;
        let written: Vec<bool> = arg.before.iter().copied().chain([arg.using]).collect();
        for (k, &using) in written.iter().enumerate() {
            let last = k + 1 == written.len();
            if value.is_none() {
                if !using {
                    while clauses.peek().is_some_and(|c| c.is_using) {
                        clauses.next();
                    }
                }
                match clauses.next() {
                    Some(c) if c.is_using == using => {
                        if !last {
                            continue;
                        }
                        let p = match &arg.named {
                            Some(named) => c.params.iter().find(|p| self.name_ref(p.name) == named),
                            None => c.params.get(arg.index).or_else(|| c.params.last().filter(|p| p.repeated)),
                        };
                        let Some(p) = p else { return false };
                        let ty = self.types.subst(p.ty, subst);
                        let bare = self.deref(ty);
                        if let Type::Param(tp) = self.types.get(bare) {
                            if sig.tparams.iter().position(|&q| q == tp).is_some_and(|i| targs.get(i) == Some(&true)) {
                                return true;
                            }
                        }
                        return self.expects_function(ty, 4);
                    }
                    Some(_) => return false,
                    None => value = Some(self.types.subst(sig.ret, subst)),
                }
            }
            // A layer of the function type the result or the value is.
            let Some(mut ty) = value else { return false };
            if !using {
                while let Some((_, r)) = self.as_context_function(ty) {
                    ty = r;
                }
            }
            let layer = if using { self.as_context_function(ty) } else { self.as_function(ty) };
            let Some((params, r)) = layer else { return false };
            if last {
                return arg.named.is_none() && params.get(arg.index).is_some_and(|&p| self.expects_function(p, 4));
            }
            value = Some(r);
        }
        false
    }

    /// What a path of names resolves to as a type at the offset: a name in the scope, or a
    /// type member of the package or the object its prefix is.
    fn written_type_path(&mut self, path: &[String], offset: u32) -> Option<TypeRef> {
        let (last, prefix) = path.split_last()?;
        let last = self.interner.intern(last);
        if prefix.is_empty() {
            return self.resolved_type(last, Span::new(offset, offset));
        }
        match self.term_path(prefix, offset)? {
            TermRef::Package(p) => self.quietly(|w| w.pkg_type(p, last)),
            r => {
                let c = self.path_object(r)?;
                self.quietly(|w| w.module_type(c, last))
            }
        }
    }

    /// The catalogue of the std's and the class path's names, built on first use.
    fn catalogue(&mut self) -> Box<Catalogue> {
        if let Some(c) = self.index.as_mut().and_then(|ix| ix.catalogue.take()) {
            return c;
        }
        let mut external: Vec<Entry> = Vec::new();
        let std: Vec<(PkgId, Name, u8)> = self.std.index().by_name.iter().map(|(&(p, n), slots)| (p, n, slots.iter().fold(0, |k, &(_, x)| k | x))).collect();
        for (p, n, k) in std {
            let name = self.name_ref(n);
            if p == ROOT_PKG || !importable(name) {
                continue;
            }
            let kind = if k & crate::stdindex::TYPE != 0 { kind::CLASS } else { kind::FUNCTION };
            external.push(entry(name, &self.pkg_path(p), kind, 1));
        }
        for (owner, name, object) in self.class_path_names() {
            if importable(&name) {
                external.push(entry(&name, &owner, if object { kind::MODULE } else { kind::CLASS }, 2));
            }
        }
        external.sort_by(|a, b| (&a.path, a.origin).cmp(&(&b.path, b.origin)));
        external.dedup_by(|later, kept| later.path == kept.path);
        external.sort_by(|a, b| (&a.lower, a.origin, &a.path).cmp(&(&b.lower, b.origin, &b.path)));
        Box::new(Catalogue { external })
    }

    /// The program's names of packages: its top-level classes, objects, aliases and members,
    /// and the classes and objects nested in its objects, private ones and those of the root
    /// package left out, ordered as the catalogue is.
    fn program_entries(&mut self) -> Vec<Entry> {
        let mut out = Vec::new();
        for i in 1..self.syms.pkgs.len() {
            let p = PkgId(i as u32);
            let owner = self.pkg_path(p);
            let entries: Vec<(Name, Option<SymId>, Option<ClassId>, Option<AliasId>)> = self.syms.pkg(p).entries.iter().map(|(&n, e)| (n, e.term, e.class, e.alias)).collect();
            for (n, term, class, alias) in entries {
                let name = self.name_str(n);
                if !importable(&name) {
                    continue;
                }
                if let Some(c) = class {
                    self.program_class_entries(c, &owner, &mut out);
                }
                if let Some(a) = alias {
                    let (file, def) = (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].def);
                    let private = def.map_or(false, |d| self.ast(file).def(d).mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) != 0);
                    if self.program_source(file) && !private {
                        out.push(entry(&name, &owner, kind::CLASS, 0));
                    }
                }
                if let (Some(s), None) = (term, class) {
                    let (file, mods, k) = (self.syms.sym(s).file, self.syms.sym(s).mods, self.syms.sym(s).kind);
                    if let SymKind::Object(c) = k {
                        self.program_class_entries(c, &owner, &mut out);
                    } else if self.program_source(file) && mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED | crate::ast::mods::ANONYMOUS) == 0 {
                        out.push(entry(&name, &owner, if k == SymKind::Def { kind::FUNCTION } else { kind::FIELD }, 0));
                    }
                }
            }
        }
        out.sort_by(|a, b| (&a.lower, &a.path).cmp(&(&b.lower, &b.path)));
        out.dedup_by(|later, kept| later.path == kept.path);
        out
    }

    /// The entry of a program class and those nested in it where it is an object.
    fn program_class_entries(&mut self, c: ClassId, owner: &str, out: &mut Vec<Entry>) {
        let (file, mods, k, name) = {
            let info = self.syms.class(c);
            (info.file, info.mods, info.kind, info.name)
        };
        let name = self.name_str(name);
        if !self.program_source(file) || mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) != 0 || !importable(&name) {
            return;
        }
        let (_, kind, _) = self.describe_candidate(Cand::Class(c), None);
        out.push(entry(&name, owner, kind, 0));
        if k == ClassKind::Object && out.len() < 1 << 20 {
            let inner = format!("{}.{}", owner, encode(&name));
            let nested: Vec<ClassId> = self.syms.class(c).nested.values().copied().collect();
            for n in nested {
                self.program_class_entries(n, &inner, out);
            }
            let aliases: Vec<(Name, AliasId)> = self.syms.class(c).type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
            for (n, a) in aliases {
                let (file, def) = (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].def);
                let hidden = def.map_or(false, |d| self.ast(file).def(d).mods & (crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED) != 0);
                let alias = self.name_str(n);
                if self.program_source(file) && !hidden && importable(&alias) {
                    out.push(entry(&alias, &inner, kind::CLASS, 0));
                }
            }
        }
    }

    /// The names out of the scope that start with `prefix`: the program's and the catalogue's,
    /// each where no binding of its spelling is in scope, nor an import of it under another name
    /// or its exclusion from a wildcard; types alone with `types`.
    #[allow(clippy::too_many_arguments)]
    fn unimported_candidates(&mut self, types: bool, prefix: &str, written: &str, offset: u32, sources: &[&[Entry]], out: &mut Vec<Candidate>) {
        let at = Span::new(offset, offset);
        let mut states: FxMap<Box<str>, Spelling> = FxMap::default();
        let imported = self.imported_elsewise();
        let mut found: Vec<&Entry> = sources.iter().flat_map(|entries| with_prefix(entries, prefix)).collect();
        found.sort_by(|a, b| (!a.name.starts_with(written), a.origin, &a.lower, &a.owner).cmp(&(!b.name.starts_with(written), b.origin, &b.lower, &b.owner)));
        for e in found {
            if types && matches!(e.kind, kind::FUNCTION | kind::FIELD) {
                continue;
            }
            if imported.iter().any(|(owner, name)| **owner == *e.owner && name.as_deref().map_or(true, |n| n == &*e.name)) {
                continue;
            }
            let state = match states.get(&e.name) {
                Some(&state) => state,
                None => {
                    let n = self.interner.intern(&e.name);
                    let state = self.spelling_state(n, at);
                    states.insert(e.name.clone(), state);
                    state
                }
            };
            match state {
                Spelling::Bound => continue,
                Spelling::Free => {}
                // Ambiguous between wildcards, an explicit import settles it; between explicit
                // imports of the spelling, another conflicts: offered where its import, among the
                // imports of the site's tree, makes the spelling name its entry at the site.
                Spelling::Ambiguous => {
                    let segments: Vec<String> = e.path.split('.').map(decode).collect();
                    let settles = match self.import_target(&segments) {
                        Some((_, _, last, target, ty, term)) => self.imported_alone(last, target, ty, term),
                        None => false,
                    };
                    if !settles {
                        continue;
                    }
                }
            }
            out.push(Candidate { name: e.name.to_string(), target: Cand::Unimported, rank: Rank::Unimported, detail: shown_path(&e.path), kind: e.kind, import: Some((e.path.to_string(), shown_path(&e.owner))), clauses: Vec::new() });
        }
    }

    /// The packages whose imports in scope bring a name under another spelling or leave it out
    /// of a wildcard, each with the name, or none for every name of the package.
    fn imported_elsewise(&mut self) -> Vec<(String, Option<String>)> {
        let mut out = Vec::new();
        let n = self.import_count();
        for i in 0..n {
            let imp = self.import_at(i);
            match imp.target {
                ImportTarget::PkgMember(p, orig) if imp.name != Some(orig) => out.push((self.pkg_path(p), Some(self.name_str(orig)))),
                ImportTarget::ClassMember(c, orig) if imp.name != Some(orig) => out.push((self.class_path(c), Some(self.name_str(orig)))),
                ImportTarget::PkgAll(p) if !imp.hidden.is_empty() => {
                    let hidden: Vec<Name> = self.import_hidden.as_slice()[imp.hidden.range()].to_vec();
                    let owner = self.pkg_path(p);
                    for h in hidden {
                        out.push((owner.clone(), Some(self.name_str(h))));
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Runs `f` in the environment `env` in query mode: the index set aside, the diagnostics
    /// reported meanwhile dropped, the constraints made rolled back.
    pub(super) fn in_query<R>(&mut self, env: Env, f: impl FnOnce(&mut Self) -> R) -> R {
        let index = self.index.take();
        // A query is an attempt that goes whole: it marks no import used (`unused.rs`), reports
        // nothing, leaves no constraint.
        let vars = self.tvars.len();
        let demanded = self.attempts.demanded();
        let mark = self.attempt();
        let r = self.with_env(env, f);
        self.retract(mark);
        // The probes' variables go with their constraints, so that the next query makes the same
        // types over the same variables rather than new ones; not where work done on demand
        // inside it published something that may name them (a signature completed), as no
        // variable a retained result reaches is recycled (`TVars::truncate`).
        if self.attempts.demanded() == demanded {
            self.tvars.truncate(vars);
        }
        self.index = index;
        r
    }

    /// What `f` gathers, what it reports meanwhile dropped: a list of candidates, which a warning
    /// on the way does not void.
    pub(super) fn muted<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let mark = self.diags.items.len();
        let r = f(self);
        self.drop_reported_since(mark);
        r
    }

    /// What `f` resolves when it reports nothing: a lookup that reports an ambiguity, or a lazy
    /// load that fails, finds nothing.
    pub(super) fn quietly<R>(&mut self, f: impl FnOnce(&mut Self) -> Option<R>) -> Option<R> {
        let mark = self.diags.items.len();
        let r = f(self);
        let reported = self.diags.items.len() > mark;
        self.drop_reported_since(mark);
        r.filter(|_| !reported)
    }

    pub(super) fn binders(&mut self, file: FileId) -> Binders {
        let mut locals = Vec::new();
        let mut tparams = Vec::new();
        let mut fresh = (0, 0);
        let mut imports = FxMap::default();
        if let Some(ix) = self.index.as_mut() {
            fresh = ix.declarations_in(file, &mut locals, &mut tparams);
            imports = std::mem::take(&mut ix.block_imports);
        }
        let mut locals: Vec<(u32, Name, SymId)> = locals.into_iter().map(|(start, s)| (start, self.syms.sym(s).name, s)).collect();
        locals.sort_unstable_by_key(|&(start, _, s)| (start, s.0));
        let mut tparams: Vec<(u32, Name, TParamId)> = tparams.into_iter().map(|(start, p)| (start, self.syms.tparam(p).name, p)).collect();
        tparams.sort_unstable_by_key(|&(start, _, p)| (start, p.0));
        Binders { locals, tparams, fresh, imports: std::cell::RefCell::new(imports) }
    }

    /// Gives the block imports resolved meanwhile back to the index.
    pub(super) fn keep_imports(&mut self, b: Binders) {
        if let Some(ix) = self.index.as_mut() {
            ix.block_imports = b.imports.into_inner();
        }
    }

    // ---- the scope at the offset ----

    /// Makes the environment the environment at `offset` of `file`.
    pub(super) fn enter_scope_at(&mut self, file: FileId, offset: u32, b: &Binders) {
        let ast = self.ast(file);
        if let Some(&d) = ast.top_level.iter().find(|&&d| holds(ast.def_range(d), offset)) {
            self.scope_def(file, ast, d, offset, b);
        }
    }

    fn frame_mut(&mut self) -> Option<&mut Frame> {
        self.env.frames.last_mut().filter(|f| matches!(f, Frame::Locals { .. }))
    }

    fn bind_term(&mut self, name: Name, s: SymId) {
        let given = self.syms.is_given(s);
        if let Some(Frame::Locals { names, givens, .. }) = self.frame_mut() {
            names.push((name, s));
            if given {
                givens.push(s);
            }
        }
    }

    fn scope_def(&mut self, file: FileId, ast: &'a Ast, d: DefId, offset: u32, b: &Binders) {
        let def = ast.def(d);
        match &def.kind {
            DefKind::Class(cls) => {
                self.scope_class(file, d, offset, b);
                for stmt in &cls.body {
                    if holds(stmt_range(ast, stmt), offset) {
                        self.scope_stmt(file, ast, stmt, offset, b);
                        return;
                    }
                }
                for clause in &cls.clauses {
                    for p in &clause.params {
                        if let Some(e) = p.default.filter(|&e| holds(ast.expr_span(e), offset)) {
                            self.scope_expr(file, ast, e, offset, b);
                        }
                    }
                }
            }
            DefKind::Given(g) => {
                self.push_scope();
                self.bind_params(&g.tparams, &g.clauses, b);
                if let Some(e) = g.alias.filter(|&e| holds(ast.expr_span(e), offset)) {
                    self.scope_expr(file, ast, e, offset, b);
                    return;
                }
                if !g.body.is_empty() {
                    self.scope_class(file, d, offset, b);
                    for stmt in &g.body {
                        if holds(stmt_range(ast, stmt), offset) {
                            self.scope_stmt(file, ast, stmt, offset, b);
                            return;
                        }
                    }
                }
            }
            DefKind::Fun(f) => {
                self.push_scope();
                // A default argument sees the clauses before its own, the body all of them.
                for (k, clause) in f.clauses.iter().enumerate() {
                    for p in &clause.params {
                        if let Some(e) = p.default.filter(|&e| holds(ast.expr_span(e), offset)) {
                            self.bind_params(&f.tparams, &f.clauses[..k], b);
                            self.scope_expr(file, ast, e, offset, b);
                            return;
                        }
                    }
                }
                self.bind_params(&f.tparams, &f.clauses, b);
                if let Some(body) = f.body.filter(|&e| holds(ast.expr_span(e), offset)) {
                    self.scope_expr(file, ast, body, offset, b);
                }
            }
            DefKind::Val { rhs: Some(e), .. } if holds(ast.expr_span(*e), offset) => self.scope_expr(file, ast, *e, offset, b),
            _ => {}
        }
    }

    /// The environment in the body of the class `d` defines: its frames and imports as `env_at`
    /// gives them.
    fn scope_class(&mut self, file: FileId, d: DefId, offset: u32, b: &Binders) {
        let Some(&c) = self.def_classes.get(file.0 as usize, &d) else { return };
        if !self.fresh_class_of(c, b) {
            return;
        }
        let mut env = self.env_at(file, Owner::Class(c), offset);
        env.file = file;
        self.env = env;
    }

    /// Whether a class of the file is of its latest typing: a local one may be of a body typed
    /// again since, whose definition the tree no longer holds.
    fn fresh_class_of(&self, c: ClassId, b: &Binders) -> bool {
        self.syms.class(c).owner != Owner::Local || c.0 >= b.fresh.1
    }

    fn fresh_sym_of(&self, s: SymId, b: &Binders) -> bool {
        self.syms.sym(s).owner != Owner::Local || s.0 >= b.fresh.0
    }

    fn bind_params(&mut self, tparams: &[ast::TypeParam], clauses: &[ast::ParamClause], b: &Binders) {
        for tp in tparams {
            if let Some(p) = b.tparam(tp.name, tp.span) {
                if let Some(Frame::Locals { tparams, .. }) = self.frame_mut() {
                    tparams.push((tp.name, p));
                }
            }
        }
        for clause in clauses {
            for p in &clause.params {
                if let Some(s) = b.local(p.name, p.span) {
                    self.bind_term(p.name, s);
                    if clause.is_using || clause.is_implicit {
                        if let Some(Frame::Locals { givens, .. }) = self.frame_mut() {
                            if !givens.contains(&s) {
                                givens.push(s);
                            }
                        }
                    }
                }
            }
        }
    }

    fn scope_stmt(&mut self, file: FileId, ast: &'a Ast, stmt: &Stmt, offset: u32, b: &Binders) {
        match *stmt {
            Stmt::Def(d) => self.scope_def(file, ast, d, offset, b),
            Stmt::Expr(e) => self.scope_expr(file, ast, e, offset, b),
            Stmt::Import(_) => {}
        }
    }

    fn scope_expr(&mut self, file: FileId, ast: &'a Ast, e: ExprId, offset: u32, b: &Binders) {
        match ast.expr(e) {
            Expr::Block(stmts) => self.scope_block(file, ast, ast.stmt_list(stmts), offset, b),
            Expr::Lambda(params, body) => {
                self.push_scope();
                for p in &ast.lambda_params[params.range()] {
                    if let Some(s) = b.local(p.name, p.span) {
                        self.bind_term(p.name, s);
                        if p.implicit || p.contextual {
                            if let Some(Frame::Locals { givens, .. }) = self.frame_mut() {
                                givens.push(s);
                            }
                        }
                    }
                }
                if holds(ast.expr_span(body), offset) {
                    self.scope_expr(file, ast, body, offset, b);
                }
            }
            Expr::Match(scrutinee, cases) | Expr::InlineMatch(scrutinee, cases) => {
                if holds(ast.expr_span(scrutinee), offset) {
                    return self.scope_expr(file, ast, scrutinee, offset, b);
                }
                self.scope_cases(file, ast, ast.case_list(cases), offset, b);
            }
            Expr::Try(i) => {
                let t = ast.try_expr(i);
                if holds(ast.expr_span(t.body), offset) {
                    return self.scope_expr(file, ast, t.body, offset, b);
                }
                for x in [t.handler, t.finalizer].into_iter().flatten() {
                    if holds(ast.expr_span(x), offset) {
                        return self.scope_expr(file, ast, x, offset, b);
                    }
                }
                self.scope_cases(file, ast, ast.case_list(t.cases), offset, b);
            }
            Expr::For(enums, body, _) => {
                self.push_scope();
                for en in &ast.enumerators[enums.range()] {
                    let (pat, rhs) = match *en {
                        ast::Enumerator::Gen(p, x) | ast::Enumerator::CaseGen(p, x) | ast::Enumerator::Val(p, x) => (Some(p), x),
                        ast::Enumerator::Guard(x) => (None, x),
                    };
                    // A generator's identifier is a variable, capitalised or not, as the typer
                    // binds it (`for (X <- xs)`).
                    let generator = matches!(*en, ast::Enumerator::Gen(..) | ast::Enumerator::CaseGen(..));
                    if holds(ast.expr_span(rhs), offset) {
                        return self.scope_expr(file, ast, rhs, offset, b);
                    }
                    if ast.expr_span(rhs).start > offset {
                        break;
                    }
                    if let Some(p) = pat {
                        match ast.pat(p) {
                            Pat::StableId(path) if generator => {
                                if let Expr::Ident(n) = ast.expr(path) {
                                    if let Some(s) = b.local(n, ast.pat_spans[p.idx()]) {
                                        self.bind_term(n, s);
                                    }
                                }
                            }
                            _ => self.bind_pattern(ast, p, b),
                        }
                    }
                }
                if holds(ast.expr_span(body), offset) {
                    self.scope_expr(file, ast, body, offset, b);
                }
            }
            Expr::NewAnon(d) => self.scope_def(file, ast, d, offset, b),
            Expr::PolyLambda(names, lambda) => {
                self.push_scope();
                let span = ast.expr_span(e);
                for &n in &ast.name_lists[names.range()] {
                    if let Some(p) = b.tparam(n, span) {
                        if let Some(Frame::Locals { tparams, .. }) = self.frame_mut() {
                            tparams.push((n, p));
                        }
                    }
                }
                if holds(ast.expr_span(lambda), offset) {
                    self.scope_expr(file, ast, lambda, offset, b);
                }
            }
            _ => {
                if let Some(child) = children(ast, e).into_iter().find(|&c| holds(ast.expr_span(c), offset)) {
                    self.scope_expr(file, ast, child, offset, b);
                }
            }
        }
    }

    fn scope_cases(&mut self, file: FileId, ast: &'a Ast, cases: &'a [ast::CaseClause], offset: u32, b: &Binders) {
        for case in cases {
            let span = ast.pat_spans[case.pat.idx()].to(ast.expr_span(case.body));
            if !holds(span, offset) {
                continue;
            }
            self.push_scope();
            self.bind_pattern(ast, case.pat, b);
            for x in case.guard.into_iter().chain([case.body]) {
                if holds(ast.expr_span(x), offset) {
                    return self.scope_expr(file, ast, x, offset, b);
                }
            }
            return;
        }
    }

    fn bind_pattern(&mut self, ast: &Ast, p: PatId, b: &Binders) {
        let span = ast.pat_spans[p.idx()];
        match ast.pat(p) {
            Pat::Bind(n, inner) => {
                if let Some(s) = b.local(n, span) {
                    self.bind_term(n, s);
                }
                if let Some(i) = inner {
                    self.bind_pattern(ast, i, b);
                }
            }
            Pat::Typed(i, _) | Pat::NamedField(_, i) | Pat::Rest(i) => self.bind_pattern(ast, i, b),
            Pat::Ctor(_, l) | Pat::Tuple(l) | Pat::Alt(l) => {
                for &i in ast.pat_list(l) {
                    self.bind_pattern(ast, i, b);
                }
            }
            _ => {}
        }
    }

    /// A block: its definitions visible throughout, its values and imports from where they
    /// end, then the statement that holds the offset.
    fn scope_block(&mut self, file: FileId, ast: &'a Ast, stmts: &'a [Stmt], offset: u32, b: &Binders) {
        self.push_scope();
        let scope = self.env.imports.len();
        for stmt in stmts {
            let Stmt::Def(d) = *stmt else { continue };
            let def = ast.def(d);
            match &def.kind {
                DefKind::Fun(_) => {
                    if let Some(&s) = self.def_syms.get(file.0 as usize, &d) {
                        if self.fresh_sym_of(s, b) {
                            self.bind_term(def.name, s);
                        }
                    }
                }
                DefKind::Class(_) => {
                    let Some(&c) = self.def_classes.get(file.0 as usize, &d) else { continue };
                    if !self.fresh_class_of(c, b) {
                        continue;
                    }
                    let module = self.syms.class(c).local_module;
                    if let Some(Frame::Locals { classes, .. }) = self.frame_mut() {
                        classes.push((def.name, c));
                    }
                    if let Some(m) = module {
                        self.bind_term(def.name, m);
                    }
                }
                DefKind::TypeAlias { .. } => {
                    if let Some(&a) = self.def_aliases.get(file.0 as usize, &d) {
                        if let Some(Frame::Locals { aliases, .. }) = self.frame_mut() {
                            aliases.push((def.name, a));
                        }
                    }
                }
                _ => {}
            }
        }
        for stmt in stmts {
            let range = stmt_range(ast, stmt);
            if holds(range, offset) {
                // A lazy value is in scope in its own initializer (a recursive function value).
                if let Stmt::Def(d) = *stmt {
                    let def = ast.def(d);
                    if let (DefKind::Val { pat: None, .. }, true) = (&def.kind, def.mods & crate::ast::mods::LAZY != 0) {
                        if let Some(s) = b.local(def.name, ast.def_range(d)) {
                            self.bind_term(def.name, s);
                        }
                    }
                }
                return self.scope_stmt(file, ast, stmt, offset, b);
            }
            if range.start > offset {
                return;
            }
            match *stmt {
                Stmt::Def(d) => {
                    let def = ast.def(d);
                    match &def.kind {
                        DefKind::Val { pat: Some(p), .. } => self.bind_pattern(ast, *p, b),
                        DefKind::Val { .. } | DefKind::Given(_) => {
                            if let Some(s) = b.local(def.name, ast.def_range(d)).or_else(|| self.def_syms.get(file.0 as usize, &d).copied().filter(|&s| self.fresh_sym_of(s, b))) {
                                self.bind_term(def.name, s);
                            }
                        }
                        _ => {}
                    }
                }
                Stmt::Import(i) => {
                    let clauses = ast.import_stmt(i);
                    let key = (file, range.start);
                    let known = b.imports.borrow().get(&key).cloned();
                    let resolved = match known {
                        Some(r) => r,
                        None => {
                            let r: Vec<ResolvedImport> = clauses.iter().filter_map(|imp| self.resolve_import(imp, clauses)).collect();
                            b.imports.borrow_mut().insert(key, r.clone());
                            r
                        }
                    };
                    for r in resolved {
                        self.env.push_import(scope, r);
                    }
                }
                Stmt::Expr(_) => {}
            }
        }
    }

    // ---- what the scope binds ----

    /// The names the scope binds that start with `prefix`, each resolved by the lookup: terms,
    /// or with `types` types.
    fn scope_candidates(&mut self, types: bool, prefix: &str, offset: u32, out: &mut Vec<Candidate>) {
        let mut names: FxMap<Name, Rank> = FxMap::default();
        let note = |names: &mut FxMap<Name, Rank>, n: Name, rank: Rank| {
            let r = names.entry(n).or_insert(rank);
            *r = (*r).min(rank);
        };
        for i in (0..self.env.frames.len()).rev() {
            match self.env.frames[i].clone() {
                Frame::Locals { names: terms, tparams, classes, aliases, .. } => {
                    if types {
                        for (n, _) in classes {
                            note(&mut names, n, Rank::Local);
                        }
                        for (n, _) in aliases {
                            note(&mut names, n, Rank::Local);
                        }
                        for (n, _) in tparams {
                            note(&mut names, n, Rank::Local);
                        }
                    } else {
                        for (n, _) in terms.iter().chain(&classes.iter().map(|&(n, c)| (n, SymId(c.0))).collect::<Vec<_>>()) {
                            note(&mut names, *n, Rank::Local);
                        }
                    }
                }
                Frame::Class(c) => {
                    for n in self.class_scope_names(c, types) {
                        note(&mut names, n, Rank::Member);
                    }
                }
            }
        }
        let n_imports = self.import_count();
        for i in 0..n_imports {
            let imp = self.import_at(i);
            for n in self.import_names(imp, types, prefix) {
                note(&mut names, n, Rank::Import);
            }
        }
        if let Some(predef) = self.loaded.as_ref().and_then(|l| l.predef) {
            for n in self.module_names(predef, types) {
                note(&mut names, n, Rank::Root);
            }
        }
        let chain = self.pkg_chain();
        for (i, &p) in chain.iter().enumerate() {
            for n in self.package_names(p, types, prefix, false) {
                note(&mut names, n, if i == 0 { Rank::Package } else { Rank::Root });
            }
        }
        let root: Vec<Name> = self.syms.pkg(ROOT_PKG).entries.iter().filter(|(_, e)| e.pkg.is_some()).map(|(&n, _)| n).collect();
        for n in root {
            note(&mut names, n, Rank::Root);
        }
        let mut found: Vec<(Name, Rank)> = names.into_iter().filter(|&(n, _)| self.offered(n, prefix)).collect();
        found.sort_by_key(|&(n, _)| n.0);
        let at = Span::new(offset, offset);
        for (n, rank) in found {
            if types {
                let Some(r) = self.resolved_type(n, at) else { continue };
                if let Some(target) = self.type_target(r) {
                    self.push_candidate(n, target, rank, out);
                }
            } else {
                let Some(r) = self.resolved_term(n, at) else { continue };
                let target = match r {
                    TermRef::Class(c) => Cand::Class(c),
                    TermRef::Package(p) => Cand::Package(p),
                    TermRef::SelfAlias(c) => Cand::SelfAlias(c),
                    other => match other.sym() {
                        Some(s) => Cand::Sym(s),
                        None => continue,
                    },
                };
                self.push_candidate(n, target, rank, out);
            }
        }
    }

    /// Whether a name is one a completion offers for `prefix`: written by a program, not made
    /// by the compiler.
    fn offered(&self, n: Name, prefix: &str) -> bool {
        let s = self.name_ref(n);
        !s.is_empty() && !s.contains('$') && !s.starts_with('<') && s != "_" && s.to_lowercase().starts_with(prefix)
    }

    fn type_target(&self, r: TypeRef) -> Option<Cand> {
        match r {
            TypeRef::Class(c) => Some(Cand::Class(c)),
            TypeRef::Alias(a) => Some(Cand::Alias(a)),
            TypeRef::Param(p) => Some(Cand::TParam(p)),
            TypeRef::Member(c, n) => match self.syms.class(c).type_aliases.get(&n) {
                Some(&a) => Some(Cand::Alias(a)),
                None => self.inherited_inner_class(c, n).map(Cand::Class),
            },
            TypeRef::ValueMember(..) => None,
        }
    }

    /// The names a class frame binds: the members of the class's own type, its nested classes,
    /// its type parameters and aliases, its enum's cases, its exports, its `self` alias.
    fn class_scope_names(&mut self, c: ClassId, types: bool) -> Vec<Name> {
        self.complete_class(c);
        let mut out = Vec::new();
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        for &b in &bases {
            self.complete_class(b);
            let info = self.syms.class(b);
            if types {
                out.extend(info.nested.keys().copied());
                out.extend(info.type_aliases.keys().copied());
            } else {
                out.extend(info.members.keys().copied());
                out.extend(info.nested.keys().copied());
            }
        }
        let info = self.syms.class(c);
        if types {
            let tparams: Vec<Name> = info.tparams.iter().map(|&p| self.syms.tparam(p).name).collect();
            out.extend(tparams);
        } else if info.self_alias != crate::names::EMPTY {
            out.push(info.self_alias);
        }
        if info.kind == ClassKind::Enum {
            if let Some(co) = info.companion {
                out.extend(self.module_names(co, types));
            }
        }
        if let Some(exports) = self.exports_of(c) {
            out.extend(if types { exports.types.keys().copied().collect::<Vec<_>>() } else { exports.terms.keys().copied().collect() });
        }
        out
    }

    /// The names an object holds: its members (inherited ones included), nested classes and
    /// aliases, and its exports.
    fn module_names(&mut self, c: ClassId, types: bool) -> Vec<Name> {
        self.complete_class(c);
        let mut out = Vec::new();
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        for b in bases {
            self.complete_class(b);
            let info = self.syms.class(b);
            out.extend(info.nested.keys().copied());
            if types {
                out.extend(info.type_aliases.keys().copied());
            } else {
                out.extend(info.members.keys().copied());
            }
        }
        if let Some(exports) = self.exports_of(c) {
            out.extend(if types { exports.types.keys().copied().collect::<Vec<_>>() } else { exports.terms.keys().copied().collect() });
        }
        out
    }

    /// The names an import brings: its own for a named one, its target's but those it hides for
    /// a wildcard.
    fn import_names(&mut self, imp: ResolvedImport, types: bool, prefix: &str) -> Vec<Name> {
        if let Some(n) = imp.name {
            return vec![n];
        }
        let names = match imp.target {
            ImportTarget::PkgAll(p) | ImportTarget::PkgGivens(p) => self.package_names(p, types, prefix, true),
            ImportTarget::ClassAll(c) | ImportTarget::ClassGivens(c) => self.module_names(c, types),
            ImportTarget::ValueAll(v) | ImportTarget::ValueGivens(v) => match self.import_value_class(v) {
                Some(c) => self.module_names(c, types),
                None => Vec::new(),
            },
            _ => Vec::new(),
        };
        names.into_iter().filter(|&n| !self.import_hides(imp, n)).collect()
    }

    /// The names of a package: its entries, its package object's, its exports', and with a
    /// prefix, or with `catalogue` (a wildcard import of the package), what the std's index and
    /// the class path hold there that no lookup entered yet.
    fn package_names(&mut self, p: PkgId, types: bool, prefix: &str, catalogue: bool) -> Vec<Name> {
        let mut out: Vec<Name> = self.syms.pkg(p).entries.iter().filter(|(_, e)| if types { e.class.is_some() || e.alias.is_some() || e.pkg.is_some() } else { e.term.is_some() || e.class.is_some() || e.pkg.is_some() || !e.extensions.is_empty() }).map(|(&n, _)| n).collect();
        if let Some(obj) = self.syms.pkg(p).package_object {
            out.extend(self.module_names(obj, types));
        }
        if self.syms.pkg(p).has_exports() {
            if let Some(exports) = self.pkg_exports_of(p) {
                out.extend(if types { exports.types.keys().copied().collect::<Vec<_>>() } else { exports.terms.keys().copied().collect() });
            }
        }
        if prefix.is_empty() && !catalogue {
            return out;
        }
        let wanted = if types { crate::stdindex::TYPE } else { crate::stdindex::TERM | crate::stdindex::TYPE };
        out.extend(self.std.index().by_name.iter().filter(|(&(q, n), slots)| q == p && slots.iter().any(|&(_, k)| k & wanted != 0) && self.name_ref(n).to_lowercase().starts_with(prefix)).map(|(&(_, n), _)| n));
        for s in self.catalogue_names(p) {
            if s.to_lowercase().starts_with(prefix) {
                out.push(self.interner.intern(&s));
            }
        }
        out
    }

    /// Pushes the candidate of `target` spelled `n`, one per alternative of an overloaded name.
    fn push_candidate(&mut self, n: Name, target: Cand, rank: Rank, out: &mut Vec<Candidate>) {
        let name = self.name_str(n);
        if let Cand::Sym(s) = target {
            if self.is_setter(s) {
                return;
            }
            if let Some(mut alts) = self.syms.alternatives(s).map(|a| a.to_vec()) {
                // The alternatives the site reaches: a private one of another class is none.
                alts.retain(|&alt| self.is_accessible(alt) && !self.is_setter(alt));
                for alt in alts {
                    let (detail, kind, clauses) = self.describe_candidate(Cand::Sym(alt), None);
                    out.push(Candidate { name: name.clone(), target: Cand::Sym(alt), rank, detail, kind, import: None, clauses });
                }
                return;
            }
        }
        let (detail, kind, clauses) = self.describe_candidate(target, None);
        out.push(Candidate { name, target, rank, detail, kind, import: None, clauses });
    }

    /// A var's setter, which no completion offers (dotty's `isValidCompletionSymbol`:
    /// `!sym.isAllOf(Mutable | Accessor)`).
    fn is_setter(&self, s: SymId) -> bool {
        self.syms.sym(s).mods & crate::ast::mods::SETTER != 0
    }

    /// What an item shows of its target, its signature seen from `owner` where it is a member
    /// of that type, its kind, and a method's clauses read from that signature.
    fn describe_candidate(&mut self, target: Cand, owner: Option<TypeId>) -> (String, u32, Vec<CallClause>) {
        match target {
            Cand::Sym(s) => {
                let (sym_kind, sym_owner, mods) = {
                    let info = self.syms.sym(s);
                    (info.kind, info.owner, info.mods)
                };
                if let SymKind::Object(c) = sym_kind {
                    return (format!("object {}", self.class_path(c)), kind::MODULE, Vec::new());
                }
                let subst = owner.map(|o| self.owner_subst(o)).unwrap_or_default();
                let sig = self.sig_arc(s);
                let text = self.sig_text_in(&sig, &subst);
                let clauses = if sym_kind == SymKind::Def {
                    let receiver = self.syms.sym(s).ext_clauses as usize;
                    self.call_clauses(&sig, &subst, receiver)
                } else {
                    Vec::new()
                };
                let kind = match sym_kind {
                    SymKind::Def => kind::METHOD,
                    SymKind::EnumValue(_) => kind::ENUM_MEMBER,
                    _ => {
                        let ret = self.types.subst(sig.ret, &subst);
                        if self.as_function(ret).is_some() {
                            kind::FUNCTION
                        } else if matches!(sym_owner, Owner::Class(_)) {
                            kind::FIELD
                        } else {
                            kind::VARIABLE
                        }
                    }
                };
                let _ = mods;
                let text = if matches!(sym_kind, SymKind::Def) { text } else { text.trim_start_matches(": ").to_string() };
                (text, kind, clauses)
            }
            Cand::Class(c) => {
                let (k, mods) = {
                    let info = self.syms.class(c);
                    (info.kind, info.mods)
                };
                let (word, kind) = match k {
                    ClassKind::Trait => ("trait", kind::INTERFACE),
                    ClassKind::Object => ("object", kind::MODULE),
                    ClassKind::Enum => ("enum", kind::ENUM),
                    ClassKind::EnumCase => ("case", kind::ENUM_MEMBER),
                    _ if mods & crate::ast::mods::CASE != 0 => ("case class", kind::CLASS),
                    _ => ("class", kind::CLASS),
                };
                (format!("{} {}", word, self.class_path(c)), kind, Vec::new())
            }
            Cand::Alias(a) => {
                let (name, owner) = {
                    let info = &self.syms.aliases[a.idx()];
                    (info.name, info.owner)
                };
                let path = match owner {
                    Owner::Class(o) => format!("{}.{}", self.class_path(o), self.name_str(name)),
                    Owner::Package(p) if p != ROOT_PKG => format!("{}.{}", self.pkg_description(p), self.name_str(name)),
                    _ => self.name_str(name),
                };
                (format!("type {}", path), kind::CLASS, Vec::new())
            }
            Cand::Package(p) => (format!("package {}", self.pkg_description(p)), kind::MODULE, Vec::new()),
            Cand::TParam(p) => (format!("type {}", self.name_str(self.syms.tparam(p).name)), kind::TYPE_PARAMETER, Vec::new()),
            Cand::SelfAlias(c) => (self.class_path(c), kind::VARIABLE, Vec::new()),
            Cand::Builtin(sig) => (sig.trim_start_matches(": ").to_string(), kind::METHOD, Vec::new()),
            Cand::Keyword => (String::new(), kind::KEYWORD, Vec::new()),
            Cand::Unimported => (String::new(), kind::CLASS, Vec::new()),
        }
    }

    /// The clauses of a call of `sig` as an item's detail prints them, its owner's type
    /// parameters replaced as `subst` says, the first `receiver` of them an extension's
    /// receiver's: what the item's snippet writes, read once per item.
    fn call_clauses(&mut self, sig: &MethodSig, subst: &Subst, receiver: usize) -> Vec<CallClause> {
        let mut out = Vec::with_capacity(sig.clauses.len());
        for (i, c) in sig.clauses.iter().enumerate() {
            let mut params = Vec::with_capacity(c.params.len());
            for p in &c.params {
                let ty = self.types.subst(p.ty, subst);
                params.push(CallParam { name: p.name, repeated: p.repeated, function: self.as_function(ty).is_some(), default: p.has_default });
            }
            out.push(CallClause { using: c.is_using || c.is_implicit, receiver: i < receiver, params });
        }
        out
    }

    /// The clauses of the primary constructor of `c` a `new` writes: none for a trait, for a
    /// constructor out of reach of the site (a private one), or for a class of several
    /// constructors (secondary ones, a Java class's), of which the one item cannot choose.
    fn ctor_clauses(&mut self, c: ClassId) -> Vec<CallClause> {
        self.complete_class(c);
        if self.syms.class(c).kind != ClassKind::Class || !self.syms.class(c).ctors.is_empty() || !self.ctor_accessible(c) {
            return Vec::new();
        }
        let sig = MethodSig { tparams: Vec::new(), clauses: self.syms.class(c).ctor.clone(), ret: ERROR };
        self.call_clauses(&sig, &Vec::new(), 0)
    }

    /// After `new`: the classes and traits, a class's term (its object) left out, each class
    /// with its constructor's clauses with `calls`.
    fn keep_instantiable(&mut self, out: &mut Vec<Candidate>, calls: bool) {
        out.retain(|c| match c.target {
            Cand::Class(k) => matches!(self.syms.class(k).kind, ClassKind::Class | ClassKind::Trait),
            Cand::Alias(_) => true,
            _ => false,
        });
        for c in out.iter_mut() {
            if c.kind == kind::CLASS {
                c.kind = kind::CONSTRUCTOR;
            }
            if let (Cand::Class(k), true) = (c.target, calls) {
                c.clauses = self.ctor_clauses(k);
            }
        }
    }

    // ---- paths ----

    /// What a path of names resolves to as a term at the offset: a package, an object, a value.
    pub(super) fn term_path(&mut self, path: &[String], offset: u32) -> Option<TermRef> {
        let (head, rest) = path.split_first()?;
        let head = self.interner.lookup(head)?;
        let at = Span::new(offset, offset);
        let mut r = self.quietly(|w| w.lookup_term_at(head, at))?;
        for seg in rest {
            let n = self.interner.lookup(seg)?;
            r = match r {
                TermRef::Package(p) => self.quietly(|w| w.pkg_term(p, n))?,
                other => match self.path_object(other) {
                    Some(c) => self.quietly(|w| w.module_term(c, n))?,
                    // A stable value's member, through its type (`o.inner`).
                    None => {
                        let ty = self.sig_of(other.sym()?).ret;
                        let ty = self.zonk(ty);
                        let (s, _) = self.quietly(|w| w.find_member(ty, n))?;
                        if !matches!(self.syms.sym(s).kind, SymKind::Val | SymKind::Param | SymKind::Object(_)) || !self.is_accessible(s) {
                            return None;
                        }
                        TermRef::Global(s)
                    }
                },
            };
        }
        Some(r)
    }

    /// Whether a term an import's path names is reached from outside its owner: no private or
    /// protected member, object or class.
    fn public_term(&self, r: TermRef) -> bool {
        let hidden = crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED;
        match r {
            TermRef::Package(_) => true,
            TermRef::Class(c) => self.syms.class(c).mods & hidden == 0,
            other => other.sym().is_some_and(|s| {
                let object_hidden = match self.syms.sym(s).kind {
                    SymKind::Object(c) => self.syms.class(c).mods & hidden != 0,
                    _ => false,
                };
                self.syms.sym(s).mods & hidden == 0 && !object_hidden
            }),
        }
    }

    /// The object a term names, where it names one.
    pub(super) fn path_object(&self, r: TermRef) -> Option<ClassId> {
        match r {
            TermRef::Class(c) if self.syms.class(c).kind == ClassKind::Object => Some(c),
            TermRef::Class(c) => self.syms.class(c).companion.filter(|&k| self.syms.class(k).kind == ClassKind::Object),
            other => match self.syms.sym(other.sym()?).kind {
                SymKind::Object(c) => Some(c),
                _ => None,
            },
        }
    }

    /// The members of a package or an object a path names: its terms, or with `types` its types.
    fn path_members(&mut self, r: TermRef, prefix: &str, types: bool, out: &mut Vec<Candidate>) {
        match r {
            TermRef::Package(p) => {
                let mut names = self.package_names(p, types, prefix, true);
                if !types {
                    names.extend(self.syms.pkg(p).entries.iter().filter(|(_, e)| e.pkg.is_some()).map(|(&n, _)| n));
                }
                names.sort_by_key(|n| n.0);
                names.dedup();
                names.retain(|&n| self.offered(n, prefix));
                for n in names {
                    let target = if types {
                        self.quietly(|w| w.pkg_type(p, n)).and_then(|r| self.type_target(r)).filter(|&t| self.type_reachable(t))
                    } else {
                        self.quietly(|w| w.pkg_term(p, n)).and_then(|r| match r {
                            TermRef::Class(c) => Some(Cand::Class(c)),
                            TermRef::Package(q) => Some(Cand::Package(q)),
                            other => other.sym().map(Cand::Sym),
                        })
                    };
                    if let Some(t) = target {
                        self.push_candidate(n, t, Rank::Member, out);
                    }
                }
            }
            other => {
                let Some(c) = self.path_object(other) else { return };
                let mut names = self.module_names(c, types);
                names.sort_by_key(|n| n.0);
                names.dedup();
                names.retain(|&n| self.offered(n, prefix));
                for n in names {
                    let target = if types {
                        self.quietly(|w| w.module_type(c, n)).and_then(|r| self.type_target(r)).filter(|&t| self.type_reachable(t))
                    } else {
                        self.quietly(|w| w.module_term(c, n)).and_then(|r| match r {
                            TermRef::Class(k) => Some(Cand::Class(k)),
                            other => other.sym().filter(|&s| self.is_accessible(s)).map(Cand::Sym),
                        })
                    };
                    if let Some(t) = target {
                        self.push_candidate(n, t, Rank::Member, out);
                    }
                }
            }
        }
    }

    /// Whether a class or an alias selected by a path is reached from the site: a private one
    /// from its owner and the owner's companion, a protected one from those and its subclasses.
    fn type_reachable(&self, t: Cand) -> bool {
        let (mods, owner) = match t {
            Cand::Class(c) => return self.is_class_accessible(c) && (self.syms.class(c).mods & crate::ast::mods::PROTECTED == 0 || self.inside_or_derives(self.syms.class(c).owner)),
            Cand::Alias(a) => {
                let (file, def, owner) = (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].def, self.syms.aliases[a.idx()].owner);
                (def.map_or(0, |d| self.ast(file).def(d).mods), owner)
            }
            _ => return true,
        };
        if mods & crate::ast::mods::PRIVATE != 0 {
            // `private[p]`: reached within `p`, as the typer's qualified access decides.
            if let Cand::Alias(a) = t {
                let (file, def) = (self.syms.aliases[a.idx()].file, self.syms.aliases[a.idx()].def);
                if let Some(at) = def.map(|d| self.ast(file).def(d).span.start) {
                    if self.has_access_scope(file, at) {
                        return self.in_access_scope(file, at, owner);
                    }
                }
            }
            let Owner::Class(o) = owner else { return true };
            let companion = self.syms.class(o).companion;
            return self.env.frames.iter().any(|f| matches!(f, Frame::Class(k) if *k == o || Some(*k) == companion));
        }
        mods & crate::ast::mods::PROTECTED == 0 || self.inside_or_derives(owner)
    }

    /// Whether the site is inside the class `owner`, its companion or a subclass of it.
    fn inside_or_derives(&self, owner: Owner) -> bool {
        let Owner::Class(o) = owner else { return true };
        let companion = self.syms.class(o).companion;
        self.env.frames.iter().any(|f| match f {
            Frame::Class(k) => *k == o || Some(*k) == companion || self.syms.class(*k).base_types.iter().any(|&(b, _)| b == o),
            _ => false,
        })
    }

    /// The types a stable value's path selects: the classes and aliases its type's classes and
    /// their bases declare.
    fn value_types(&mut self, r: TermRef, prefix: &str, out: &mut Vec<Candidate>) {
        let Some(s) = r.sym() else { return };
        let ty = self.sig_of(s).ret;
        let ty = self.zonk(ty);
        let mut classes = Vec::new();
        self.receiver_classes(ty, &mut classes, 0);
        for c in classes {
            self.complete_class(c);
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            for b in bases {
                self.complete_class(b);
                let nested: Vec<(Name, ClassId)> = self.syms.class(b).nested.iter().map(|(&n, &k)| (n, k)).collect();
                for (n, k) in nested {
                    if self.offered(n, prefix) && self.is_class_accessible(k) {
                        self.push_candidate(n, Cand::Class(k), Rank::Member, out);
                    }
                }
                let aliases: Vec<(Name, AliasId)> = self.syms.class(b).type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
                for (n, a) in aliases {
                    if self.offered(n, prefix) && self.type_reachable(Cand::Alias(a)) {
                        self.push_candidate(n, Cand::Alias(a), Rank::Member, out);
                    }
                }
            }
        }
    }

    // ---- the members of a receiver ----

    /// The classes whose members a value of type `t` has.
    fn receiver_classes(&mut self, t: TypeId, out: &mut Vec<ClassId>, depth: u32) {
        if depth > 8 {
            return;
        }
        let t = self.deref(t);
        match self.types.get(t) {
            Type::Class(c, _) => {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
            Type::Param(p) => {
                let upper = self.syms.tparam(p).upper;
                if upper != ANY {
                    self.receiver_classes(upper, out, depth + 1);
                }
            }
            Type::Inter(a, b) => {
                self.receiver_classes(a, out, depth + 1);
                self.receiver_classes(b, out, depth + 1);
            }
            Type::Lit(_) => {
                let w = self.widen_lit(t);
                self.receiver_classes(w, out, depth + 1);
            }
            Type::Union(..) => {
                if let Some(j) = self.union_join(t) {
                    self.receiver_classes(j, out, depth + 1);
                }
            }
            _ => {
                let d = self.dealias(t);
                if d != t {
                    return self.receiver_classes(d, out, depth + 1);
                }
                if let Some(c) = self.class_of(t) {
                    if !out.contains(&c) {
                        out.push(c);
                    }
                }
            }
        }
    }

    /// The names a receiver of type `t` may select: the members of its classes and their bases.
    fn member_names(&mut self, t: TypeId) -> Vec<Name> {
        let mut classes = Vec::new();
        self.receiver_classes(t, &mut classes, 0);
        let mut out = Vec::new();
        for c in classes {
            self.complete_class(c);
            if self.loaded.is_some() {
                // A Java class's members are read when a lookup misses one.
                for _ in 0..4 {
                    if !self.java_member_miss(c) {
                        break;
                    }
                }
            }
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            for b in bases {
                self.complete_class(b);
                out.extend(self.syms.class(b).members.keys().copied());
            }
            if self.syms.class(c).kind == ClassKind::Object {
                out.extend(self.syms.class(c).nested.keys().copied());
            }
            if let Some(exports) = self.exports_of(c) {
                out.extend(exports.terms.keys().copied());
            }
        }
        out.sort_by_key(|n| n.0);
        out.dedup();
        out
    }

    fn member_candidates(&mut self, t: TypeId, through_this: bool, prefix: &str, out: &mut Vec<Candidate>) {
        if t == ERROR || self.types.contains_error(t) {
            return;
        }
        let start = out.len();
        for n in self.member_names(t) {
            if !self.offered(n, prefix) || self.name_ref(n) == "<init>" {
                continue;
            }
            if let Some((s, owner_ty)) = self.find_member(t, n) {
                let alts = match self.syms.alternatives(s) {
                    Some(alts) => alts.to_vec(),
                    None => vec![s],
                };
                for alt in alts {
                    if !self.member_reachable(alt, through_this, t) || self.is_setter(alt) {
                        continue;
                    }
                    let (detail, kind, clauses) = self.describe_candidate(Cand::Sym(alt), Some(owner_ty));
                    out.push(Candidate { name: self.name_str(n), target: Cand::Sym(alt), rank: Rank::Member, detail, kind, import: None, clauses });
                }
                continue;
            }
            // An object's nested class, selected as a term; else a member its classes export,
            // which the selection finds in their export tables when no member has the name.
            let mut classes = Vec::new();
            self.receiver_classes(t, &mut classes, 0);
            if let Some(k) = classes.iter().find_map(|&c| self.syms.class(c).nested.get(&n).copied()) {
                if self.is_class_accessible(k) {
                    self.push_candidate(n, Cand::Class(k), Rank::Member, out);
                }
                continue;
            }
            let exported = classes.iter().find_map(|&c| self.exports_of(c).and_then(|e| e.terms.get(&n).copied()));
            let target = match exported {
                Some(TermRef::Class(k)) => Some(Cand::Class(k)),
                Some(r) => r.sym().map(Cand::Sym),
                None => None,
            };
            if let Some(target) = target {
                self.push_candidate(n, target, Rank::Member, out);
            }
        }
        let have: Vec<String> = out[start..].iter().map(|c| c.name.clone()).collect();
        let d = self.deref(t);
        // An enum's members the selection makes without a symbol: the companion's `values`,
        // `valueOf` and `fromOrdinal`, a value's `ordinal`.
        let enum_members: Vec<(&'static str, String)> = match self.types.get(d) {
            Type::Class(c, _) => {
                let (kind, companion, owner) = (self.syms.class(c).kind, self.syms.class(c).companion, self.syms.class(c).owner);
                let of_enum = |w: &Self, k: ClassId| w.syms.class(k).kind == ClassKind::Enum;
                // Only an enum whose cases are all values has `values`, `valueOf` and
                // `fromOrdinal`, as the selection decides.
                let singletons = |w: &Self, e: ClassId| w.syms.class(e).children.iter().all(|&k| w.syms.class(k).singleton.is_some());
                if kind == ClassKind::Object && companion.is_some_and(|e| of_enum(self, e) && singletons(self, e)) {
                    let e = self.class_path(companion.unwrap());
                    vec![("values", format!("Array[{}]", e)), ("valueOf", format!("(name: String): {}", e)), ("fromOrdinal", format!("(ordinal: Int): {}", e))]
                } else if kind == ClassKind::Enum || kind == ClassKind::EnumCase || matches!(owner, Owner::Class(o) if self.syms.class(o).companion.is_some_and(|e| of_enum(self, e))) {
                    vec![("ordinal", "Int".to_string())]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };
        for (name, sig) in enum_members {
            if !have.iter().any(|h| h == name) && name.to_lowercase().starts_with(prefix) {
                out.push(Candidate { name: name.to_string(), target: Cand::Builtin(""), rank: Rank::Member, detail: sig, kind: kind::METHOD, import: None, clauses: Vec::new() });
            }
        }
        let numeric = self.is_numeric(d).is_some();
        let reference = !numeric && !matches!(self.types.get(d), Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Builtin);
        let builtins = UNIVERSAL.iter().chain(if numeric { NUMERIC } else { &[] }).chain(if reference { REFERENCE } else { &[] });
        for &(name, sig) in builtins {
            if !have.iter().any(|h| h == name) && name.to_lowercase().starts_with(prefix) {
                out.push(Candidate { name: name.to_string(), target: Cand::Builtin(sig), rank: Rank::Member, detail: sig.trim_start_matches(": ").to_string(), kind: kind::METHOD, import: None, clauses: Vec::new() });
            }
        }
        let mut have: Vec<String> = out[start..].iter().map(|c| c.name.clone()).collect();
        self.extension_candidates(t, prefix, &mut have, out);
        self.conversion_candidates(t, prefix, &have, out);
    }

    /// Whether the selection's access rules admit the member `s` from the site: a private one
    /// within its class and companion, a qualified one within its scope, a protected one through
    /// `this` or from a subclass on one of its instances.
    pub(super) fn member_reachable(&mut self, s: SymId, through_this: bool, recv_ty: TypeId) -> bool {
        if !self.is_accessible(s) {
            return false;
        }
        if self.syms.sym(s).mods & crate::ast::mods::PROTECTED == 0 {
            return true;
        }
        self.protected_denied(s, through_this, recv_ty).is_none()
    }

    /// The extension methods the selection would try, by the spellings the scope, its givens
    /// and the receiver's implicit scope hold that start with `prefix`, each kept where its
    /// receiver parameter takes the receiver.
    fn extension_candidates(&mut self, t: TypeId, prefix: &str, have: &mut Vec<String>, out: &mut Vec<Candidate>) {
        let mut names: Vec<Name> = Vec::new();
        let of_class = |w: &mut Self, c: ClassId, names: &mut Vec<Name>| {
            w.complete_class(c);
            let bases: Vec<ClassId> = w.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            for b in bases {
                w.complete_class(b);
                let exts: Vec<SymId> = w.syms.class_raw(b).extensions.clone();
                names.extend(exts.into_iter().map(|s| w.syms.sym(s).name));
            }
            if let Some(exports) = w.exports_of(c) {
                names.extend(exports.extensions.keys().copied());
            }
        };
        for i in (0..self.env.frames.len()).rev() {
            match self.env.frames[i].clone() {
                Frame::Locals { names: terms, givens, .. } => {
                    names.extend(terms.iter().filter(|&&(_, s)| self.syms.sym(s).is_extension).map(|&(n, _)| n));
                    for g in givens {
                        let ret = self.sig_of(g).ret;
                        if let Some(c) = self.class_of(ret) {
                            of_class(self, c, &mut names);
                        }
                    }
                }
                Frame::Class(c) => {
                    of_class(self, c, &mut names);
                    let givens: Vec<SymId> = self.syms.class_raw(c).givens.clone();
                    for g in givens {
                        let ret = self.sig_of(g).ret;
                        if let Some(k) = self.class_of(ret) {
                            of_class(self, k, &mut names);
                        }
                    }
                }
            }
        }
        // The extensions of a given's type, which an import of the given brings.
        let of_givens = |w: &mut Self, givens: Vec<SymId>, names: &mut Vec<Name>| {
            for g in givens {
                if !w.syms.is_given(g) {
                    continue;
                }
                let ret = w.sig_of(g).ret;
                if let Some(k) = w.class_of(ret) {
                    of_class(w, k, names);
                }
            }
        };
        let n_imports = self.import_count();
        for i in 0..n_imports {
            let imp = self.import_at(i);
            match imp.target {
                ImportTarget::ClassAll(c) | ImportTarget::ClassGivens(c) => {
                    of_class(self, c, &mut names);
                    let givens = self.syms.class_raw(c).givens.clone();
                    of_givens(self, givens, &mut names);
                }
                ImportTarget::ClassMember(c, orig) => {
                    names.push(imp.name.unwrap_or(orig));
                    let given = self.quietly(|w| w.module_term(c, orig)).and_then(|r| r.sym());
                    of_givens(self, given.into_iter().collect(), &mut names);
                }
                ImportTarget::PkgMember(p, orig) => {
                    names.push(imp.name.unwrap_or(orig));
                    let given = self.quietly(|w| w.pkg_term(p, orig)).and_then(|r| r.sym());
                    of_givens(self, given.into_iter().collect(), &mut names);
                }
                ImportTarget::PkgAll(p) | ImportTarget::PkgGivens(p) => {
                    names.extend(self.package_extension_names(p, prefix));
                    let givens = self.syms.pkg(p).givens.clone();
                    of_givens(self, givens, &mut names);
                }
                ImportTarget::ValueAll(v) | ImportTarget::ValueGivens(v) => {
                    if let Some(c) = self.import_value_class(v) {
                        of_class(self, c, &mut names);
                    }
                }
                _ => {}
            }
        }
        let chain = self.pkg_chain();
        for &p in chain.iter() {
            names.extend(self.package_extension_names(p, prefix));
        }
        if let Some(predef) = self.loaded.as_ref().and_then(|l| l.predef) {
            of_class(self, predef, &mut names);
        }
        for m in self.implicit_scope_modules(t) {
            of_class(self, m, &mut names);
            let givens: Vec<SymId> = self.syms.class_raw(m).givens.clone();
            for g in givens {
                let ret = self.sig_of(g).ret;
                if let Some(k) = self.class_of(ret) {
                    of_class(self, k, &mut names);
                }
            }
        }
        names.sort_by_key(|n| n.0);
        names.dedup();
        for n in names {
            if !self.offered(n, prefix) || have.iter().any(|h| h == self.name_ref(n)) {
                continue;
            }
            // Each candidate with where it is found: lexically, in an object of the receiver's
            // implicit scope, or as a member of a given's type, which an inherited one's owner's
            // parameters are read through, as the selection reads them.
            let mut found: Vec<(SymId, Found)> = Vec::new();
            for e in self.muted(|w| w.lexical_extensions(n)) {
                found.push((e, Found::Lexical));
            }
            // An inherited one's object is where the search found it, each occurrence its own
            // (`ext_modules`, which the caller of `implicit_scope_extensions` truncates).
            let mark = self.ext_modules.len();
            let in_scope = self.muted(|w| w.implicit_scope_extensions(t, n));
            let mut sites: Vec<Option<(SymId, super::implicits::GivenScope)>> = self.ext_modules[mark..].iter().copied().map(Some).collect();
            self.ext_modules.truncate(mark);
            for e in in_scope {
                let module = sites.iter_mut().find(|s| s.is_some_and(|(x, _)| x == e)).and_then(Option::take).map(|(_, m)| m);
                found.push((e, Found::Module(module)));
            }
            let givens = self.muted(|w| w.givens_with_extension(n, t).0);
            for ((g, _), given_ty) in givens {
                // A given's own type parameters are what the receiver decides (`given ops[T]:
                // Ops[T]`), fresh for the probe.
                let tparams = self.sig_of(g).tparams.clone();
                let subst: Vec<(TParamId, TypeId)> = tparams.iter().map(|&p| (p, self.fresh_var())).collect();
                let given_ty = self.types.subst(given_ty, &subst);
                // As `given_extension_call` finds them: the extensions of the given's type's
                // bases, each with its base as the given's type sees it.
                let Some(gc) = self.class_of(given_ty) else { continue };
                self.complete_class(gc);
                let bases: Vec<ClassId> = self.syms.class(gc).base_types.iter().map(|&(b, _)| b).collect();
                for b in bases {
                    let exts: Vec<SymId> = self.syms.class(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == n).collect();
                    for e in exts {
                        if let Some(bt) = self.base_type(given_ty, b) {
                            found.push((e, Found::Given(bt)));
                        }
                    }
                }
            }
            let mut offered: Vec<SymId> = Vec::new();
            for (e, at) in found {
                if !self.syms.sym(e).is_extension || offered.contains(&e) {
                    continue;
                }
                let trait_owner = match self.syms.sym(e).owner {
                    Owner::Class(c) if self.syms.class(c).kind != ClassKind::Object => Some(c),
                    _ => None,
                };
                let (owner_subst, prefix) = match (at, trait_owner) {
                    (Found::Given(owner_ty), _) => (self.owner_subst(owner_ty), None),
                    (Found::Module(Some(scope)), Some(c)) => self.scope_extension_site(scope, c),
                    (Found::Lexical, Some(c)) => {
                        let site = self.trait_member_site(e, c);
                        (self.trait_member_subst(site, c), None)
                    }
                    _ => (Vec::new(), None),
                };
                let call = MethodCall { recv: None, sym: e, owner_subst, ext_recv: None, prefix };
                let mark = self.snapshot();
                let applies = self.extension_applicable(&call, t);
                self.rollback(mark);
                if !applies {
                    continue;
                }
                // The signature as the call sees it: its owner's parameters and its prefix's.
                let sig = self.sig_arc(e);
                let sig = match call.prefix {
                    Some(prefix) => self.sig_seen_from(sig, prefix, e),
                    None => sig,
                };
                let detail = self.sig_text_in(&sig, &call.owner_subst);
                let receiver = self.syms.sym(e).ext_clauses as usize;
                let clauses = self.call_clauses(&sig, &call.owner_subst, receiver);
                out.push(Candidate { name: self.name_str(n), target: Cand::Sym(e), rank: Rank::Extension, detail, kind: kind::METHOD, import: None, clauses });
                offered.push(e);
                have.push(self.name_str(n));
            }
        }
    }

    /// The spellings of the extension methods a package defines, those its std files and its
    /// package object hold included.
    fn package_extension_names(&mut self, p: PkgId, prefix: &str) -> Vec<Name> {
        let mut out: Vec<Name> = self.syms.pkg(p).entries.iter().filter(|(_, e)| !e.extensions.is_empty()).map(|(&n, _)| n).collect();
        if let Some(obj) = self.syms.pkg(p).package_object {
            out.extend(self.syms.class_raw(obj).extensions.iter().map(|&s| self.syms.sym(s).name));
        }
        if !prefix.is_empty() {
            out.extend(self.std.index().by_name.iter().filter(|(&(q, n), slots)| q == p && slots.iter().any(|&(_, k)| k & crate::stdindex::TERM != 0) && self.name_ref(n).to_lowercase().starts_with(prefix)).map(|(&(_, n), _)| n));
        }
        out
    }

    /// The members reachable through the conversions in scope that take the receiver, each
    /// marked as such.
    fn conversion_candidates(&mut self, t: TypeId, prefix: &str, have: &[String], out: &mut Vec<Candidate>) {
        let targets = self.conversion_targets(t);
        // Each spelling by the conversions of the nearest level that give a member of it: one
        // conversion is what the selection applies, two of one level are ambiguous to it.
        let mut by_name: Vec<(Name, usize, Vec<(SymId, TypeId)>)> = Vec::new();
        for &(g, to, level) in &targets {
            for n in self.member_names(to) {
                if !self.offered(n, prefix) || self.name_ref(n) == "<init>" || have.iter().any(|h| h == self.name_ref(n)) {
                    continue;
                }
                match by_name.iter_mut().find(|(m, _, _)| *m == n) {
                    Some((_, l, gs)) if *l == level => gs.push((g, to)),
                    Some(_) => {}
                    None => by_name.push((n, level, vec![(g, to)])),
                }
            }
        }
        for (n, _, providers) in by_name {
            let [(g, to)] = providers[..] else { continue };
            let via = self.name_str(self.syms.sym(g).name);
            let Some((s, owner_ty)) = self.find_member(to, n) else { continue };
            let alts = match self.syms.alternatives(s) {
                Some(alts) => alts.to_vec(),
                None => vec![s],
            };
            for alt in alts {
                if !self.member_reachable(alt, false, to) {
                    continue;
                }
                let (detail, kind, clauses) = self.describe_candidate(Cand::Sym(alt), Some(owner_ty));
                out.push(Candidate { name: self.name_str(n), target: Cand::Sym(alt), rank: Rank::Conversion, detail: format!("{} (through {})", detail, via), kind, import: None, clauses });
            }
        }
    }

    // ---- the items ----

    #[allow(clippy::too_many_arguments)]
    fn render(&mut self, mut candidates: Vec<Candidate>, cx: &Context, text: &str, flags: u32, calls: bool, path: &Path, files: &Files) -> Json {
        let generation = current_generation();
        // A list with names out of the scope is asked again at the next keystroke, as one that
        // was cut is: its items' resolve names the document's version, which the client's own
        // filtering would leave behind.
        let incomplete = candidates.len() > CAP || candidates.iter().any(|c| c.import.is_some());
        candidates.truncate(CAP);
        let lines = Lines::new(text);
        let insert = lines.range(cx.insert, files.positions);
        let replace = lines.range(cx.replace, files.positions);
        let items: Vec<Json> = candidates
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let label = if c.target == Cand::Keyword { c.name.clone() } else { spelled(&c.name) };
                let snippet = if calls { self.call_snippet(&c, &label, matches!(cx.site, Site::New { .. })) } else { None };
                let written = snippet.as_deref().unwrap_or(&label);
                let edit = if flags & 1 != 0 {
                    obj([("newText", written.into()), ("insert", insert.clone()), ("replace", replace.clone())])
                } else {
                    obj([("range", insert.clone()), ("newText", written.into())])
                };
                let mut fields = vec![
                    ("label".to_string(), Json::from(label.as_str())),
                    ("kind".to_string(), c.kind.into()),
                    ("sortText".to_string(), format!("{}{:04}", c.rank as u8, i).into()),
                    ("filterText".to_string(), c.name.as_str().into()),
                    ("textEdit".to_string(), edit),
                ];
                if snippet.is_some() {
                    fields.push(("insertTextFormat".to_string(), 2u32.into()));
                }
                if !c.detail.is_empty() {
                    fields.insert(2, ("detail".to_string(), c.detail.into()));
                }
                if let Some((import, owner)) = c.import {
                    fields.insert(2, ("labelDetails".to_string(), obj([("description", owner.into())])));
                    let data = obj([("import", import.into()), ("path", path.display().to_string().into()), ("offset", cx.insert.start.into()), ("generation", generation.as_str().into())]);
                    fields.push(("data".to_string(), data));
                }
                Json::Obj(fields)
            })
            .collect();
        list(incomplete, items)
    }

    /// The snippet of a call of the item (`label` its name as written): its name, then each
    /// clause the call writes, which leaves out a `using` clause and, where a selection binds it,
    /// an extension's receiver; each parameter a placeholder of its name, numbered across the
    /// clauses, and the cursor after the last. `None` where the name stands alone: no method, or
    /// no class after `new`, an operator, no clause written, a constructor without parameters.
    fn call_snippet(&self, c: &Candidate, label: &str, new: bool) -> Option<String> {
        let method = matches!(c.target, Cand::Sym(s) if self.syms.sym(s).kind == SymKind::Def);
        let class = new && matches!(c.target, Cand::Class(_) | Cand::Unimported);
        if !(method || class) {
            return None;
        }
        snippet(label, &c.clauses, c.rank == Rank::Extension, class, |n| self.name_ref(n).to_string())
    }
}

/// The snippet of a call of `clauses` named `label` (`Worker::call_snippet`), an extension's
/// receiver clauses left out where `bound`; `None` for an operator, for no clause written, and
/// for a constructor without parameters.
fn snippet(label: &str, clauses: &[CallClause], bound: bool, ctor: bool, name: impl Fn(Name) -> String) -> Option<String> {
    if symbolic(label.strip_prefix('`').and_then(|l| l.strip_suffix('`')).unwrap_or(label)) {
        return None;
    }
    let written: Vec<&CallClause> = clauses.iter().filter(|k| !k.using && !(k.receiver && bound)).collect();
    if written.is_empty() || ctor && written.iter().all(|k| k.params.is_empty()) {
        return None;
    }
    let mut out = escaped(label);
    let mut n = 0;
    for k in written {
        out.push('(');
        for (i, p) in k.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            n += 1;
            out.push_str(&format!("${{{}:{}}}", n, escaped(&spelled(&name(p.name)))));
        }
        out.push(')');
    }
    out.push_str("$0");
    Some(out)
}

/// Whether a name is an operator (`++`, `⊕`) or ends in one after `_` (`foo_+`, `unary_!`):
/// Scala's operator characters, the ASCII ones and Unicode's other symbols. A backquoted name
/// of other characters besides (`+name`) is none.
fn symbolic(name: &str) -> bool {
    let op = |c: char| is_op(c) || !c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace();
    !name.is_empty() && name.chars().all(op) || name.rsplit_once('_').is_some_and(|(_, tail)| !tail.is_empty() && tail.chars().all(op))
}

/// Text a snippet holds as written: its `\`, `$` and `}` escaped, as the LSP snippet grammar
/// reads them.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '$' | '}') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Of the candidates of one spelling and target, the first: a name the scope and an import
/// both give is one item.
fn dedup(out: &mut Vec<Candidate>) {
    let mut seen: std::collections::HashSet<(String, String), crate::intern::FxBuild> = Default::default();
    let mut targets: Vec<(String, Cand)> = Vec::new();
    out.retain(|c| match &c.import {
        Some((path, _)) => seen.insert((c.name.clone(), path.clone())),
        None => {
            if targets.iter().any(|(n, t)| *n == c.name && *t == c.target) {
                return false;
            }
            targets.push((c.name.clone(), c.target));
            true
        }
    });
}

/// The program generation an item names: the process and the typings it settled.
fn current_generation() -> String {
    format!("{}:{}", std::process::id(), super::index::GENERATION.load(std::sync::atomic::Ordering::Relaxed))
}

/// Whether a name of a catalogue is one an import can write, backquoted where it needs to be: a
/// name a program writes, not one the compiler made.
fn importable(name: &str) -> bool {
    !name.is_empty() && name != "_" && !name.contains('$') && !name.starts_with('<') && !name.contains('`') && !name.contains('\n')
}

/// Where an import goes for the site at `offset` of the tree `site` of a file whose own tree is
/// `top`, and what goes around it: after the last top-level import before the site of the
/// site's tree (a `package p:` block's, else the file's) on a line of its own with its
/// indentation, a `;` before it where the import ends a statement the line goes on with; else
/// after the file's last package clause, a blank line between; else at the start of the file, a
/// blank line after. The file's line ending is kept.
fn import_insertion(site: &Ast, top: &Ast, text: &str, offset: u32) -> (u32, String, String) {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let before_site = |ast: &Ast| ast.import_ranges.iter().rev().find(|r| r.end <= offset).copied();
    if let Some(r) = before_site(site).or_else(|| before_site(top)) {
        let line_start = text[..r.start as usize].rfind('\n').map_or(0, |i| i + 1);
        let indent = &text[line_start..r.start as usize];
        let indent = if indent.chars().all(char::is_whitespace) { indent } else { "" };
        let rest = text[r.end as usize..].trim_start_matches([' ', '\t']);
        let separator = if rest.starts_with(';') { ";" } else { "" };
        return (r.end, format!("{}{}{}", separator, nl, indent), String::new());
    }
    if let Some(r) = top.package_ranges.last() {
        return (r.end, format!("{}{}", nl, nl), String::new());
    }
    (0, String::new(), format!("{}{}", nl, nl))
}

pub(super) fn list(incomplete: bool, items: Vec<Json>) -> Json {
    obj([("isIncomplete", incomplete.into()), ("items", Json::Arr(items))])
}

/// A name as source spells it: backquoted where it is no identifier.
pub(super) fn spelled(name: &str) -> String {
    let plain = {
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_alphabetic() || c == '_' || c == '$' => {
                let mut ok = true;
                let mut op_tail = false;
                for c in chars {
                    if op_tail {
                        ok &= is_op(c);
                    } else if c == '_' {
                        continue;
                    } else if c.is_alphanumeric() || c == '$' {
                        continue;
                    } else if is_op(c) && name.contains('_') {
                        op_tail = true;
                    } else {
                        ok = false;
                    }
                }
                ok
            }
            Some(c) if is_op(c) => name.chars().all(is_op),
            _ => false,
        }
    };
    if plain && crate::token::keyword(name).is_none() {
        name.to_string()
    } else {
        format!("`{}`", name)
    }
}

fn is_op(c: char) -> bool {
    "!#%&*+-/:<=>?@\\^|~".contains(c)
}

/// The range of a statement.
fn stmt_range(ast: &Ast, stmt: &Stmt) -> Span {
    match *stmt {
        Stmt::Def(d) => ast.def_range(d),
        Stmt::Expr(e) => ast.expr_span(e),
        Stmt::Import(i) => {
            let clauses = ast.import_stmt(i);
            match (clauses.first(), clauses.last()) {
                (Some(a), Some(b)) => a.span.to(b.span),
                _ => Span::new(u32::MAX, 0),
            }
        }
    }
}

/// The expressions an expression holds, for the walk to the offset.
fn children(ast: &Ast, e: ExprId) -> Vec<ExprId> {
    let list = |l: ast::ListRef| ast.expr_list(l).to_vec();
    match ast.expr(e) {
        Expr::Select(q, _) | Expr::TypeApply(q, _) | Expr::NamedArg(_, q) | Expr::Prefix(_, q) => vec![q],
        Expr::Apply(f, args) | Expr::UsingApply(f, args) => std::iter::once(f).chain(list(args)).collect(),
        Expr::Infix(a, _, b) | Expr::While(a, b) | Expr::Assign(a, b) => vec![a, b],
        Expr::If(c, t, e) | Expr::InlineIf(c, t, e) => [Some(c), Some(t), e].into_iter().flatten().collect(),
        Expr::Tuple(items) | Expr::New(_, items) => list(items),
        Expr::NamedTuple(_, values) => list(values),
        Expr::Interp(_, _, args) => list(args),
        Expr::Parens(x) | Expr::Typed(x, _) | Expr::Unchecked(x) | Expr::Throw(x) | Expr::Quote(x) | Expr::Splice(x) => vec![x],
        Expr::Return(x) => x.into_iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippets() {
        let names = ["x", "y", "f", "s", "type", "a}b", "x$1", "c\\d"];
        let name = |n: Name| names[n.0 as usize].to_string();
        let param = |n: u32| CallParam { name: Name(n), repeated: false, function: false, default: false };
        let clause = |using: bool, receiver: bool, params: Vec<u32>| CallClause { using, receiver, params: params.into_iter().map(param).collect() };
        let call = |label: &str, clauses: &[CallClause], bound: bool, ctor: bool| snippet(label, clauses, bound, ctor, name);
        // Numbered across the clauses, a using clause left out wherever it stands.
        let curried = [clause(true, false, vec![3]), clause(false, false, vec![0, 1]), clause(true, false, vec![3]), clause(false, false, vec![2])];
        assert_eq!(call("f", &curried, false, false).as_deref(), Some("f(${1:x}, ${2:y})(${3:f})$0"));
        // An extension's receiver: left out where a selection binds it, kept where it does not.
        let ext = [clause(false, true, vec![0]), clause(true, true, vec![3]), clause(false, false, vec![1])];
        assert_eq!(call("append", &ext, true, false).as_deref(), Some("append(${1:y})$0"));
        assert_eq!(call("append", &ext, false, false).as_deref(), Some("append(${1:x})(${2:y})$0"));
        assert_eq!(call("len", &ext[..2], true, false), None);
        // An empty clause; none but using clauses; a constructor without parameters.
        assert_eq!(call("run", &[clause(false, false, vec![])], false, false).as_deref(), Some("run()$0"));
        assert_eq!(call("run", &[clause(true, false, vec![3])], false, false), None);
        assert_eq!(call("run", &[], false, false), None);
        assert_eq!(call("Empty", &[clause(false, false, vec![])], false, true), None);
        assert_eq!(call("Point", &[clause(false, false, vec![0, 1])], false, true).as_deref(), Some("Point(${1:x}, ${2:y})$0"));
        // Operators alone; a backquoted name and the placeholders' spellings escaped.
        assert_eq!(call("++", &[clause(false, false, vec![0])], false, false), None);
        assert_eq!(call("foo_+", &[clause(false, false, vec![0])], false, false), None);
        assert_eq!(call("`⊕`", &[clause(false, false, vec![0])], false, false), None);
        assert_eq!(call("`foo_⊕`", &[clause(false, false, vec![0])], false, false), None);
        assert_eq!(call("`αβ`", &[clause(false, false, vec![0])], false, false).as_deref(), Some("`αβ`(${1:x})$0"));
        assert_eq!(call("`+name`", &[clause(false, false, vec![0])], false, false).as_deref(), Some("`+name`(${1:x})$0"));
        assert_eq!(call("`a-b`", &[clause(false, false, vec![4, 5, 6, 7])], false, false).as_deref(), Some("`a-b`(${1:`type`}, ${2:`a\\}b`}, ${3:x\\$1}, ${4:`c\\\\d`})$0"));
        assert_eq!(escaped("a$b}c\\d{"), "a\\$b\\}c\\\\d{");
    }

    #[test]
    fn spellings() {
        assert_eq!(spelled("foo"), "foo");
        assert_eq!(spelled("type"), "`type`");
        assert_eq!(spelled("my val"), "`my val`");
        assert_eq!(spelled("++"), "++");
        assert_eq!(spelled("unary_!"), "unary_!");
        assert_eq!(spelled("a-b"), "`a-b`");
    }
}
