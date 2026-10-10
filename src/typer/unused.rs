//! The unused-import check: the import selectors no name resolved through, as scalac 3.8.4's
//! `CheckUnused` reports them under `-Wunused:imports` (E198, `unused import`). teq gives them
//! under `--wunused imports` as warnings, and in a language server's session (`--index`) always,
//! as hints outside the flag (docs/TARGETS.md, "Unused imports").
//!
//! **Marks.** A name the typer resolves through an import marks the import's selector, named
//! by its file and its `Import::span` (the clause's start through the selector: one per
//! selector, and moved by `shape::compare_moving` as the index's records are). Every path that
//! resolves through an import marks: `lookup`'s hit, terms and types alike; the local an import
//! of a stable value's member makes, where it is read and not where it is made
//! (`enter_value_import`); a hiding selector (`a as _`) where a name it hides is looked up
//! through its wildcard. What a search commits is attributed once it is committed, as dotty's
//! `resolveUsage` does (`attribute`): the givens a given search's tree takes, a winner of a
//! nested search inside a candidate that failed left out (`given_winners`, kept in the memo with
//! the tree), an extension method, a conversion; the import credited is the one of highest
//! precedence that brings the symbol on the object the search read it on. The prefixes of a
//! synthetic reference are attributed as names, CheckUnused's `loopOverPrefixes`: an implicit
//! argument's object and packages (`attribute_prefixes`), a module's owners
//! (`mark_module_prefixes`). An annotation is typed quietly for its marks (`mark_annotation`), a
//! check session typing none. The code of an inline expansion marks nothing: the check of the
//! definition types the body where it stands, which marks what it uses.
//!
//! **Lifetimes.** Marks go to a journal that moves with the index's (`index_mark`): an attempt
//! the typer abandons (an extension given up for a conversion, a probe) takes its marks back, and
//! a completion query keeps none. A mark has the lifetime of the component that made it, as a
//! diagnostic does (`Diagnostics::of_bodies`): what is resolved once (the file's imports, a
//! signature written out, a parent, an export) keeps its marks across a retype of the file,
//! moved with the selectors; what a retype does again (the bodies, an inferred signature, a
//! class's imports) drops its marks and makes them again. The parallel typer's workers mark on
//! their own; the merge unions the sets (`absorb_unused`).
//!
//! **The report** runs once the typing is over (`report_unused`), after the merge and the final
//! passes: per program file every selector without a mark is `unused import` at its
//! `Import::selector_span`, in source order, but one inside an inline method, which dotty never
//! registers, and the language imports, which the parser keeps
//! apart. A file whose parse recovered from a syntax error reports none, and nor does a program
//! with an error under the flag, as scalac runs no phase after a typer that reported one; a hint
//! leaves out a file with an error alone. scalac's own selection rules (CheckUnused's
//! `matchingSelector`): a wildcard beside a named selector of the same name is not used by it;
//! a hiding selector is used only where it hides a name a lookup would have reached; a fully
//! qualified name uses no import.

use super::implicits::{GivenRef, GivenScope};
use super::{Frame, ImportTarget, ResolvedImport, Worker};
use crate::ast::{self, ImportSel};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::types::{ClassId, PkgId, SymId, TypeId};

/// Whether the check runs, and what its findings are.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Off,
    /// A language server's session without the flag: hints, which nothing counts.
    Hint,
    /// `--wunused imports`: warnings, which `--werror` counts.
    Warn,
}

/// The kinds of definitions `--wunused` reports (`warnings::Unused`), beside the imports.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DefKinds {
    pub privates: bool,
    pub locals: bool,
    pub explicits: bool,
    pub implicits: bool,
    pub patvars: bool,
    /// Whether any kind is: the one byte each typed expression tests (`defs_on`).
    some: bool,
}

impl DefKinds {
    /// The kinds a build's setting reports: where it reports one, the typed expressions'
    /// references are checked (`deprecation::references_checked`).
    pub fn of(u: &crate::warnings::Unused) -> DefKinds {
        let some = u.privates || u.locals || u.explicits || u.implicits || u.patvars;
        if some {
            super::deprecation::note_unused_defs();
        }
        DefKinds { privates: u.privates, locals: u.locals, explicits: u.explicits, implicits: u.implicits, patvars: u.patvars, some }
    }

    #[inline]
    pub fn any(self) -> bool {
        self.some
    }
}

/// A selector of the program, the key of its marks: its file and its `Import::span`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Sel {
    pub file: FileId,
    pub span: Span,
}

/// What a mark is of: an import's selector (the import check's), or a definition of the
/// program read or assigned (the other kinds', `unused_defs.rs`), by its file and the start of
/// its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    Sel(Sel),
    Read(FileId, u32),
    Assign(FileId, u32),
    /// The default of a parameter used by a call that leaves its argument out: the default
    /// getter's use, by the parameter's place.
    Default(FileId, u32),
}

impl Key {
    fn file(self) -> FileId {
        match self {
            Key::Sel(s) => s.file,
            Key::Read(f, _) | Key::Assign(f, _) | Key::Default(f, _) => f,
        }
    }
}

/// A selector and a `given T`'s type, an index into `Worker::import_sels`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SelRef(u32);

impl SelRef {
    /// Outside the check, and what no selector of the program wrote: Predef's root import,
    /// completion's own.
    pub const NONE: SelRef = SelRef(u32::MAX);
}

/// A use by what a retype keeps, and by what it does again.
const KEPT: u8 = 1;
const REDONE: u8 = 2;
/// The key of a unit's import of the package `scala.compiletime.testing`, which erases the
/// unit's checks in scalac (CheckUnused's `isNullified`): no selector stands there.
const TESTING: Span = Span { start: u32::MAX, end: u32::MAX };

/// What `attribute` matches the imports against: the symbol, its owner and name, whether it is
/// a given (a Scala 3 one apart), whether a renaming selector may be credited, and what the
/// search read it on.
#[derive(Clone, Copy)]
struct Facts {
    sym: SymId,
    owner: Owner,
    name: Name,
    given: bool,
    scala3_given: bool,
    renamed: bool,
    via: GivenScope,
}

#[derive(Default)]
pub struct Unused {
    pub mode: Mode,
    /// The kinds of definitions reported (`unused_defs.rs`), whose reads and assignments are
    /// marked while any is.
    pub defs: DefKinds,
    /// The target of the assignment being typed (`x` of `x = e`), which its typing marks
    /// assigned rather than read (CheckUnused's `AssignmentTarget`).
    pub assign_target: Option<ast::ExprId>,
    /// The symbols that stand where their definition is not: a comprehension's variable,
    /// made at the comprehension and keyed by its pattern's place, or restored from a pack and
    /// keyed by the variable it stands for (`unused_defs.rs`).
    pub(super) def_at: FxMap<SymId, u32>,
    /// The marks made since the last settling, each with its bits: what an attempt abandoned
    /// takes back (`index_drop`).
    journal: Vec<(Key, u8)>,
    /// The settled marks.
    pub(super) marks: FxMap<Key, u8>,
    /// The locals of imports of stable values' members, each with its selector, which a read
    /// of the local marks.
    aliases: FxMap<SymId, Sel>,
    /// The candidates the given searches under way took, the nested ones' first, each level's
    /// winner after what it took (`implicits.rs`): attributed when the outermost search is
    /// committed.
    pub given_winners: Vec<GivenRef>,
    /// The last report was withheld for the program's errors: the next reports every file.
    withheld: bool,
    /// How many searches of an inline expansion stand at its call (`summon_at_site`), which
    /// mark what they take although the code is an expansion's.
    pub at_site: u32,
    /// Per unit its file (`report_unused`).
    roots: Option<std::sync::Arc<Vec<FileId>>>,
    /// The object the given candidate just instantiated was read on, its reference's receiver
    /// before an inline expansion replaced it: taken by the candidate's success.
    pub receiver: Option<ClassId>,
    /// The file imports an attribution resolved quietly where nothing else needed them yet
    /// (`attribution_imports`), for the walks to come.
    quiet: FxMap<FileId, std::sync::Arc<Vec<ResolvedImport>>>,
    /// What an attribution of a package-owned target found where no import of a block or a
    /// class is visible, which depends on the file alone: the import it credits, by the file,
    /// the target and the attribution's terms (`attribute_memo`).
    memo: FxMap<(FileId, Target), Option<ResolvedImport>>,
}

/// What an attribution looks for, the key of its memo.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Sym(SymId, bool, u64),
    Class(ClassId),
    Package(PkgId),
}

impl Unused {
    /// Another worker's state at the fork: the mode, nothing marked.
    pub fn attach(&self) -> Unused {
        Unused { mode: self.mode, defs: self.defs, ..Default::default() }
    }

    #[inline]
    pub fn on(&self) -> bool {
        self.mode != Mode::Off
    }

    /// Whether the reads and assignments of definitions are marked: a kind of them is reported.
    #[inline]
    pub fn defs_on(&self) -> bool {
        self.defs.any()
    }

    /// Whether the definition at `start` of `file` is read, or with `assign` assigned, by a
    /// settled mark.
    pub(super) fn has(&self, file: FileId, start: u32, assign: bool) -> bool {
        let key = if assign { Key::Assign(file, start) } else { Key::Read(file, start) };
        self.marks.get(&key).is_some_and(|&b| b != 0)
    }

    /// Whether a call used the default of the parameter at `start` of `file`.
    pub(super) fn has_default(&self, file: FileId, start: u32) -> bool {
        self.marks.get(&Key::Default(file, start)).is_some_and(|&b| b != 0)
    }

    pub fn journal_len(&self) -> usize {
        self.journal.len()
    }

    pub fn drop_since(&mut self, mark: usize) {
        self.journal.truncate(mark);
    }

    /// The marks made since `mark` are a definition's whose typing is kept (a body typed on
    /// demand, its inferred signature cached): settled, so that an attempt under way that is
    /// abandoned does not take them back with its own.
    pub fn settle_since(&mut self, mark: usize) {
        if mark < self.journal.len() {
            for (key, bits) in self.journal.drain(mark..) {
                *self.marks.entry(key).or_default() |= bits;
            }
        }
    }

    /// The marks of `key` made since `mark` taken back.
    pub fn forget_since(&mut self, mark: usize, key: Key) {
        let since = self.journal.split_off(mark.min(self.journal.len()));
        self.journal.extend(since.into_iter().filter(|&(k, _)| k != key));
    }

    pub fn take_since(&mut self, mark: usize) -> Vec<(Key, u8)> {
        self.journal.split_off(mark.min(self.journal.len()))
    }

    pub fn put_back(&mut self, taken: Vec<(Key, u8)>) {
        self.journal.extend(taken);
    }

    fn settle(&mut self) {
        for (key, bits) in std::mem::take(&mut self.journal) {
            *self.marks.entry(key).or_default() |= bits;
        }
    }

}

impl<'a> Worker<'a> {
    /// The reference of an import resolved now: an entry of `import_sels` where the check runs.
    pub(super) fn sel_ref(&mut self, sel: Sel, bound: Option<TypeId>) -> SelRef {
        if !self.unused.on() {
            return SelRef::NONE;
        }
        self.with_loader(|w| SelRef(w.import_sels.push((sel, bound)) as u32))
    }

    /// The selector of `imp` and its `given T`'s type, where it has one.
    #[inline]
    pub(super) fn sel_of(&self, imp: ResolvedImport) -> Option<(Sel, Option<TypeId>)> {
        (imp.sel != SelRef::NONE).then(|| *self.import_sels.get(imp.sel.0 as usize))
    }

    /// Whether what is resolved now marks: the check runs, and the code is no inline expansion's.
    #[inline(always)]
    fn marking(&self) -> bool {
        self.unused.on() && (self.inline.depth == 0 || self.unused.at_site > 0)
    }

    /// `sel` is used, with the lifetime of what is being done.
    #[inline]
    fn mark_sel(&mut self, sel: Sel) {
        if (sel.file.0 as usize) >= self.files.len() || self.files[sel.file.0 as usize].is_std {
            return;
        }
        self.mark_key(Key::Sel(sel));
    }

    /// `key` is marked, with the lifetime of what is being done.
    #[inline]
    pub(super) fn mark_key(&mut self, key: Key) {
        let bits = if self.diags.of_bodies { REDONE } else { KEPT };
        if self.unused.journal.last() != Some(&(key, bits)) {
            self.unused.journal.push((key, bits));
        }
    }

    /// A name resolved through `imp`.
    #[inline]
    pub(super) fn mark_import(&mut self, imp: ResolvedImport) {
        if self.marking() {
            if let Some((sel, _)) = self.sel_of(imp) {
                self.mark_sel(sel);
            }
        }
    }

    /// `name` was looked up through the wildcard `imp`, which leaves it out: the hiding selector
    /// of its clause that takes it out (`{a as _, *}`) is used, as CheckUnused's
    /// `masksMatchingMember` counts it.
    #[cold]
    pub(super) fn mark_hiding(&mut self, imp: ResolvedImport, name: Name) {
        if !self.marking() || imp.name.is_some() || matches!(imp.target, ImportTarget::ClassGivens(_) | ImportTarget::PkgGivens(_) | ImportTarget::ValueGivens(_)) {
            return;
        }
        let Some((wildcard, _)) = self.sel_of(imp) else { return };
        let ast = self.ast(wildcard.file);
        let start = wildcard.span.start;
        let hiding = ast.imports.iter().chain(&ast.local_imports).find(|other| {
            other.span.start == start && matches!(other.sel, ImportSel::Name(n, Some(names::WILDCARD)) if n == name)
        });
        if let Some(h) = hiding {
            let sel = Sel { file: wildcard.file, span: h.span };
            self.mark_sel(sel);
        }
    }

    /// `name` was found defined in the scope of the frame `frame`, whose own imports come after
    /// `next` in the visible ones: a hiding selector among them that leaves the name out is used,
    /// since scalac's import stands inside the scope it is written in (CheckUnused's contexts),
    /// and a lookup reaches it before the definition.
    #[cold]
    pub(super) fn mark_hiding_in_scope(&mut self, name: Name, next: usize, frame: usize) {
        let n_local = self.env.imports.len();
        for k in next..n_local {
            let imp = self.import_at(k);
            if (imp.depth as usize) <= frame {
                break;
            }
            if self.import_hides(imp, name) {
                self.mark_hiding(imp, name);
            }
        }
    }

    /// Whether the package `p` is a member of a package the file's clauses enclose its code in,
    /// where it stands as a definition and no import of it is used (CheckUnused's `Definition`
    /// for a package).
    pub(super) fn enclosing_package_member(&mut self, p: PkgId) -> bool {
        let Some(parent) = self.syms.pkg(p).parent else { return false };
        parent != ROOT_PKG && self.in_clause_packages(parent)
    }

    /// The annotations of the definition `d` (its own, its parameters', its type parameters'),
    /// resolved where it stands for the marks alone: a session that captures nothing types no
    /// annotation (`capture.rs`), and an annotation's class and the names among its arguments
    /// use the imports they resolve through. Run by the walk with the definition, whose
    /// lifetime the marks take.
    pub(super) fn mark_annotations(&mut self, file: FileId, d: ast::DefId) {
        if !self.marking() {
            return;
        }
        let ast = self.ast(file);
        let def = ast.def(d);
        let (tparams, clauses): (&[ast::TypeParam], &[ast::ParamClause]) = match &def.kind {
            ast::DefKind::Fun(f) => (&f.tparams, &f.clauses),
            ast::DefKind::Class(c) => (&c.tparams, &c.clauses),
            ast::DefKind::Given(g) => (&g.tparams, &g.clauses),
            ast::DefKind::TypeAlias { tparams, .. } => (tparams, &[]),
            ast::DefKind::Val { .. } => (&[], &[]),
        };
        for a in &def.annots {
            self.mark_annotation(file, a.instance);
        }
        // The annotations of its parameters and type parameters see its type parameters.
        let inner: Vec<&ast::Annot> = tparams.iter().flat_map(|tp| tp.annots.iter()).chain(clauses.iter().flat_map(|c| c.params.iter()).flat_map(|p| ast.param_annots(p).iter())).collect();
        if !inner.is_empty() {
            let own = self.own_tparams(file, d);
            self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: own, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
            for a in inner {
                self.mark_annotation(file, a.instance);
            }
            self.env.frames.pop();
        }
        // The inline annotations of a method's or a value's header and body, and of a class's
        // header (its parameters' types, its type parameters' bounds, its parents, its self type:
        // what stands before the first statement of its body, whose definitions have their own),
        // looked up where the definition stands with its type parameters; a local definition's are
        // looked up again where it stands.
        let range = ast.def_ranges.get(d.0 as usize).copied().filter(|r| *r != crate::ast::NO_RANGE);
        if let (Some(mut range), true) = (range, !ast.inline_annots.is_empty()) {
            if let ast::DefKind::Class(cls) = &def.kind {
                if let Some(first) = cls.body.first() {
                    range.end = match *first {
                        ast::Stmt::Def(m) => ast.def_ranges.get(m.0 as usize).filter(|r| **r != crate::ast::NO_RANGE).map_or(range.end, |r| r.start),
                        ast::Stmt::Expr(e) => ast.expr_span(e).start,
                        ast::Stmt::Import(i) => ast.import_stmt(i).first().map_or(range.end, |imp| imp.span.start),
                    };
                }
            }
            let at = |a: &ast::Annot| ast.expr_span(a.instance).start;
            let from = ast.inline_annots.partition_point(|a| at(a) < range.start);
            let inline: Vec<ast::ExprId> = ast.inline_annots[from..].iter().take_while(|a| at(a) < range.end).map(|a| a.instance).collect();
            if !inline.is_empty() {
                let own = self.own_tparams(file, d);
                self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: own, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
                for instance in inline {
                    self.mark_annotation(file, instance);
                }
                self.env.frames.pop();
            }
        }
    }

    /// The type parameters of the definition `d` by name: a method's or a given's from its
    /// signature, a class's from its symbol.
    fn own_tparams(&mut self, file: FileId, d: ast::DefId) -> Vec<(Name, crate::types::TParamId)> {
        let tps: Vec<crate::types::TParamId> = match &self.ast(file).def(d).kind {
            ast::DefKind::Class(_) => match self.def_classes.get(file.0 as usize, &d).copied() {
                Some(c) => self.syms.class(c).tparams.clone(),
                None => Vec::new(),
            },
            _ => match self.def_syms.get(file.0 as usize, &d).copied() {
                Some(s) => self.sig_arc(s).tparams.clone(),
                None => Vec::new(),
            },
        };
        tps.into_iter().map(|tp| (self.syms.tparam(tp).name, tp)).collect()
    }

    /// Types the annotation `instance` (`new C[T](args)`) for its marks alone: its class, the
    /// names of its arguments with what they take (an implicit argument's search included), as a
    /// products build types it for its pickle (`capture_annotations`), quietly (what it reports
    /// at itself goes, as teq reports no unknown annotation), the tree left behind and its index
    /// records dropped, outside the capture, whose records a products build writes.
    fn mark_annotation(&mut self, file: FileId, instance: ast::ExprId) {
        let recorded = self.index_mark();
        let capture = self.prog.capture.take();
        let deps = self.deps.take();
        let outer = std::mem::replace(&mut self.typing_annotation, true);
        let mark = self.diags.items.len();
        let frame = self.var_frame_begin();
        let trail = self.snapshot();
        let pending = self.attempts.pending_len();
        match self.java_annotation_class(instance) {
            Some(c) => {
                self.type_java_annotation(c, instance);
            }
            None => {
                self.type_expr(instance, None);
            }
        }
        // The annotation's plain inline calls expand before what it reported is dropped.
        if self.attempts.pending_len() > pending {
            self.flush_pending_from(pending);
        }
        self.rollback(trail);
        self.var_frame_end(frame);
        let span = self.ast(file).expr_span(instance);
        let own = |d: &crate::source::Diagnostic| d.file == file && d.span.start >= span.start && d.span.end <= span.end;
        let elsewhere: Vec<crate::source::Diagnostic> = self.diags.items.drain(mark..).filter(|d| !own(d)).collect();
        self.diags.items.extend(elsewhere);
        self.typing_annotation = outer;
        self.deps = deps;
        self.prog.capture = capture;
        self.index_drop_records_within(recorded, file, span);
    }

    pub(super) fn mark_names_of_type(&mut self, ast: &'a ast::Ast, t: ast::TyExprId) {
        match ast.ty(t) {
            ast::TyExpr::Name(n) => {
                self.lookup_type(n);
            }
            ast::TyExpr::Select(q, _) | ast::TyExpr::Project(q, _) => match ast.ty(q) {
                ast::TyExpr::Name(n) => {
                    self.lookup_term(n);
                }
                _ => self.mark_names_of_type(ast, q),
            },
            ast::TyExpr::Apply(h, args) => {
                self.mark_names_of_type(ast, h);
                for &a in ast.ty_list(args) {
                    self.mark_names_of_type(ast, a);
                }
            }
            _ => {}
        }
    }

    /// `l == r` or `l != r`, or a pattern's equality test (`pat == scrutinee` once scalac's
    /// pattern matcher has run): CheckUnused's `resolveScoped` of `CanEqual[L, R]`, which marks
    /// the import of the first given of the enclosing scopes that conforms, walking out from
    /// the innermost; a scope that defines one ends the walk. As there, a given is a value of
    /// its type: one with parameters never conforms.
    pub(super) fn mark_can_equal(&mut self, l: TypeId, r: TypeId) {
        if !self.marking() && !self.unused.defs_on() {
            return;
        }
        let Some(ce) = self.b.can_equal else { return };
        let (l, r) = (self.widen_path(l), self.widen_path(r));
        let (l, r) = (self.widen_lit(l), self.widen_lit(r));
        let target = self.types.class(ce, &[l, r]);
        let wanted = super::implicits::Wanted::Class(ce);
        let n_local = self.env.imports.len();
        let mut next = 0;
        let mut found = Vec::new();
        for i in (0..self.env.frames.len()).rev() {
            found.clear();
            match &self.env.frames[i] {
                Frame::Locals { givens, .. } => found.extend(givens.iter().map(|&g| (g, super::implicits::GivenScope::Lexical))),
                Frame::Class(c) => {
                    let c = *c;
                    if let Some(index) = self.class_given_index(c) {
                        index.select(wanted, super::implicits::GivenScope::Lexical, &mut found);
                    }
                }
            }
            if let Some(g) = self.first_value_conforming(&found, target) {
                // The given the scope provides is read (`refUsage(found.denot.symbol)`).
                self.use_sym(g, Span::default());
                return;
            }
            while next < n_local {
                let imp = self.import_at(next);
                if (imp.depth as usize) <= i {
                    break;
                }
                next += 1;
                if self.import_brings_conforming(imp, wanted, target) {
                    return;
                }
            }
        }
        let (n_imports, fresh) = self.attribution_imports();
        for k in next..n_imports {
            let imp = self.import_at(k);
            if self.import_brings_conforming(imp, wanted, target) {
                break;
            }
        }
        self.forget_fresh_imports(fresh);
    }

    /// Whether `imp` brings a value of `CanEqual` that conforms to `target`, which ends the
    /// walk; the import is used when it is a given selector or a named one, as `resolveScoped`
    /// credits them (`sel.isGiven || sel.rename == found.name`), never a plain wildcard.
    fn import_brings_conforming(&mut self, imp: ResolvedImport, wanted: super::implicits::Wanted, target: TypeId) -> bool {
        if imp.sel == SelRef::NONE {
            return false;
        }
        // A named selector's candidates from its object's or package's index of givens, by the
        // name: no lookup of the member, which a package of the class path would load.
        let mut found = Vec::new();
        match imp.target {
            ImportTarget::ClassMember(c, n) => {
                if let Some(index) = self.class_given_index(c) {
                    index.select(wanted, GivenScope::Lexical, &mut found);
                }
                found.retain(|&(g, _)| self.syms.sym(g).name == n);
            }
            ImportTarget::PkgMember(p, n) => {
                self.given_index(p).select(wanted, GivenScope::Lexical, &mut found);
                found.retain(|&(g, _)| self.syms.sym(g).name == n);
            }
            ImportTarget::ValueMember(v, n) => {
                if let Some(index) = self.import_value_class(v).and_then(|c| self.class_given_index(c)) {
                    index.select(wanted, GivenScope::Lexical, &mut found);
                }
                found.retain(|&(g, _)| self.syms.sym(g).name == n);
            }
            _ => self.import_givens(imp, wanted, &mut found),
        }
        // A `given T` selector is credited with the givens whose result conforms to `T` alone.
        if let Some((_, Some(bound))) = self.sel_of(imp) {
            found.retain(|&(g, _)| self.result_conforms(g, bound));
        }
        if !self.any_value_conforms(&found, target) {
            return false;
        }
        let credited = imp.name.is_some() || matches!(imp.target, ImportTarget::ClassGivens(_) | ImportTarget::PkgGivens(_) | ImportTarget::ValueGivens(_));
        if credited {
            self.mark_import(imp);
        }
        true
    }

    /// Whether the final result of `sym` conforms to `bound`, its type parameters as they are
    /// (`sym.info.finalResultType <:< sel.boundTpe`): what credits a `given T` selector, where
    /// what its clause brings is `matches_import_bound`'s.
    fn result_conforms(&mut self, sym: SymId, bound: TypeId) -> bool {
        let sig = self.sig_arc(sym);
        let mark = self.snapshot();
        let fits = self.is_sub(sig.ret, bound);
        self.rollback(mark);
        fits
    }

    /// Whether one of `givens` without parameters has a type that conforms to `target`.
    fn any_value_conforms(&mut self, givens: &[GivenRef], target: TypeId) -> bool {
        self.first_value_conforming(givens, target).is_some()
    }

    /// The first of `givens` without parameters whose type conforms to `target`.
    fn first_value_conforming(&mut self, givens: &[GivenRef], target: TypeId) -> Option<SymId> {
        givens.iter().map(|&(g, _)| g).find(|&g| {
            let sig = self.sig_arc(g);
            if !sig.tparams.is_empty() || !sig.clauses.is_empty() {
                return false;
            }
            let mark = self.snapshot();
            let fits = self.is_sub(sig.ret, target);
            self.rollback(mark);
            fits
        })
    }

    /// The parents of the enum case `c`, looked up for the marks where its enum's companion
    /// stands: CheckUnused reads an enum case from outside the companion it is a member of
    /// (`prepareForTypeDef`'s `ctx.outer`), where the names of its parents use the imports.
    pub(super) fn mark_enum_case_parents(&mut self, c: ClassId, file: FileId, parents: &'a [ast::Parent]) {
        let Owner::Class(companion) = self.syms.class(c).owner else { return };
        let (owner, at) = {
            let info = self.syms.class(companion);
            (info.owner, info.span.start)
        };
        if owner == Owner::Local {
            return;
        }
        let env = self.env_at(file, owner, at);
        let ast = self.ast(file);
        self.with_env(env, |t| {
            for p in parents {
                t.mark_names_of_type(ast, p.ty);
            }
        });
    }

    /// An import whose path resolved to the package `p`: one of `scala.compiletime.testing`
    /// nullifies the unit's report, as in scalac.
    pub(super) fn note_import_package(&mut self, p: PkgId) {
        if self.marking() && self.is_compiletime_testing(p) {
            let file = self.env.file;
            self.mark_sel(Sel { file, span: TESTING });
        }
    }

    /// Whether `p` is `scala.compiletime.testing`, read by its names outward.
    fn is_compiletime_testing(&self, p: PkgId) -> bool {
        let mut at = Some(p);
        for want in ["testing", "compiletime", "scala"] {
            let Some(k) = at.filter(|&k| k != ROOT_PKG) else { return false };
            if self.interner.get(self.syms.pkg(k).name) != want {
                return false;
            }
            at = self.syms.pkg(k).parent;
        }
        at.is_none_or(|k| k == ROOT_PKG)
    }

    /// The local `local` stands for the member an import of a stable value selects.
    pub(super) fn note_import_alias(&mut self, local: SymId, imp: &ast::Import) {
        if self.unused.on() {
            let sel = Sel { file: self.env.file, span: imp.span };
            self.unused.aliases.insert(local, sel);
        }
    }

    /// A local was read: the import whose alias it is, if it is one, is used.
    #[inline]
    pub(super) fn mark_alias_read(&mut self, local: SymId) {
        if self.unused.aliases.is_empty() || !self.marking() {
            return;
        }
        if let Some(&sel) = self.unused.aliases.get(&local) {
            self.mark_sel(sel);
        }
    }

    /// The given searches under way took nothing yet, or the outermost one ended: the winners
    /// it took are attributed when `commit` says it is kept.
    pub(super) fn settle_given_winners(&mut self, commit: bool) {
        if self.unused.given_winners.is_empty() {
            return;
        }
        let winners = std::mem::take(&mut self.unused.given_winners);
        if commit {
            for &(g, via) in &winners {
                self.attribute_as(g, true, via);
                self.attribute_prefixes(g, via);
                // An implicit argument's reference reads the given (`refUsage`), and the objects
                // of its path, which its tree names (`Foo.int`).
                self.use_sym(g, Span::default());
                let mut owner = self.syms.sym(g).owner;
                for _ in 0..10 {
                    let Owner::Class(c) = owner else { break };
                    if self.syms.class(c).kind != ClassKind::Object {
                        break;
                    }
                    self.use_class(c, Span::default());
                    owner = self.syms.class(c).owner;
                }
            }
        }
    }

    /// The prefixes of a given a search took, as CheckUnused's `loopOverPrefixes` resolves those
    /// of an implicit argument, which scalac writes as an identifier when its prefix is static:
    /// the object it is read on and the objects and packages enclosing it, each attributed as a
    /// name would be (`import meridian.core.*` is used by a given of `meridian.core.Codecs`), to
    /// a non-static owner, the root, or ten of them.
    fn attribute_prefixes(&mut self, g: SymId, via: GivenScope) {
        let mut at = match via {
            GivenScope::Module(m) => Owner::Class(m),
            GivenScope::Value(_) | GivenScope::Path(_) => return,
            GivenScope::Lexical => self.syms.sym(g).owner,
        };
        for _ in 0..10 {
            match at {
                Owner::Class(c) => {
                    let (kind, owner, module) = {
                        let info = self.syms.class(c);
                        (info.kind, info.owner, info.module_sym)
                    };
                    if kind != ClassKind::Object || !matches!(owner, Owner::Class(_) | Owner::Package(_)) {
                        return;
                    }
                    if let Some(m) = module {
                        self.attribute(m);
                    }
                    at = owner;
                }
                Owner::Package(p) => {
                    let Some(parent) = self.syms.pkg(p).parent.filter(|_| p != ROOT_PKG && self.syms.pkg(p).name != names::EMPTY) else { return };
                    self.attribute_package(p, parent);
                    at = Owner::Package(parent);
                }
                Owner::Local => return,
            }
        }
    }

    /// The definition `d` makes a module (an object, the companion of a case class or an enum, a
    /// given instance; at the top level, anything but a plain class or trait, scalac wrapping the
    /// rest in the file's package object): scalac's `new O$()` of its module value names it by a
    /// synthetic identifier whose prefixes CheckUnused resolves (`loopOverPrefixes`), from the
    /// definition's owner out, as an implicit argument's (`attribute_prefixes`).
    pub(super) fn mark_module_prefixes(&mut self, file: FileId, d: ast::DefId, owner: Owner, top_level: bool) {
        if !self.marking() {
            return;
        }
        let def = self.ast(file).def(d);
        let module = match &def.kind {
            ast::DefKind::Class(cls) => matches!(cls.kind, ast::ClassKind::Object | ast::ClassKind::Enum) || def.mods & crate::ast::mods::CASE != 0,
            ast::DefKind::Given(g) => !g.body.is_empty() || top_level,
            _ => top_level,
        };
        if module {
            self.attribute_owner_prefixes(owner);
        }
    }

    /// The top-level `@main` method `sym`: scalac's proxy, the class `MainProxies.mainProxy` adds
    /// to the method's package after the file's package object, calls it by its name with the
    /// package as its prefix (`run()` of `p`, synthetic), from where a definition of the package
    /// object does not stand and the imports of the file do: CheckUnused's `resolveUsage` credits
    /// the innermost import of highest precedence that brings it, a named import of the method
    /// under its own name before a wildcard of its package.
    pub(super) fn credit_main_proxy(&mut self, sym: SymId) {
        if !self.marking() {
            return;
        }
        let (owner, name) = {
            let s = self.syms.sym(sym);
            (s.owner, s.name)
        };
        let Owner::Package(p) = owner else { return };
        let (n_imports, fresh) = self.attribution_imports();
        let mut best: Option<(ResolvedImport, u8)> = None;
        for i in 0..n_imports {
            let imp = self.import_at(i);
            if imp.sel == SelRef::NONE || imp.depth != 0 {
                continue;
            }
            let precedence = match imp.target {
                ImportTarget::PkgMember(q, n) if q == p && n == name && imp.name == Some(name) => 2,
                ImportTarget::PkgAll(q) if q == p => 3,
                _ => continue,
            };
            // The file's imports come in source order: a later one is the inner.
            if best.is_none_or(|(_, b)| precedence <= b) {
                best = Some((imp, precedence));
            }
        }
        self.forget_fresh_imports(fresh);
        self.attributed(best);
    }

    /// The owners outward from `at`, each attributed as a name would be: a class or object by its
    /// name, a package by its parent's imports; to the root, a local owner, or ten of them.
    fn attribute_owner_prefixes(&mut self, mut at: Owner) {
        for _ in 0..10 {
            match at {
                Owner::Class(c) => {
                    let (owner, module) = {
                        let info = self.syms.class(c);
                        (info.owner, info.module_sym)
                    };
                    match module {
                        Some(m) => self.attribute(m),
                        None => self.attribute_class(c),
                    }
                    at = owner;
                }
                Owner::Package(p) => {
                    let Some(parent) = self.syms.pkg(p).parent.filter(|_| p != ROOT_PKG && self.syms.pkg(p).name != names::EMPTY) else { return };
                    self.attribute_package(p, parent);
                    at = Owner::Package(parent);
                }
                Owner::Local => return,
            }
        }
    }

    /// `attribute` for the class `c` by its type's name: a named import of it or a wildcard of its
    /// package or object; none where it is a definition here (a class of the enclosing ones, a
    /// member of a package the clauses open defined in this file).
    fn attribute_class(&mut self, c: ClassId) {
        let (owner, name, file) = {
            let info = self.syms.class(c);
            (info.owner, info.name, info.file)
        };
        let memoised = matches!(owner, Owner::Package(_));
        if memoised {
            if let Some(found) = self.memo_get(Target::Class(c)) {
                return self.attributed(found.map(|imp| (imp, 0)));
            }
        }
        if self.env.frames.iter().any(|f| matches!(f, Frame::Class(k) if *k == c)) {
            return;
        }
        if let Owner::Package(p) = owner {
            let here = self.env.file;
            if (file == here || self.source(file).path == self.source(here).path) && self.in_clause_packages(p) {
                return;
            }
        }
        let mut best: Option<(ResolvedImport, u8)> = None;
        let (n_imports, fresh) = self.attribution_imports();
        for i in 0..n_imports {
            let imp = self.import_at(i);
            if imp.sel == SelRef::NONE {
                continue;
            }
            let precedence = if imp.name.is_some() { 2 } else { 3 };
            if best.is_some_and(|(_, b)| b <= precedence) {
                continue;
            }
            let brings = match (imp.target, owner) {
                (ImportTarget::PkgMember(q, n), Owner::Package(p)) => q == p && n == name,
                (ImportTarget::PkgAll(q), Owner::Package(p)) => q == p && !self.import_hides(imp, name),
                (ImportTarget::ClassMember(o, n), Owner::Class(k)) => o == k && n == name,
                (ImportTarget::ClassAll(o), Owner::Class(k)) => o == k && !self.import_hides(imp, name),
                _ => false,
            };
            if brings {
                best = Some((imp, precedence));
            }
        }
        self.forget_fresh_imports(fresh);
        if memoised {
            self.memo_put(Target::Class(c), best.map(|(imp, _)| imp));
        }
        self.attributed(best);
    }

    /// `attribute` for the package `p`, a member of `parent`: the import of highest precedence that
    /// brings it, unless the file's clauses open `parent` (a definition, CheckUnused's for a
    /// package).
    fn attribute_package(&mut self, p: PkgId, parent: PkgId) {
        if let Some(found) = self.memo_get(Target::Package(p)) {
            return self.attributed(found.map(|imp| (imp, 0)));
        }
        if self.in_clause_packages(parent) {
            return self.memo_put(Target::Package(p), None);
        }
        let name = self.syms.pkg(p).name;
        let mut best: Option<(ResolvedImport, u8)> = None;
        let (n_imports, fresh) = self.attribution_imports();
        for i in 0..n_imports {
            let imp = self.import_at(i);
            if imp.sel == SelRef::NONE {
                continue;
            }
            let precedence = if imp.name.is_some() { 2 } else { 3 };
            if best.is_some_and(|(_, b)| b <= precedence) {
                continue;
            }
            let brings = match imp.target {
                ImportTarget::PkgMember(q, n) => q == parent && n == name,
                ImportTarget::PkgAll(q) => q == parent && !self.import_hides(imp, name),
                _ => false,
            };
            if brings {
                best = Some((imp, precedence));
            }
        }
        self.forget_fresh_imports(fresh);
        self.memo_put(Target::Package(p), best.map(|(imp, _)| imp));
        self.attributed(best);
    }

    /// The count of the visible imports for an attribution, which is no reason to resolve the
    /// file's imports: where nothing resolved them yet they are resolved for the walk alone, what
    /// that reports at the file's import clauses dropped, and forgotten after it
    /// (`forget_fresh_imports`), so that a build reports what resolving one reports where a name
    /// first needs it, as it does without the check (an import of a member the lean std lacks
    /// that nothing uses is no error).
    fn attribution_imports(&mut self) -> (usize, bool) {
        let file = self.env.file;
        let fresh = self.file_imports[file.0 as usize].is_none();
        if !fresh {
            return (self.import_count(), false);
        }
        // Resolved quietly once, then kept apart for the walks to come.
        if let Some(quiet) = self.unused.quiet.get(&file).cloned() {
            self.file_imports[file.0 as usize] = Some(quiet);
            return (self.import_count(), true);
        }
        let mark = self.diags.items.len();
        let n = self.import_count();
        let ast = self.ast(file);
        let at_imports = |d: &crate::source::Diagnostic| d.file == file && ast.imports.iter().any(|imp| imp.span.start <= d.span.start && d.span.end <= imp.span.end);
        let kept: Vec<crate::source::Diagnostic> = self.diags.items.drain(mark..).filter(|d| !at_imports(d)).collect();
        self.diags.items.extend(kept);
        if let Some(list) = self.file_imports[file.0 as usize].clone() {
            self.unused.quiet.insert(file, list);
        }
        (n, true)
    }

    /// The memoised answer of an attribution of `target`, where no import of a block or a class
    /// is visible and the answer depends on the file alone.
    fn memo_get(&self, target: Target) -> Option<Option<ResolvedImport>> {
        if !self.env.imports.is_empty() {
            return None;
        }
        self.unused.memo.get(&(self.env.file, target)).copied()
    }

    fn memo_put(&mut self, target: Target, found: Option<ResolvedImport>) {
        if self.env.imports.is_empty() {
            self.unused.memo.insert((self.env.file, target), found);
        }
    }

    fn forget_fresh_imports(&mut self, fresh: bool) {
        if fresh {
            self.file_imports[self.env.file.0 as usize] = None;
        }
    }

    /// Whether `p` is a package the file's `package` clauses open: `pkg_chain` without the
    /// `scala` and `java.lang` it adds behind them.
    pub(super) fn in_clause_packages(&mut self, p: PkgId) -> bool {
        let chain = self.pkg_chain();
        let clauses = self.cur_ast().package_clauses.len().max(1).min(chain.len());
        chain[..clauses].contains(&p)
    }

    /// dotty's `resolveUsage` for what a search committed (a given, an extension method, a
    /// conversion), which it found without a lookup by name: the import of highest precedence
    /// that brings `sym` where the search stands, walking out from the innermost scope; a named
    /// import beats a wildcard, and of one precedence the innermost wins. A scope that defines
    /// `sym` ends the walk, and so does an enclosing class of this file that has it as a member,
    /// which takes it from any import met before (CheckUnused's `Definition`).
    pub(super) fn attribute(&mut self, sym: SymId) {
        self.attribute_as(sym, true, GivenScope::Lexical);
    }

    /// `attribute` for what a search read on the object or value `via`, the receiver a
    /// conversion was applied through.
    pub(super) fn attribute_via(&mut self, sym: SymId, via: GivenScope) {
        self.attribute_as(sym, true, via);
    }

    /// An extension method applied by the extension syntax: as `attribute`, but a selector that
    /// renames it is not credited, since CheckUnused matches a renamed selector by the name the
    /// typer leaves in the tree, the method's own (`import A.f as f2` stays unused by `2.f2`).
    pub(super) fn attribute_extension(&mut self, sym: SymId) {
        self.attribute_as(sym, false, GivenScope::Lexical);
    }

    /// `via` is what a search read the symbol on: an object (`Module`) or a stable value
    /// (`Value`) takes only an import of that one, CheckUnused's prefix test.
    fn attribute_as(&mut self, sym: SymId, renamed: bool, via: GivenScope) {
        if !self.marking() {
            return;
        }
        // The local an import of a stable value's member made (`import c.n` of a given `n`): the
        // search took it, so the import is used.
        if let Some(&sel) = self.unused.aliases.get(&sym) {
            return self.mark_sel(sel);
        }
        let (owner, file, name) = {
            let s = self.syms.sym(sym);
            (s.owner, s.file, s.name)
        };
        // A member of a package the file's clauses open, defined in this file (the `package p:`
        // blocks of a file of its path): a definition, which takes it from any import.
        if let Owner::Package(p) = owner {
            let here = self.env.file;
            if (file == here || self.source(file).path == self.source(here).path) && self.in_clause_packages(p) {
                return;
            }
        }
        // Past the frames that may define it, with no import of a block or a class visible, the
        // answer is the file's imports': memoised.
        let via_code = via.memo_code();
        let key = Target::Sym(sym, renamed, via_code);
        let given = self.syms.is_given(sym);
        let scala3_given = self.syms.sym(sym).mods & crate::ast::mods::GIVEN != 0 || self.syms.sym(sym).kind == SymKind::Given;
        let facts = Facts { sym, owner, name, given, scala3_given, renamed, via };
        let n_local = self.env.imports.len();
        let mut best: Option<(ResolvedImport, u8)> = None;
        let mut next = 0;
        for i in (0..self.env.frames.len()).rev() {
            let class = match &self.env.frames[i] {
                Frame::Locals { names, givens, .. } => {
                    if owner == Owner::Local && (givens.contains(&sym) || names.iter().any(|&(_, s)| s == sym)) {
                        return self.attributed(best);
                    }
                    None
                }
                Frame::Class(c) => Some(*c),
            };
            let defines = match (class, owner) {
                (Some(c), Owner::Class(o)) => (c == o || self.derives_from(c, o)) && file == self.env.file,
                _ => false,
            };
            if defines {
                return;
            }
            while next < n_local {
                let imp = self.import_at(next);
                if (imp.depth as usize) <= i {
                    break;
                }
                next += 1;
                self.consider_import(imp, &facts, &mut best);
            }
        }
        let memoised = n_local == 0;
        if memoised {
            if let Some(found) = self.memo_get(key) {
                return self.attributed(found.map(|imp| (imp, 0)));
            }
        }
        let (n_imports, fresh) = self.attribution_imports();
        for i in next..n_imports {
            let imp = self.import_at(i);
            self.consider_import(imp, &facts, &mut best);
        }
        self.forget_fresh_imports(fresh);
        if memoised {
            self.memo_put(key, best.map(|(imp, _)| imp));
        }
        self.attributed(best);
    }

    fn attributed(&mut self, best: Option<(ResolvedImport, u8)>) {
        if let Some((imp, _)) = best {
            self.mark_import(imp);
        }
    }

    fn consider_import(&mut self, imp: ResolvedImport, facts: &Facts, best: &mut Option<(ResolvedImport, u8)>) {
        let Facts { sym, owner, name, given, scala3_given, renamed, via } = *facts;
        let Some((_, bound)) = self.sel_of(imp) else { return };
        if !renamed && imp.name.is_some_and(|n| n != name) {
            return;
        }
        // CheckUnused's precedence: a named import 2, a wildcard 3; a later find replaces an
        // earlier one only when it is stronger.
        let precedence = if imp.name.is_some() { 2 } else { 3 };
        if best.is_some_and(|(_, p)| p <= precedence) {
            return;
        }
        if imp.name.is_none() && self.import_hides(imp, name) {
            return;
        }
        // The import's prefix: the object or value it reads the member on, which must be the
        // one the search read it on where that is known.
        let member_of = |t: &mut Self, c: ClassId| match (owner, via) {
            (_, GivenScope::Module(m)) => c == m,
            (_, GivenScope::Value(_) | GivenScope::Path(_)) => false,
            (Owner::Class(o), GivenScope::Lexical) => o == c || t.derives_from(c, o),
            _ => false,
        };
        let in_pkg = |t: &Self, p: PkgId| match via {
            GivenScope::Module(m) => t.syms.pkg(p).package_object == Some(m),
            GivenScope::Value(_) | GivenScope::Path(_) => false,
            GivenScope::Lexical => owner == Owner::Package(p),
        };
        let of_value = |t: &mut Self, v: crate::typer::ValueImport| match via {
            GivenScope::Value(w) => v == w,
            GivenScope::Module(_) | GivenScope::Path(_) => false,
            GivenScope::Lexical => t.import_value_class(v).is_some_and(|c| matches!(owner, Owner::Class(o) if o == c || t.derives_from(c, o))),
        };
        let brings = match imp.target {
            ImportTarget::ClassMember(c, n) => n == name && member_of(self, c),
            ImportTarget::ClassAll(c) => !scala3_given && member_of(self, c),
            ImportTarget::ClassGivens(c) => given && member_of(self, c),
            ImportTarget::PkgMember(p, n) => n == name && in_pkg(self, p),
            ImportTarget::PkgAll(p) => !scala3_given && in_pkg(self, p),
            ImportTarget::PkgGivens(p) => given && in_pkg(self, p),
            ImportTarget::ValueMember(v, n) => n == name && of_value(self, v),
            ImportTarget::ValueAll(v) => !scala3_given && of_value(self, v),
            ImportTarget::ValueGivens(v) => given && of_value(self, v),
            ImportTarget::UnimportPredef | ImportTarget::Unresolved => false,
        };
        if brings && bound.is_none_or(|bound| self.result_conforms(sym, bound)) {
            *best = Some((imp, precedence));
        }
    }

    /// Another worker's marks, after the fork: unioned with this one's.
    pub(super) fn absorb_unused(&mut self, other: &mut Worker<'a>) {
        other.unused.settle();
        for (key, bits) in std::mem::take(&mut other.unused.marks) {
            *self.unused.marks.entry(key).or_default() |= bits;
        }
        self.unused.journal.append(&mut other.unused.journal);
        // The locals of a body are out of scope once it is typed, and another worker's are
        // renumbered by the merge.
        self.unused.aliases.clear();
    }

    /// A retype of `file` types its bodies again: the marks their typing made go, those of what
    /// is resolved once move to where the edit left their selectors.
    pub(super) fn forget_unused(&mut self, file: FileId, remap: &crate::shape::Remap) {
        if !self.unused.on() && !self.unused.defs_on() {
            return;
        }
        self.unused.settle();
        let of_file: Vec<(Key, u8)> = self.unused.marks.iter().filter(|(key, _)| key.file() == file).map(|(&k, &b)| (k, b)).collect();
        for (key, bits) in of_file {
            self.unused.marks.remove(&key);
            let kept = bits & KEPT;
            if kept == 0 {
                continue;
            }
            // A selector moves with its import, the file's import of `scala.compiletime.testing`
            // being one of its once-resolved imports; a definition with its name, where it stands
            // outside the bodies.
            let moved = match key {
                Key::Sel(sel) if sel.span == TESTING => Some(Key::Sel(sel)),
                Key::Sel(sel) => remap.moved.get(&sel.span).map(|&span| Key::Sel(Sel { file, span })),
                Key::Read(_, at) => remap.spans.get(&at).map(|s| Key::Read(file, s.start)),
                Key::Assign(_, at) => remap.spans.get(&at).map(|s| Key::Assign(file, s.start)),
                Key::Default(_, at) => remap.spans.get(&at).map(|s| Key::Default(file, s.start)),
            };
            if let Some(key) = moved {
                *self.unused.marks.entry(key).or_default() |= kept;
            }
        }
        // What the attributions found reads the file's imports, which move.
        self.unused.memo.clear();
        self.unused.quiet.remove(&file);
        // The file's imports stay resolved across the retype: their selectors' keys move with the
        // edit, so that the bodies typed again mark the selectors where they now stand.
        if let Some(list) = self.file_imports[file.0 as usize].clone() {
            let mut list = (*list).clone();
            for imp in list.iter_mut() {
                let Some((sel, bound)) = self.sel_of(*imp) else { continue };
                if let Some(&span) = remap.moved.get(&sel.span) {
                    imp.sel = self.sel_ref(Sel { file: sel.file, span }, bound);
                }
            }
            self.file_imports[file.0 as usize] = Some(std::sync::Arc::new(list));
        }
        self.unused.aliases.clear();
    }

    /// The report: every unused selector of the program's files, or of `only` (the files a
    /// retype typed again) where nothing else changed, as a warning under the flag and a hint
    /// in a language server's session outside it, at the selector, in source order.
    pub fn report_unused(&mut self, only: Option<&[FileId]>) {
        if !self.unused.on() && !self.unused.defs_on() {
            return;
        }
        self.unused.settle();
        self.unused.aliases.clear();
        // A file's `package p:` blocks are units of their own (`frontend::lay_out`), each a copy
        // of the file with the file's imports before the blocks: a selector is the file's, used
        // where any of its units marked it, and reported once, at the file.
        let root = self.unit_roots();
        let only = if self.unused.withheld { None } else { only };
        let roots: Vec<FileId> = match only {
            Some(fs) => {
                let mut r: Vec<FileId> = fs.iter().map(|f| root.get(f.0 as usize).copied().unwrap_or(*f)).collect();
                r.sort();
                r.dedup();
                r
            }
            None => (0..self.files.len() as u32).map(FileId).filter(|f| root.get(f.0 as usize) == Some(f)).collect(),
        };
        let hint = self.unused.mode == Mode::Hint;
        let errors: Vec<FileId> = self
            .diags
            .items
            .iter()
            .filter(|d| !d.is_warning && !d.dependent)
            .map(|d| d.file)
            .chain(self.diags.syntax.iter().map(|d| d.file))
            .map(|f| root.get(f.0 as usize).copied().unwrap_or(f))
            .collect();
        // scalac runs no phase after a typer that reported an error: every file's earlier
        // findings go with it, the next report gives them all again.
        self.unused.withheld = !hint && (!errors.is_empty() || self.diags.syntax_errors);
        if self.unused.withheld {
            self.diags.items.retain(|d| !d.unnecessary);
            return;
        }
        let reported = |d: &crate::source::Diagnostic| d.unnecessary && (only.is_none() || roots.contains(&d.file));
        self.diags.items.retain(|d| !reported(d));
        let mut units: FxMap<FileId, Vec<FileId>> = FxMap::default();
        for (u, &r) in root.iter().enumerate() {
            if roots.contains(&r) {
                units.entry(r).or_default().push(FileId(u as u32));
            }
        }
        for file in roots {
            if !self.program_source(file) || errors.contains(&file) {
                continue;
            }
            let of_file = units.remove(&file).unwrap_or_else(|| vec![file]);
            let testing = |t: &Self, u: FileId| t.unused.marks.contains_key(&Key::Sel(Sel { file: u, span: TESTING }));
            if of_file.iter().any(|&u| !self.ast(u).recoveries.is_empty() || testing(self, u)) {
                continue;
            }
            // The definitions no mark reads (`unused_defs.rs`), each at its unit.
            if self.unused.defs_on() {
                for (u, span, msg) in self.unused_definitions(&of_file) {
                    self.diags.unnecessary(u, span, msg, false, None);
                }
            }
            if !self.unused.on() {
                continue;
            }
            let mut selectors: Vec<&ast::Import> = Vec::new();
            for &u in &of_file {
                let ast = self.ast(u);
                // dotty registers no import of an inline method's body.
                let inline_defs: Vec<Span> = ast
                    .defs
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.mods & crate::ast::mods::INLINE != 0 && matches!(d.kind, ast::DefKind::Fun(_)))
                    .filter_map(|(i, _)| ast.def_ranges.get(i).copied())
                    .filter(|r| *r != crate::ast::NO_RANGE)
                    .collect();
                let in_inline = |s: Span| inline_defs.iter().any(|r| r.start <= s.start && s.end <= r.end);
                selectors.extend(ast.imports.iter().chain(ast.local_imports.iter().filter(|imp| !in_inline(imp.span))));
            }
            selectors.sort_by_key(|imp| (imp.selector_span.start, imp.span.start));
            selectors.dedup_by_key(|imp| imp.span);
            let used: Vec<bool> = selectors.iter().map(|imp| of_file.iter().any(|&u| self.unused.marks.get(&Key::Sel(Sel { file: u, span: imp.span })).is_some_and(|&b| b != 0))).collect();
            // The edits removing them, which a language server offers (`unused_edits.rs`).
            let mut actions = super::unused_edits::import_actions(&self.files[file.0 as usize].text, &selectors, &used);
            for (i, imp) in selectors.into_iter().enumerate() {
                if !used[i] {
                    let origin = self.selector_origin(file, imp);
                    self.diags.unnecessary(file, imp.selector_span, "unused import", hint, Some(origin));
                    if let (Some(a), Some(d)) = (actions[i].take(), self.diags.items.last_mut()) {
                        d.action = Some(Box::new(a));
                    }
                }
            }
        }
    }

    /// The origin of the warning of an unused selector of `file`, the import of it alone as
    /// scalac shows it without `import ` (`CheckUnused.warnImport`): `O.x`, `p.{a as b}`, `p.*`.
    fn selector_origin(&self, file: FileId, imp: &ast::Import) -> Box<str> {
        let mut out: String = imp.path.iter().map(|&n| self.interner.get(n)).collect::<Vec<_>>().join(".");
        if !out.is_empty() {
            out.push('.');
        }
        let written = self.files.as_slice().get(file.0 as usize).and_then(|f| f.text.get(imp.selector_span.start as usize..imp.selector_span.end as usize)).unwrap_or("");
        match &imp.sel {
            ImportSel::Wildcard => out.push('*'),
            ImportSel::Name(n, None) => out.push_str(self.interner.get(*n)),
            _ => {
                out.push('{');
                out.push_str(written);
                out.push('}');
            }
        }
        out.into_boxed_str()
    }

    /// Per unit the file it is a unit of: itself, or for a `package p:` block of a file the
    /// file, which comes first with its path.
    fn unit_roots(&mut self) -> std::sync::Arc<Vec<FileId>> {
        if let Some(r) = &self.unused.roots {
            if r.len() == self.files.len() {
                return r.clone();
            }
        }
        let mut first: FxMap<&str, FileId> = FxMap::default();
        let roots: Vec<FileId> = (0..self.files.len()).map(|i| *first.entry(self.files[i].path.as_str()).or_insert(FileId(i as u32))).collect();
        let roots = std::sync::Arc::new(roots);
        self.unused.roots = Some(roots.clone());
        roots
    }
}

