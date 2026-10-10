use super::{Env, Frame, ImportTarget, ResolvedImport, Worker, ValueImport};
use crate::ast::{self, mods, DefKind, ImportSel, ListRef, TyExpr, TyExprId};
use crate::intern::Name;
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::types::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeRef {
    Class(ClassId),
    Alias(AliasId),
    Param(TParamId),
    /// An abstract type member met in the body of the class: `C.this.T`.
    Member(ClassId, Name),
    /// A type member of a stable value an import opens: `v.T` after `import v.T`.
    ValueMember(ValueImport, Name),
}

#[derive(Clone, Copy, Debug)]
pub enum TermRef {
    Local(SymId),
    This(ClassId, SymId),
    ModuleMember(ClassId, SymId),
    Global(SymId),
    Class(ClassId),
    Package(PkgId),
    /// A member of the stable val of a package or an object, imported from the val.
    ValueMember(ValueImport, SymId),
    /// The constructor proxy of a class nested in the class of such a val, imported from it
    /// (`import o.*; Inner(1)` is `o.Inner(1)`, dotty's `NamerOps.addConstructorProxies`).
    ValueClass(ValueImport, ClassId),
    /// The proxy of a class an object inherits from a class or trait that declares it, whose
    /// instance the object is (`import Schema.*; Record(f)` is `Schema.Record(f)`).
    ModuleClass(ClassId, ClassId),
    /// The `self =>` alias of an enclosing class: its `this`.
    SelfAlias(ClassId),
}

impl TermRef {
    pub fn sym(self) -> Option<SymId> {
        match self {
            TermRef::Local(s) | TermRef::This(_, s) | TermRef::ModuleMember(_, s) | TermRef::Global(s) | TermRef::ValueMember(_, s) => {
                Some(s)
            }
            TermRef::Class(_) | TermRef::Package(_) | TermRef::SelfAlias(_) | TermRef::ValueClass(..) | TermRef::ModuleClass(..) => None,
        }
    }
}

/// An import of an extension's name (`Worker::import_alternatives`): the object it imports from,
/// where it is one, on which an extension it inherits is called, and the extensions it brings.
#[derive(PartialEq)]
pub(super) struct ImportAlternative {
    pub module: Option<ClassId>,
    pub syms: Vec<SymId>,
}

impl<'a> Worker<'a> {
    // ---- imports ----

    /// How many imports are visible: those of the enclosing blocks and bodies, then the file's,
    /// which are resolved on first use.
    #[inline]
    pub(super) fn import_count(&mut self) -> usize {
        let file = self.env.file;
        if self.file_imports[file.0 as usize].is_none() {
            self.resolve_file_imports(file);
        }
        self.env.imports.len() + self.file_imports[file.0 as usize].as_ref().map_or(0, |v| v.len())
    }

    /// The visible imports from the innermost scope outwards; a scope lists its named imports
    /// before its wildcards, later ones first.
    #[inline(always)]
    pub(super) fn import_at(&self, i: usize) -> ResolvedImport {
        let local = &self.env.imports;
        match local.len().checked_sub(i + 1) {
            Some(at) => local[at],
            None => self.file_imports[self.env.file.0 as usize].as_ref().unwrap()[i - local.len()],
        }
    }

    #[inline]
    pub(super) fn import_hides(&self, imp: ResolvedImport, name: Name) -> bool {
        !imp.hidden.is_empty() && self.import_hidden.as_slice()[imp.hidden.range()].contains(&name)
    }

    /// An import sees the ones before it, so `import a.b.Models.*` can be followed by
    /// `import Kind.*`.
    #[cold]
    /// The file's top-level imports each with its clause's span, resolved as
    /// `resolve_file_imports` resolves them, for the TASTy writer, which pickles them where the
    /// source has them.
    pub fn file_import_clauses(&mut self, file: FileId) -> Vec<(Span, ResolvedImport)> {
        let clauses = &self.ast(file).imports;
        let env = Env { file, frames: Vec::new(), imports: Vec::new() };
        self.once(|t| t.with_env(env, |t| clauses.iter().filter_map(|imp| t.resolve_import(imp, clauses).map(|r| (imp.span, r))).collect()))
    }

    fn resolve_file_imports(&mut self, file: FileId) {
        self.file_imports[file.0 as usize] = Some(std::sync::Arc::new(Vec::new()));
        let clauses = &self.ast(file).imports;
        let env = Env { file, frames: Vec::new(), imports: Vec::new() };
        self.once(|t| {
            t.with_env(env, |t| {
                for imp in clauses {
                    let Some(resolved) = t.resolve_import(imp, clauses) else { continue };
                    let list = std::sync::Arc::make_mut(t.file_imports[file.0 as usize].as_mut().unwrap());
                    let at = match resolved.name {
                        Some(_) => 0,
                        None => list.iter().position(|other| other.name.is_none()).unwrap_or(list.len()),
                    };
                    list.insert(at, resolved);
                }
            })
        });
    }

    /// Resolves the imports of a statement in the current environment and brings them into the
    /// scope that starts at `scope` in `Env::imports`.
    /// What it answers, in a body under the definition check, is the imports it entered, which
    /// the body's record keeps.
    #[cold]
    pub(super) fn enter_imports(&mut self, clauses: &'a [ast::Import], scope: usize) -> Vec<ResolvedImport> {
        let noting = self.checks_inline_definition() && self.quote.level == 0;
        // What a block of a captured body imports the capture keeps too (`capture_block_imports`).
        let keeping = noting || self.capturing();
        self.note_quiet_imports(clauses);
        let mut entered = Vec::new();
        for imp in clauses {
            // An inline expansion's imports are its definition's, recorded where it is typed.
            if self.deps.is_some() && self.inline.depth == 0 {
                self.deps_import(imp);
            }
            if let Some(local) = self.enter_value_import(imp) {
                if noting {
                    let tree = self.inline.args[&local].expr;
                    self.note_value_alias(local, tree);
                }
                continue;
            }
            if let Some(resolved) = self.resolve_import(imp, clauses) {
                if self.inline.depth > 0 {
                    let frame = self.env.frames.len() - 1;
                    self.inline.body_imports.push((frame, resolved));
                } else if matches!(imp.sel, ImportSel::Name(..)) {
                    self.check_value_selector(resolved.target, imp.selector_span);
                }
                if keeping {
                    entered.push(resolved);
                }
                self.env.push_import(scope, resolved);
            }
        }
        entered
    }

    /// `import v.{m as a}` in a block, the prefix a stable value (SLS 4.7): `a` names `v.m`,
    /// a given when `m` is one (`import init.{Underlying as Init}` over an implicit
    /// `val Underlying: Type[Underlying]`). The local stands for the selection, which each use
    /// copies. A prefix that reaches an object or a package is the ordinary import's. The
    /// local, where the import is one of these.
    #[cold]
    pub(super) fn enter_value_import(&mut self, imp: &ast::Import) -> Option<SymId> {
        let ImportSel::Name(member, alias) = imp.sel else { return None };
        if alias == Some(names::WILDCARD) {
            return None;
        }
        let Some((&head, rest)) = imp.path.split_first() else { return None };
        let Some(r) = self.lookup_term(head) else { return None };
        let Some(s) = r.sym() else { return None };
        if !matches!(self.syms.sym(s).kind, SymKind::Val | SymKind::Param | SymKind::Given) || self.syms.sym(s).by_name {
            return None;
        }
        // A local object or one nested in a class is imported from as an object is, its types
        // included.
        if self.local_module_of_sym(s).is_some() || self.inner_object_of_sym(s).is_some() {
            return None;
        }
        let stable = |t: &mut Self, ty: TypeId, n: Name| match t.find_member(ty, n) {
            Some((m, _)) => matches!(t.syms.sym(m).kind, SymKind::Val | SymKind::Given) && !t.syms.sym(m).by_name,
            None => false,
        };
        // The path's typing is an attempt: kept where the import binds, gone where it does not.
        let mark = self.attempt();
        let Some(super::apply::Callee::Value(mut te, mut ty)) = self.term_callee(r, head, imp.span) else {
            self.retract(mark);
            return None;
        };
        for &seg in rest {
            if !stable(self, ty, seg) {
                self.retract(mark);
                return None;
            }
            (te, ty) = self.apply_member(te, ty, seg, None, Vec::new(), imp.span, None);
        }
        // A method without parameters is read at each use as a value is (`import a.toInt as
        // ti`); one with parameters is not entered. A type member alone (`import m.{Key}`) is
        // the ordinary import's, which binds it as `m.Key`.
        let parameterless = |t: &mut Self, ty: TypeId, n: Name| match t.find_member(ty, n) {
            Some((m, _)) if matches!(t.syms.sym(m).kind, SymKind::Def) => {
                let sig = t.sig_of(m);
                sig.tparams.is_empty() && sig.clauses.iter().all(|c| c.params.is_empty())
            }
            _ => false,
        };
        if !stable(self, ty, member) && !parameterless(self, ty, member) {
            self.retract(mark);
            return None;
        }
        self.close(mark);
        let (m, _) = self.find_member(ty, member).unwrap();
        let given = self.syms.is_given(m);
        let (te, ty) = self.apply_member(te, ty, member, None, Vec::new(), imp.span, None);
        let name = alias.unwrap_or(member);
        let local = self.syms.new_sym(name, SymKind::Val, if given { crate::ast::mods::IMPLICIT } else { 0 }, Owner::Local, self.env.file, None, imp.span);
        let sig = self.value_sig(ty);
        let mut info = self.syms.sym_mut(local);
        info.sig = Some(sig);
        info.state().set(crate::symbols::Completion::Done);
        self.inline.args.insert(local, super::inline::InlineArg { expr: te, ty, source: None });
        self.inline.import_aliases.push(local);
        self.note_import_alias(local, imp);
        self.bind_local(name, local);
        if given {
            self.bind_given(local);
        }
        Some(local)
    }

    #[cold]
    pub(super) fn class_imports(&mut self, c: ClassId) -> Arc<[(u32, ResolvedImport)]> {
        if let Some(found) = self.class_imports.get(&c) {
            return found.clone();
        }
        self.class_imports.insert(c, Arc::from(Vec::new()));
        let (file, owner, span, def) = {
            let i = self.syms.class(c);
            (i.file, i.owner, i.span, i.def)
        };
        let ast = self.ast(file);
        let body: &[ast::Stmt] = match def.map(|d| &ast.def(d).kind) {
            Some(DefKind::Class(cls)) => &cls.body,
            Some(DefKind::Given(g)) => &g.body,
            _ => &[],
        };
        let mut env = match owner {
            Owner::Local => self.anon_env(c),
            _ => self.env_at(file, owner, span.start),
        };
        env.frames.push(Frame::Class(c));
        // A retype forgets the imports of the file's classes, which hold their positions.
        let resolved = self.again(|t| {
            t.with_env(env, |t| {
                let scope = t.env.imports.len();
                let mut out = Vec::new();
                let outer = if t.deps.is_some() { Some(t.deps_enter_class_body(c)) } else { None };
                // A class under `@nowarn` (its own or an enclosing class's) silences its imports.
                let quiet = t.unused.on() && t.class_nowarn(c) > 0;
                for stmt in body {
                    let ast::Stmt::Import(i) = *stmt else { continue };
                    let clauses = ast.import_stmt(i);
                    if quiet {
                        let nowarn = std::mem::replace(&mut t.nowarn, 1);
                        t.note_quiet_imports(clauses);
                        t.nowarn = nowarn;
                    }
                    for imp in clauses {
                        if let Some(resolved) = t.resolve_import(imp, clauses) {
                            out.push((imp.span.start, resolved));
                            t.env.push_import(scope, resolved);
                        }
                        if t.deps.is_some() {
                            t.deps_import(imp);
                        }
                    }
                }
                if let Some(outer) = outer {
                    t.deps_leave(outer);
                }
                out
            })
        });
        let resolved: Arc<[(u32, ResolvedImport)]> = resolved.into();
        self.class_imports.insert(c, resolved.clone());
        resolved
    }

    /// The imports of a statement of the body of `c`, for the statements after it.
    #[cold]
    pub(super) fn enter_class_imports(&mut self, c: ClassId, stmt: u32, scope: usize) {
        let clauses = self.cur_ast().import_stmt(stmt);
        let (Some(first), Some(last)) = (clauses.first(), clauses.last()) else { return };
        let (from, to) = (first.span.start, last.span.start);
        for &(start, imp) in self.class_imports(c).iter() {
            if start >= from && start <= to {
                self.env.push_import(scope, imp);
                // A named selector through a value is checked against what the value has, found
                // in the statement's clauses only then.
                if let ImportTarget::ValueMember(_, member) = imp.target {
                    let selector = self.cur_ast().import_stmt(stmt).iter().find(|a| a.span.start == start && matches!(a.sel, ImportSel::Name(m, _) if m == member)).map(|a| a.selector_span);
                    if let Some(at) = selector {
                        self.check_value_selector(imp.target, at);
                    }
                }
            }
        }
    }

    /// A named selector of an import through a value names a member of the value's type, a term
    /// or a type (dotty's `Checking.checkImportSelectors`, `checkIdent`, which `typedImport` runs
    /// whether or not the name is read): "value x is not a member of T" otherwise. Checked where
    /// the import's statement is typed, as the class's or the block's.
    pub(super) fn check_value_selector(&mut self, target: ImportTarget, at: Span) {
        let ImportTarget::ValueMember(v, member) = target else { return };
        let ty = self.import_value_ret(v);
        let ty = self.zonk(ty);
        if ty == ERROR || self.types.contains_error(ty) || matches!(self.types.get(ty), Type::Var(_)) {
            return;
        }
        // Only a class's own type has members this lookup knows all of; anything else is taken
        // as the import was before, its names read where they are used: a builtin type's members
        // are the typer's primitives (`n.toLong`), an opaque type's its underlying type's, and a
        // type parameter's, an intersection's or a refinement's go through bounds and parts.
        let shape = self.deref_alias(ty);
        let Type::Class(c, _) = self.types.get(shape) else { return };
        if matches!(self.syms.class(c).kind, ClassKind::Builtin | ClassKind::Opaque) || self.is_builtin_type(shape) {
            return;
        }
        let extension = self.class_of(ty).is_some_and(|c| {
            self.complete_class(c);
            let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
            bases.into_iter().any(|b| self.syms.class(b).extensions.iter().any(|&s| self.syms.sym(s).name == member))
        });
        // The members every value has, which no class declares here (`Any`'s and `AnyRef`'s).
        let universal = matches!(
            self.name_str(member).as_str(),
            "==" | "!=" | "##" | "asInstanceOf" | "isInstanceOf" | "equals" | "hashCode" | "toString" | "getClass" | "eq" | "ne" | "synchronized" | "notify" | "notifyAll" | "wait"
        );
        if universal || self.find_member(ty, member).is_some() || self.has_member_or_std_extension(ty, member) || extension || self.value_has_type(v, member) {
            return;
        }
        let msg = format!("value {} is not a member of {}", self.name_str(member), self.show(ty));
        self.error(at, msg);
    }

    /// What the wildcard `imp` leaves out: the names its clause's other selectors take.
    fn wildcard_hidden(&mut self, imp: &ast::Import, clause: &[ast::Import]) -> ast::ListRef {
        let names: Vec<Name> = clause.iter().filter(|other| other.span.start == imp.span.start).filter_map(|other| match other.sel {
            ImportSel::Name(n, _) => Some(n),
            _ => None,
        }).collect();
        self.with_loader(|w| {
            let start = w.import_hidden.len() as u32;
            for n in &names {
                w.import_hidden.push(*n);
            }
            ast::ListRef { start, len: names.len() as u32 }
        })
    }

    /// `clause` holds the other selectors of the import, which a wildcard leaves out.
    pub(super) fn resolve_import(&mut self, imp: &ast::Import, clause: &[ast::Import]) -> Option<ResolvedImport> {
        let file = self.env.file;
        let sel = super::unused::Sel { file, span: imp.span };
        let Some(target) = self.resolve_import_path(&imp.path, imp.span) else {
            // A path of the program's that does not resolve binds nothing, and what it could
            // bind is unknown.
            if !self.program_file(file) {
                return None;
            }
            let (name, hidden) = match imp.sel {
                ImportSel::Wildcard | ImportSel::Given => (None, self.wildcard_hidden(imp, clause)),
                ImportSel::Name(_, Some(names::WILDCARD)) => return None,
                ImportSel::Name(n, alias) => (Some(alias.unwrap_or(n)), ast::ListRef::EMPTY),
            };
            return Some(ResolvedImport { name, target: ImportTarget::Unresolved, hidden, bound: None, depth: 0, stmt: imp.span.start, unimports_predef: None, sel: self.sel_ref(sel, None) });
        };
        if self.index.is_some() {
            self.index_import(imp, target);
        }
        if let (true, ImportTarget::PkgAll(p)) = (self.unused.on(), target) {
            self.note_import_package(p);
        }
        let unimports_predef = self.names_predef(&imp.path, target).then_some(imp.span.start);
        let (name, member) = match imp.sel {
            ImportSel::Wildcard | ImportSel::Given => {
                // A `given T` brings the givens that conform to `T` (`ImportInfo.givenBound`), and
                // the names of its type use their imports. The bound is typed with the import
                // (`Namer.importBound`: `typedAheadType(sel.bound)`): a bound that fails is reported
                // and stays an error, which no given matches, never an unbounded import.
                let bound = imp.bound.map(|b| self.resolve_type(b));
                let hidden = self.wildcard_hidden(imp, clause);
                let given_only = matches!(imp.sel, ImportSel::Given);
                let target = match target {
                    ImportTarget::PkgAll(p) if given_only => ImportTarget::PkgGivens(p),
                    ImportTarget::ClassAll(c) if given_only => ImportTarget::ClassGivens(c),
                    ImportTarget::ValueAll(v) if given_only => ImportTarget::ValueGivens(v),
                    t => t,
                };
                let bound = bound.filter(|_| given_only);
                return Some(ResolvedImport { name: None, target, hidden, bound, depth: 0, stmt: imp.span.start, unimports_predef, sel: self.sel_ref(sel, bound) });
            }
            // `import Predef.{x as _}` binds nothing but still takes the root import away.
            ImportSel::Name(_, Some(names::WILDCARD)) if unimports_predef.is_some() => {
                return Some(ResolvedImport { name: None, target: ImportTarget::UnimportPredef, hidden: ast::ListRef::EMPTY, bound: None, depth: 0, stmt: imp.span.start, unimports_predef, sel: self.sel_ref(sel, None) });
            }
            ImportSel::Name(_, Some(names::WILDCARD)) => return None,
            ImportSel::Name(n, alias) => {
                self.check_selector_order(imp, clause, n);
                (alias.unwrap_or(n), n)
            }
        };
        // A name a clause with a wildcard renames is among the wildcard's exclusions, which
        // `ImportInfo.importedImplicits` checks first: it brings no implicit under either name.
        let renamed_beside_wildcard = name != member
            && clause.iter().any(|other| other.span.start == imp.span.start && matches!(other.sel, ImportSel::Wildcard | ImportSel::Given));
        let bound = renamed_beside_wildcard.then_some(NOTHING);
        let target = match target {
            ImportTarget::PkgAll(p) => {
                let pkg = self.syms.pkg(p);
                let exports = pkg.has_exports();
                let known = pkg.entries.contains_key(&member)
                    || (self.demand_std(p, member, crate::stdindex::TYPE | crate::stdindex::TERM) && self.syms.pkg(p).entries.contains_key(&member))
                    || ((self.loaded.is_some() || self.syms.pkg(p).package_object.is_some() || exports) && {
                        // An export table still being built may hold the member.
                        let blocks = self.export_blocks;
                        self.pkg_term(p, member).is_some()
                            || self.pkg_type(p, member).is_some()
                            || self.pkg_exports_of(p).map_or(false, |e| e.extensions.contains_key(&member))
                            || self.export_blocks != blocks
                    });
                if !known {
                    let msg = format!("{} is not a member of the package", self.name_str(member));
                    self.diags.error(file, imp.span, msg);
                }
                ImportTarget::PkgMember(p, member)
            }
            ImportTarget::ClassAll(c) => ImportTarget::ClassMember(c, member),
            ImportTarget::ValueAll(v) => ImportTarget::ValueMember(v, member),
            t => t,
        };
        Some(ResolvedImport { name: Some(name), target, hidden: ast::ListRef::EMPTY, bound, depth: 0, stmt: imp.span.start, unimports_predef, sel: self.sel_ref(sel, None) })
    }

    /// Whether the import's qualifier is `Predef`, one of scalac's root imports: the std's
    /// package `scala` standing for it, or scala-library's object.
    fn names_predef(&self, path: &[Name], target: ImportTarget) -> bool {
        let scala = |n: &Name| self.interner.get(*n) == "scala";
        let root = match path {
            [names::PREDEF] => true,
            [s, names::PREDEF] => scala(s),
            [r, s, names::PREDEF] => *r == names::ROOT && scala(s),
            _ => false,
        };
        root && match target {
            ImportTarget::PkgAll(p) => p == self.b.scala_pkg,
            ImportTarget::ClassAll(c) => self.loaded.as_ref().and_then(|l| l.predef) == Some(c),
            _ => false,
        }
    }

    /// Whether an import in scope takes `Predef`'s root import away, as scalac's `unimported`. A
    /// file's import reaches the classes that start after it.
    #[cold]
    pub(super) fn predef_unimported(&mut self) -> bool {
        let n_local = self.env.imports.len();
        let outermost = self.env.frames.iter().find_map(|f| match f {
            Frame::Class(c) if self.syms.class(*c).file == self.env.file => Some(self.syms.class(*c).span.start),
            _ => None,
        });
        (0..self.import_count()).any(|i| match self.import_at(i).unimports_predef {
            Some(at) => i < n_local || outermost.map_or(true, |start| start > at),
            None => false,
        })
    }

    /// The selectors before `imp` on the same import line: none may name `n` again, and a
    /// wildcard has to be the last of them.
    #[cold]
    fn check_selector_order(&mut self, imp: &ast::Import, clause: &[ast::Import], n: Name) {
        let mut earlier = clause.iter().filter(|other| other.span.start == imp.span.start);
        for other in earlier.by_ref() {
            if std::ptr::eq(other, imp) {
                break;
            }
            let msg = match other.sel {
                ImportSel::Name(m, _) if m == n => format!("{} is imported twice on the same import line", self.name_str(n)),
                ImportSel::Wildcard | ImportSel::Given => "named imports cannot follow wildcard imports".to_string(),
                _ => continue,
            };
            self.error(imp.span, msg);
            return;
        }
    }

    /// The first segment is a name in scope where the import stands, earlier imports included.
    /// A segment may also be a value whose type is a class (`import quotes.reflect.*`, where
    /// `quotes` is a given `Quotes`): the members are then selected on that type, and an object
    /// among them is what the import opens.
    fn resolve_import_path(&mut self, path: &[Name], span: Span) -> Option<ImportTarget> {
        let mut target: Option<ImportTarget> = None;
        let mut value: Option<TypeId> = None;
        // The stable val of a package or an object the path has reached, whose type is read
        // only when a further segment selects on it: the import may be resolved while that
        // type is being named.
        let mut static_val: Option<(SymId, Option<ClassId>, Option<ValueImport>)> = None;
        for &seg in path {
            if let Some(reached) = static_val.take() {
                let ty = self.sig_of(reached.0).ret;
                value = Some(self.zonk(ty));
                // An object nested in a class, selected on the stable value: the import reads
                // its members through that value (`import o.R.*`, `import o.R.S.*`), which
                // becomes an entry of its own.
                if let Some((m, _)) = self.find_member(ty, seg) {
                    if self.inner_object_of_sym(m).is_some() && self.is_accessible(m) {
                        let prev = ValueImport(self.import_values.len() as u32);
                        self.import_values.push(reached);
                        static_val = Some((m, None, Some(prev)));
                        value = None;
                        continue;
                    }
                }
            }
            let next = match (target, value) {
                (_, Some(ty)) => self.import_step_value(ty, seg),
                (None, _) => match self.import_head(seg) {
                    Some(t) => Some(Ok(t)),
                    None => {
                        let head = self.lookup_term(seg);
                        if let Some(TermRef::This(c, s)) = head {
                            if self.illegal_instance_prefix(c, s, span) {
                                return None;
                            }
                        }
                        static_val = head.and_then(|r| self.static_import_val(r));
                        if static_val.is_some() {
                            continue;
                        }
                        self.import_value_head(seg).map(Err)
                    }
                },
                (Some(ImportTarget::PkgAll(p)), _) => match self.import_step_pkg(p, seg) {
                    Some(t) => Some(Ok(t)),
                    None => {
                        static_val = self.pkg_term(p, seg).and_then(|r| self.static_import_val(r));
                        if static_val.is_some() {
                            continue;
                        }
                        None
                    }
                },
                (Some(ImportTarget::ClassAll(c)), _) => match self.import_step_class(c, seg) {
                    Some(t) => Some(Ok(t)),
                    None => {
                        static_val = self.module_term(c, seg).and_then(|r| self.static_import_val(r));
                        if static_val.is_some() {
                            continue;
                        }
                        None
                    }
                },
                _ => None,
            };
            match next {
                Some(Ok(t)) => {
                    target = Some(t);
                    value = None;
                }
                Some(Err(ty)) => value = Some(ty),
                None => {
                    if let (None, Some(TermRef::Local(s))) = (target, self.lookup_term(seg)) {
                        if self.syms.sym(s).kind == SymKind::Var {
                            let ty = self.sig_of(s).ret;
                            let msg = format!("({} : {}) is not a valid import prefix, since it is not an immutable path", self.name_str(seg), self.show(ty));
                            self.error(span, msg);
                            return None;
                        }
                    }
                    let is_value = match target {
                        None => self.lookup_term(seg).is_some(),
                        Some(ImportTarget::PkgAll(p)) => self.pkg_term(p, seg).is_some(),
                        Some(ImportTarget::ClassAll(c)) => self.module_term(c, seg).is_some(),
                        _ => false,
                    };
                    let problem = if is_value { "is not an object or a package" } else { "not found" };
                    let msg = format!("cannot resolve import: {} {}", self.name_str(seg), problem);
                    self.error(span, msg);
                    return None;
                }
            }
        }
        if let Some(value) = static_val {
            let id = self.with_loader(|w| ValueImport(w.import_values.push(value) as u32));
            return Some(ImportTarget::ValueAll(id));
        }
        if let Some(ty) = value {
            if let [seg] = path {
                if self.lookup_term(*seg).and_then(|r| r.sym()).map_or(false, |s| self.is_given_with_clauses(s)) {
                    let msg = format!("{} is not a valid import prefix, since it is not an immutable path", self.show(ty));
                    self.error(span, msg);
                    return None;
                }
            }
            let msg = format!("cannot resolve import: {} is not an object or a package", self.name_str(*path.last().unwrap()));
            self.error(span, msg);
            return None;
        }
        target.or(Some(ImportTarget::PkgAll(ROOT_PKG)))
    }

    /// A val of a package or an object, a local val or parameter, or a val of an enclosing
    /// class's instance, that an import may select members of: a stable path, as scalac requires
    /// of an import's qualifier (`Checking.checkStable`), read on that instance (`C.this.v`).
    fn static_import_val(&mut self, r: TermRef) -> Option<(SymId, Option<ClassId>, Option<ValueImport>)> {
        let (s, module) = match r {
            TermRef::Global(s) => (s, None),
            TermRef::ModuleMember(c, s) => (s, Some(c)),
            // An object's own val read in its body stays the object's limitation.
            TermRef::This(c, s) if self.syms.class(c).kind != ClassKind::Object => (s, Some(c)),
            TermRef::Local(s) if matches!(self.syms.sym(s).kind, SymKind::Val | SymKind::Param | SymKind::Given) => (s, None),
            _ => return None,
        };
        let info = self.syms.sym(s);
        let stable = matches!(info.kind, SymKind::Val | SymKind::Param | SymKind::Given) && !info.by_name && info.mods & ast::mods::MUTABLE == 0;
        (stable && !self.is_given_with_clauses(s)).then_some((s, module, None))
    }

    /// A member of an enclosing class's instance that no import may select on, reported: a var or
    /// a method, which is no stable path (E083), and a lazy val that is not final, which is no
    /// legal one (`Checking.checkLegalImportOrExportPath`, its realizability). The member's type
    /// is shown where reading it types no initialiser in the middle of a completion, which may
    /// be the completion of a class that initialiser makes.
    fn illegal_instance_prefix(&mut self, c: ClassId, s: SymId, span: Span) -> bool {
        let info = self.syms.sym(s);
        let (kind, mods) = (info.kind, info.mods);
        let known = info.sig.is_some() || self.completing == 0 || self.declares_type(s);
        let parameterless = kind == SymKind::Def && (!known || self.sig_of(s).clauses.is_empty());
        let final_member = mods & (mods::FINAL | mods::PRIVATE) != 0 || self.syms.class(c).kind == ClassKind::Object || self.syms.class(c).mods & mods::FINAL != 0;
        let lazy = kind == SymKind::Val && mods & mods::LAZY != 0 && !final_member;
        if kind != SymKind::Var && !parameterless && !lazy {
            return false;
        }
        let shown = match known {
            true => {
                let ty = self.sig_of(s).ret;
                format!(" : {}{}", if parameterless { "=> " } else { "" }, self.show(ty))
            }
            false => String::new(),
        };
        let path = format!("({}.this.{}{})", self.name_str(self.syms.class(c).name), self.name_str(self.syms.sym(s).name), shown);
        let msg = match lazy {
            true => format!("{} is not a legal path since it refers to nonfinal lazy value {}", path, self.name_str(self.syms.sym(s).name)),
            false => format!("{} is not a valid import prefix, since it is not an immutable path", path),
        };
        self.error(span, msg);
        true
    }

    /// Whether the definition of `s` writes its type (a val's or a method's result).
    fn declares_type(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        match info.def {
            Some(d) => match &self.ast(info.file).def(d).kind {
                ast::DefKind::Val { ty, .. } => ty.is_some(),
                ast::DefKind::Fun(f) => f.ret.is_some(),
                _ => false,
            },
            None => info.sig.is_some(),
        }
    }

    /// A given that takes type or term parameters, a method rather than a path (dotty's
    /// `Parsers.givenDef`).
    fn is_given_with_clauses(&mut self, s: SymId) -> bool {
        self.syms.sym(s).kind == SymKind::Given && self.is_method_sym(s)
    }

    /// A value in scope whose type is a class: the type its members are selected on.
    fn import_value_head(&mut self, seg: Name) -> Option<TypeId> {
        let s = self.lookup_term(seg)?.sym()?;
        let info = self.syms.sym(s);
        if !matches!(info.kind, SymKind::Val | SymKind::Param | SymKind::Def | SymKind::Given) {
            return None;
        }
        // Imports are resolved before the bodies of the file are typed: only a declared type
        // can be read here.
        let declared = match info.def {
            Some(d) => match &self.ast(info.file).def(d).kind {
                ast::DefKind::Val { ty, .. } => ty.is_some(),
                ast::DefKind::Fun(f) => f.ret.is_some(),
                ast::DefKind::Given(_) => true,
                _ => false,
            },
            None => info.sig.is_some(),
        };
        if !declared {
            return None;
        }
        let ret = self.sig_of(s).ret;
        let ret = self.zonk(ret);
        // `quotes.reflect` of a `def quotes(using q: Quotes): q.type`: the members of `Quotes`.
        let ret = if self.types.is_path(ret) { self.widen_path(ret) } else { ret };
        matches!(self.types.get(ret), Type::Class(..)).then_some(ret)
    }

    /// The member `seg` of a value's type: an object opens the import, a value carries on.
    fn import_step_value(&mut self, ty: TypeId, seg: Name) -> Option<Result<ImportTarget, TypeId>> {
        let (sym, _) = self.find_member(ty, seg)?;
        let member_ty = self.sig_of(sym).ret;
        let member_ty = self.zonk(member_ty);
        match self.types.get(member_ty) {
            Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object => Some(Ok(ImportTarget::ClassAll(c))),
            Type::Class(..) => Some(Err(member_ty)),
            _ => None,
        }
    }

    fn import_head(&mut self, seg: Name) -> Option<ImportTarget> {
        match self.lookup_term(seg)? {
            TermRef::Package(p) => Some(ImportTarget::PkgAll(p)),
            r => {
                let s = r.sym()?;
                match self.syms.sym(s).kind {
                    SymKind::Object(c) => Some(ImportTarget::ClassAll(c)),
                    _ => self.local_module_of_sym(s).or_else(|| self.inner_object_of_sym(s)).map(ImportTarget::ClassAll),
                }
            }
        }
    }

    pub(super) fn import_step_pkg(&mut self, p: PkgId, seg: Name) -> Option<ImportTarget> {
        if let Some(sub) = self.demand_pkg(p, seg) {
            return Some(ImportTarget::PkgAll(sub));
        }
        match self.pkg_term(p, seg)? {
            // A JDK package entered by the lookup (`import java.time.*` as the first JDK path).
            TermRef::Package(sub) => Some(ImportTarget::PkgAll(sub)),
            r => match self.syms.sym(r.sym()?).kind {
                SymKind::Object(c) => Some(ImportTarget::ClassAll(c)),
                _ => None,
            },
        }
    }

    pub(super) fn import_step_class(&mut self, c: ClassId, seg: Name) -> Option<ImportTarget> {
        let m = self.module_term(c, seg)?.sym()?;
        match self.syms.sym(m).kind {
            SymKind::Object(oc) => Some(ImportTarget::ClassAll(oc)),
            _ => None,
        }
    }

    // ---- name lookup ----

    /// The packages of the file's `package` clauses, innermost first, and `scala`, computed once
    /// per file. A clause `a.b.c` does not open `a.b` and `a`; the empty package is only visible
    /// to files without a clause.
    pub(super) fn pkg_chain(&mut self) -> Arc<[PkgId]> {
        let file = self.env.file.0 as usize;
        if let Some(chain) = self.file_chains.get(file).and_then(|c| c.as_ref()) {
            return chain.clone();
        }
        let ast = self.cur_ast();
        let mut out = Vec::with_capacity(4);
        let mut p = ROOT_PKG;
        let mut seg = 0;
        for &end in &ast.package_clauses {
            while seg < end as usize {
                p = self.syms.pkg(p).entries[&ast.package[seg]].pkg.expect("the namer created the package");
                seg += 1;
            }
            out.push(p);
        }
        out.reverse();
        if out.is_empty() {
            out.push(ROOT_PKG);
        }
        if !out.contains(&self.b.scala_pkg) {
            out.push(self.b.scala_pkg);
        }
        // `java.lang` is imported behind `scala`, as scalac's root imports order them.
        if let Some(lang) = self.java_lang_pkg() {
            if !out.contains(&lang) {
                out.push(lang);
            }
        }
        let chain: Arc<[PkgId]> = out.into();
        // A pseudo file another worker appended has no slot here yet.
        if self.file_chains.len() <= file {
            self.file_chains.resize(file + 1, None);
        }
        self.file_chains[file] = Some(chain.clone());
        chain
    }

    pub(crate) fn class_scope_type(&mut self, c: ClassId, name: Name) -> Option<TypeRef> {
        let info = self.syms.class(c);
        if let Some(&p) = info.tparams.iter().find(|&&p| self.syms.tparam(p).name == name) {
            return Some(TypeRef::Param(p));
        }
        if let Some(&n) = info.nested.get(&name) {
            return Some(TypeRef::Class(n));
        }
        if let Some(&a) = info.type_aliases.get(&name) {
            return Some(self.alias_ref(c, a));
        }
        if info.inherits_types {
            if let Some(r) = self.inherited_type_member(c, name) {
                return Some(r);
            }
        }
        let info = self.syms.class(c);
        if info.kind == ClassKind::Enum {
            if let Some(&n) = info.companion.and_then(|co| self.syms.class(co).info.nested.get(&name)) {
                return Some(TypeRef::Class(n));
            }
        }
        // A type member of the self type is in scope, path-dependent through `c.this`.
        if self.self_type_member_class(c, name).is_some() {
            return Some(TypeRef::Member(c, name));
        }
        self.exports_of(c)?.types.get(&name).copied()
    }

    /// Whether `c` or one of its base types declares the type member `name`.
    pub(super) fn has_type_member(&mut self, c: ClassId, name: Name) -> bool {
        self.complete_class(c);
        self.syms.class(c).base_types.iter().any(|&(b, _)| self.syms.class(b).type_aliases.contains_key(&name))
    }

    /// How the alias `a` of class `c` is named inside `c`: an abstract member through `this`.
    fn alias_ref(&self, c: ClassId, a: AliasId) -> TypeRef {
        let info = &self.syms.aliases[a.idx()];
        if info.is_abstract() {
            TypeRef::Member(c, info.name)
        } else {
            TypeRef::Alias(a)
        }
    }

    /// A type member an ancestor of `c` declares, as seen inside `c`: through `c.this` where
    /// the ancestor's parameters take the arguments `c` extends it with (`type Typeclass[T] =
    /// TypeClass[T]` of `Common[TypeClass[_]]`, inside a `Derivation[TC[_]] extends
    /// Common[TC]`), by the alias itself where nothing is to substitute.
    fn inherited_type_member(&mut self, c: ClassId, name: Name) -> Option<TypeRef> {
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let object = self.syms.class(c).kind == ClassKind::Object;
        for b in bases {
            // An opaque type of a trait is the object's own copy through an object.
            if let Some(&n) = self.syms.class(b).nested.get(&name).filter(|&&n| object && self.is_opaque_path_class(n)) {
                return Some(TypeRef::Class(self.derived_opaque(n, c)));
            }
            // A class nested in a class or trait is inherited with it (`new C` in a subclass),
            // through the subclass's `this` (`Sub.this.C`, an object's own type for an object).
            if let Some(&n) = self.syms.class(b).nested.get(&name).filter(|_| self.syms.class(b).kind != ClassKind::Object) {
                if self.is_inner_class(n) {
                    return Some(TypeRef::Member(c, name));
                }
                return Some(TypeRef::Class(n));
            }
            if let Some(&a) = self.syms.class(b).type_aliases.get(&name) {
                // Through the subclass's `this`, as dotty's `TypeRef(Bar.this, Bla)` sees its info
                // from the prefix: what the alias names of its owner's `this` is the subclass's
                // (`type Bla[X <: A] = X` of `Foo` in `Bar extends Foo` has the bound `Bar.this.A`).
                let through_this = !self.syms.class(b).tparams.is_empty() || (b != c && self.syms.class(b).kind != ClassKind::Object);
                return Some(match self.alias_ref(b, a) {
                    TypeRef::Member(_, n) => TypeRef::Member(c, n),
                    TypeRef::Alias(_) if through_this => TypeRef::Member(c, name),
                    other => other,
                });
            }
        }
        None
    }

    /// An object read from a jar enters its members when it completes. One another worker is
    /// completing is waited for (`complete_class`), not read half entered; one this thread is
    /// completing is read as far as it is.
    #[inline]
    fn complete_loaded_module(&mut self, c: ClassId) {
        let info = self.syms.class(c);
        if info.state() != Completion::Done && info.def.is_none() && self.loaded.is_some() {
            self.complete_class(c);
        }
    }

    /// A type that is nested in the object `c` or exported by it.
    pub fn module_type(&mut self, c: ClassId, name: Name) -> Option<TypeRef> {
        self.complete_loaded_module(c);
        if let Some(&n) = self.syms.class(c).nested.get(&name) {
            return Some(TypeRef::Class(n));
        }
        if let Some(&a) = self.syms.class(c).type_aliases.get(&name) {
            return Some(self.alias_ref(c, a));
        }
        if self.syms.class(c).inherits_types {
            if let Some(r) = self.inherited_type_member(c, name) {
                return Some(r);
            }
        }
        // A std object standing for a JDK class takes the class's nested classes it leaves out
        // (`AbstractMap.SimpleEntry` on the JVM) when one is asked for.
        if self.loaded.is_some() && self.syms.class(c).def.is_some() && self.java_member_miss(c) {
            if let Some(&n) = self.syms.class(c).nested.get(&name) {
                return Some(TypeRef::Class(n));
            }
        }
        self.exports_of(c)?.types.get(&name).copied()
    }

    pub fn lookup_type(&mut self, name: Name) -> Option<TypeRef> {
        self.lookup::<TypeRef>(name, None)
    }

    pub fn lookup_type_at(&mut self, name: Name, span: Span) -> Option<TypeRef> {
        self.lookup::<TypeRef>(name, Some(span))
    }

    #[inline]
    pub fn pkg_type(&mut self, p: PkgId, name: Name) -> Option<TypeRef> {
        let pkg = self.syms.pkg(p);
        let defined =
            pkg.entries.get(&name).and_then(|e| e.class.map(TypeRef::Class).or(e.alias.map(TypeRef::Alias)));
        if let Some(found) = defined {
            return self.visible_pkg_type(p, found);
        }
        if pkg.has_exports() {
            if let Some(&r) = self.pkg_exports_of(p).as_ref().and_then(|e| e.types.get(&name)) {
                return Some(r);
            }
        }
        if p == self.b.scala_pkg {
            if let Some(c) = self.arity_class_named(name) {
                return Some(TypeRef::Class(c));
            }
        }
        // The std file defining the name enters first; then the jars and the JDK are asked.
        if self.demand_std(p, name, crate::stdindex::TYPE) {
            if let Some(e) = self.syms.pkg(p).entries.get(&name) {
                if let Some(found) = e.class.map(TypeRef::Class).or(e.alias.map(TypeRef::Alias)) {
                    return self.visible_pkg_type(p, found);
                }
            }
            if let Some(&r) = self.pkg_exports_of(p).as_ref().and_then(|e| e.types.get(&name)) {
                return Some(r);
            }
        }
        if self.sees_classpath() || self.std_bridges_into(p) {
            if let Some(bound) = self.std_bound_type(p, name) {
                return Some(bound);
            }
            // With several workers, what another worker entered since the entries were read above.
            if self.load_pkg_member(p, name) || self.forked {
                // An object of the jar names no type: the package object's alias of the same name
                // (`zio.Trace`) is the type.
                let loaded = self.syms.pkg(p).entries.get(&name).and_then(|e| e.class.map(TypeRef::Class).or(e.alias.map(TypeRef::Alias)));
                if loaded.is_some() {
                    return loaded;
                }
            }
            // Another module's top-level exports, its package objects entered now.
            if self.syms.pkg(p).has_exports() {
                if let Some(&r) = self.pkg_exports_of(p).as_ref().and_then(|e| e.types.get(&name)) {
                    return Some(r);
                }
            }
        }
        let obj = self.syms.pkg(p).package_object?;
        let found = self.module_type(obj, name)?;
        self.visible_pkg_type(p, found)
    }

    /// The Java packages the std's platform layer bridges into without defining them: a std
    /// file names `java.time.ZoneId` (`TimeZone.toZoneId`), which is the JDK's on the JVM and a
    /// jar's on JavaScript, so those names come from the classpath as a program's would.
    fn std_bridges_into(&self, p: PkgId) -> bool {
        if self.loaded.is_none() || !self.source(self.env.file).is_std {
            return false;
        }
        let mut at = Some(p);
        while let Some(k) = at.filter(|&k| k != ROOT_PKG) {
            let parent = self.syms.pkg(k).parent;
            if self.interner.get(self.syms.pkg(k).name) == "time" && parent.map_or(false, |j| self.interner.get(self.syms.pkg(j).name) == "java" && self.syms.pkg(j).parent == Some(ROOT_PKG)) {
                return true;
            }
            at = parent;
        }
        false
    }

    /// A jar's `private[p]` alias is not seen from outside `p`.
    fn visible_pkg_type(&self, p: PkgId, found: TypeRef) -> Option<TypeRef> {
        if let TypeRef::Alias(a) = found {
            if self.loaded.as_ref().map_or(false, |l| l.qualified_private_aliases.contains_key(&a)) && !self.file_within_pkg(p) {
                return None;
            }
        }
        Some(found)
    }

    /// Whether the current file's package is `p` or one under it.
    fn file_within_pkg(&self, p: PkgId) -> bool {
        let mut pkg = Some(self.file_pkgs[self.env.file.0 as usize]);
        while let Some(k) = pkg {
            if k == p {
                return true;
            }
            pkg = self.syms.pkg(k).parent;
        }
        false
    }

    /// The standard library is written against itself alone; a program's files and the jars
    /// see the classpath, and so does the library signature this worker maps.
    #[inline]
    pub(super) fn sees_classpath(&self) -> bool {
        let Some(loaded) = &self.loaded else { return false };
        let file = self.env.file;
        loaded.scala_library || self.mapping_signature || !self.source(file).is_std || loaded.is_jar(file)
    }

    /// `sees_classpath` for a worker that does not hold the loader's lock, which a library
    /// signature is mapped under alone: what the lock's holder maps is not this worker's.
    pub(super) fn sees_classpath_outside_lock(&self) -> bool {
        let Some(loaded) = &self.loaded else { return false };
        let file = self.env.file;
        loaded.scala_library || !self.source(file).is_std || loaded.is_jar(file)
    }

    #[inline]
    /// A term of a package: a member of its package object, which the loader spills into the
    /// package, is read through the object.
    fn package_term(syms: &Symbols, s: SymId) -> TermRef {
        match syms.sym(s).owner {
            Owner::Class(c) => TermRef::ModuleMember(c, s),
            _ => TermRef::Global(s),
        }
    }

    /// A lone extension method of the package is a term of it where it is called with the
    /// receiver as its first argument (`strip("hello")`, a library body's `Predef.nn[T](x)`);
    /// a bare reference finds it through its receiver, so that it takes no name from a
    /// companion's synthesized members and clashes with no import.
    pub fn pkg_term(&mut self, p: PkgId, name: Name) -> Option<TermRef> {
        let lone_extension = self.extension_call_head || self.in_jar(self.env.file);
        let pkg = self.syms.pkg(p);
        let defined = pkg.entries.get(&name).and_then(|e| {
            let lone_extension = match e.extensions.as_slice() {
                [s] if lone_extension => Some(Self::package_term(&self.syms, *s)),
                _ => None,
            };
            e.term
                .map(|s| Self::package_term(&self.syms, s))
                .or(e.class.map(TermRef::Class))
                .or(e.pkg.map(TermRef::Package))
                .or(lone_extension)
        });
        if let Some(found) = defined {
            // A builtin class whose companion a std file holds (`Array`) enters it; one whose
            // companion the jars hold is looked up there once; a Java class a signature named
            // (`DayOfWeek` from `getDayOfWeek`) is read for its statics when a term names it
            // (`DayOfWeek.MONDAY`).
            if matches!(found, TermRef::Class(c) if self.syms.class(c).kind == ClassKind::Builtin) && self.demand_std(p, name, crate::stdindex::TERM) {
                if let Some(s) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.term) {
                    return Some(Self::package_term(&self.syms, s));
                }
            }
            let placeholder = matches!(found, TermRef::Class(c) if self.is_java_placeholder(c));
            if self.sees_classpath() && (placeholder || matches!(found, TermRef::Class(c) if self.syms.class(c).kind == ClassKind::Builtin)) {
                // With several workers the entries read above may be older than the loader's:
                // another worker may have entered the name since, which the loader then has
                // nothing to enter for, and the entries are read again.
                if self.load_pkg_member(p, name) || self.forked {
                    if let Some(s) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.term) {
                        return Some(if placeholder { Self::package_term(&self.syms, s) } else { TermRef::Global(s) });
                    }
                }
            }
            return Some(found);
        }
        if pkg.has_exports() {
            if let Some(&r) = self.pkg_exports_of(p).as_ref().and_then(|e| e.terms.get(&name)) {
                return Some(r);
            }
        }
        if p == self.b.scala_pkg {
            if let Some(c) = self.arity_class_named(name) {
                return Some(TermRef::Class(c));
            }
        }
        if self.demand_std(p, name, crate::stdindex::TERM) {
            let pkg = self.syms.pkg(p);
            let found = pkg.entries.get(&name).and_then(|e| {
                let lone_extension = match e.extensions.as_slice() {
                    [s] if lone_extension => Some(Self::package_term(&self.syms, *s)),
                    _ => None,
                };
                e.term.map(|s| Self::package_term(&self.syms, s)).or(e.class.map(TermRef::Class)).or(e.pkg.map(TermRef::Package)).or(lone_extension)
            });
            if found.is_some() {
                return found;
            }
            if let Some(&r) = self.pkg_exports_of(p).as_ref().and_then(|e| e.terms.get(&name)) {
                return Some(r);
            }
        }
        if let Some(sub) = self.demand_std_package(p, name) {
            return Some(TermRef::Package(sub));
        }
        if self.sees_classpath() || self.std_bridges_into(p) {
            if let Some(bound) = self.std_bound_term(p, name) {
                return Some(bound);
            }
            // What the jars hold under the name, an extension of a package object entered with
            // it included (`inlined.into`, which `dsl`'s export forwards to); with several
            // workers, what another worker entered since the entries were read above.
            if self.load_pkg_member(p, name) || self.forked {
                if let Some(e) = self.syms.pkg(p).entries.get(&name) {
                    let lone_extension = match e.extensions.as_slice() {
                        [s] if lone_extension => Some(Self::package_term(&self.syms, *s)),
                        _ => None,
                    };
                    return e.term.map(|s| Self::package_term(&self.syms, s)).or(e.class.map(TermRef::Class)).or(e.pkg.map(TermRef::Package)).or(lone_extension);
                }
            }
            // Another module's top-level exports, its package objects entered now.
            if self.syms.pkg(p).has_exports() {
                if let Some(&r) = self.pkg_exports_of(p).as_ref().and_then(|e| e.terms.get(&name)) {
                    return Some(r);
                }
            }
        }
        let obj = self.syms.pkg(p).package_object?;
        self.module_term(obj, name).or_else(|| self.package_object_inherited(obj, name))
    }

    /// A member the package object `obj` inherits, which is its package's as the object's own
    /// are (dotty's `PackageClassDenotation.computeMembersNamed`: the package objects'
    /// non-private members but `Any`'s and `Object`'s), whatever module the object came from.
    fn package_object_inherited(&mut self, obj: ClassId, name: Name) -> Option<TermRef> {
        // A package object whose completion is under way is passed over, as
        // `computeMembersNamed` passes over a package object `isCompleting`: its class's
        // completion, or its table's, whose parents may name a path through its own package
        // (`import p.sub.T; package object p extends T`, the file's imports then being read).
        let completing = self.syms.class(obj).state().get() == Completion::InProgress
            || self.export_stack.iter().any(|f| f.owner == super::exports::ExportOwner::Class(obj));
        if completing {
            return None;
        }
        let ty = self.types.class(obj, &[]);
        if let Some((s, _)) = self.find_member(ty, name) {
            let Owner::Class(owner) = self.syms.sym(s).owner else { return None };
            if owner == obj || owner == self.b.any_ref || self.syms.sym(s).mods & mods::PRIVATE != 0 {
                return None;
            }
            return Some(TermRef::ModuleMember(obj, s));
        }
        // An extension method a base declares, applied on the package object.
        for j in 1..self.syms.class_raw(obj).base_types.len() {
            let b = self.syms.class_raw(obj).base_types[j].0;
            self.complete_class(b);
            let found = self.syms.class_raw(b).extensions.iter().copied().find(|&e| self.syms.sym(e).name == name && self.syms.sym(e).mods & mods::PRIVATE == 0);
            if let Some(e) = found {
                return Some(TermRef::ModuleMember(obj, e));
            }
        }
        None
    }

    pub(super) fn program_source(&self, f: FileId) -> bool {
        !self.in_jar(f) && !self.is_body_file(f) && !self.source(f).is_std
    }

    /// Extension methods called `name` that the package `p` defines, exports or gets from the
    /// parents of its package object.
    #[inline]
    fn pkg_extensions(&mut self, p: PkgId, name: Name, out: &mut Vec<SymId>) {
        self.demand_std(p, name, crate::stdindex::TERM);
        let pkg = self.syms.pkg(p);
        if let Some(e) = pkg.entries.get(&name) {
            out.extend(e.extensions.iter().copied());
        }
        if let Some(obj) = pkg.package_object {
            self.module_extensions(obj, name, out);
            // And those it inherits, the package's as its own are (dotty's
            // `PackageClassDenotation.computeMembersNamed`).
            for i in 1..self.syms.class_raw(obj).base_types.len() {
                let b = self.syms.class_raw(obj).base_types[i].0;
                self.complete_class(b);
                let found: Vec<SymId> = self.syms.class_raw(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name && !out.contains(&s)).collect();
                out.extend(found);
            }
        }
        if !self.syms.pkg(p).has_exports() {
            return;
        }
        if let Some(list) = self.pkg_exports_of(p).as_ref().and_then(|e| e.extensions.get(&name)) {
            out.extend(list.iter().copied());
        }
    }

    /// A term that the object `c` defines, inherits or exports.
    /// A protected member the object `c` inherits, which is in scope in the object's body as in
    /// a class's, though no selection from outside finds it.
    fn inherited_protected(&mut self, c: ClassId, name: Name) -> Option<SymId> {
        self.complete_class(c);
        let self_ty = self.syms.this_type(c);
        let bare = std::mem::replace(&mut self.bare_lookup, true);
        let member = self.find_member(self_ty, name);
        self.bare_lookup = bare;
        member.map(|(s, _)| s).filter(|&s| self.syms.sym(s).mods & crate::ast::mods::PROTECTED != 0)
    }

    pub fn module_term(&mut self, c: ClassId, name: Name) -> Option<TermRef> {
        if let Some(r) = self.predef_std_term(c, name) {
            return Some(r);
        }
        self.complete_loaded_module(c);
        let info = self.syms.class(c);
        if let Some(&s) = info.members.get(&name) {
            return match self.settled(c, name, s) {
                Some(settled) => Some(TermRef::ModuleMember(c, settled)),
                None => self.module_term(c, name),
            };
        }
        if let Some(&n) = info.nested.get(&name) {
            return Some(TermRef::Class(n));
        }
        // A proxy stands where no member of the name does (`NamerOps.addConstructorProxies`'s
        // `memberExists`), inherited ones included.
        if let Some(n) = self.inherited_nested_class(c, name) {
            let module_ty = self.types.class(c, &[]);
            if self.find_member(module_ty, name).is_none() {
                return Some(if self.outer_class(n).is_some() { TermRef::ModuleClass(c, n) } else { TermRef::Class(n) });
            }
        }
        if let Some(r) = self.exports_of(c).and_then(|e| e.terms.get(&name).copied()) {
            return Some(r);
        }
        if self.extension_call_head {
            return self.module_extension_term(c, name);
        }
        None
    }

    /// An extension method of the object `c` named by an application's head (`tag(x)` with `tag`
    /// imported from `c` or `c`'s own): a term of `c` like any method, as dotty's `findRef` binds
    /// it, applied as its selection on `c` is (`Syntax.tag(x)`).
    #[cold]
    #[inline(never)]
    fn module_extension_term(&mut self, c: ClassId, name: Name) -> Option<TermRef> {
        if self.syms.class(c).extensions.is_empty() && self.exports_of(c).is_none() {
            return None;
        }
        let mut found = Vec::new();
        self.module_extensions(c, name, &mut found);
        // One the object declares or inherits; an exported one is another object's.
        let declared = found.into_iter().find(|&e| {
            let Owner::Class(o) = self.syms.sym(e).owner else { return false };
            self.syms.class(c).base_types.iter().any(|&(b, _)| b == o) && self.is_given_accessible(e)
        })?;
        Some(TermRef::ModuleMember(c, declared))
    }

    /// An extension method a class body sees named by an application's head: the class's own or
    /// one its bases declare, applied on the class's instance (`this.tag(x)`).
    #[cold]
    #[inline(never)]
    fn class_extension_term(&mut self, c: ClassId, name: Name) -> Option<TermRef> {
        for j in 0..self.syms.class_raw(c).base_types.len() {
            let b = self.syms.class_raw(c).base_types[j].0;
            self.complete_class(b);
            let found = self.syms.class_raw(b).extensions.iter().copied().find(|&s| self.syms.sym(s).name == name);
            if let Some(e) = found.filter(|&e| self.is_given_accessible(e)) {
                return Some(TermRef::This(c, e));
            }
        }
        None
    }

    /// An extension method of the class of an imported stable value named by an application's
    /// head, applied on the value (`v.tag(x)`).
    #[cold]
    #[inline(never)]
    fn value_extension_term(&mut self, v: ValueImport, name: Name) -> Option<TermRef> {
        let mut found = Vec::new();
        self.value_extensions(v, name, &mut found);
        let e = found.into_iter().find(|&e| self.is_given_accessible(e))?;
        Some(TermRef::ValueMember(v, e))
    }

    pub fn lookup_term(&mut self, name: Name) -> Option<TermRef> {
        self.lookup_term_at_opt(name, None)
    }

    /// Reports a reference that Scala's binding precedence leaves ambiguous at `span`.
    pub fn lookup_term_at(&mut self, name: Name, span: Span) -> Option<TermRef> {
        self.lookup_term_at_opt(name, Some(span))
    }

    fn lookup_term_at_opt(&mut self, name: Name, span: Option<Span>) -> Option<TermRef> {
        if name == names::ROOT {
            return Some(TermRef::Package(ROOT_PKG));
        }
        if let Some(r) = self.lookup::<TermRef>(name, span) {
            if let (true, TermRef::Local(local)) = (self.unused.on(), r) {
                self.mark_alias_read(local);
            }
            return Some(r);
        }
        // Without a jar that defines `scala.Predef`, what it holds in source is the std's package
        // `scala`, as the loader reads a library body's `Predef.x`.
        if name == names::PREDEF && self.program_source(self.env.file) && self.loaded.as_ref().map_or(true, |l| l.predef.is_none()) {
            return Some(TermRef::Package(self.b.scala_pkg));
        }
        TermRef::root_package(self, name)
    }

    /// SLS 2: a definition of the scope beats its imports, a named import beats a wildcard of the
    /// same scope, an inner scope shadows outer bindings of the same or lower precedence, and
    /// an import cannot shadow a definition or a more specific import of an enclosing scope;
    /// package members from other files come last. Imports of a scope stand in `Env::imports`
    /// with the depth of that scope's frame plus one, the file's at depth 0.
    #[inline(always)]
    fn lookup<R: Binding>(&mut self, name: Name, span: Option<Span>) -> Option<R> {
        let n_local = self.env.imports.len();
        let mut next = 0;
        let mut hit: Option<(R, ResolvedImport, usize)> = None;
        // The import a hit uses (`unused.rs`): `hit`'s own, or a named import of an enclosing
        // scope that brings what an inner wildcard brought, as CheckUnused's precedence has it.
        let mut used: Option<ResolvedImport> = None;
        let mut innermost_class = true;
        for i in (0..self.env.frames.len()).rev() {
            if let Some(r) = R::in_frame(self, i, innermost_class, name) {
                if let Some((h, prev, scope)) = hit {
                    if scope != usize::MAX && !R::same(self, h, r) {
                        self.ambiguous_reference(name, span, Bound::Frame(i), prev);
                    }
                } else if R::IS_TERM && next > 0 && span.is_some() {
                    if let Some(imp) = self.inner_import_of_extension(name, next) {
                        self.ambiguous_reference(name, span, Bound::Frame(i), imp);
                    }
                }
                if let (Some(span), true) = (span, R::inherited(self, r, i)) {
                    self.check_inherited_shadowing(name, span, i);
                }
                if self.unused.on() {
                    self.mark_hiding_in_scope(name, next, i);
                }
                return Some(r);
            }
            if matches!(self.env.frames[i], Frame::Class(_)) {
                innermost_class = false;
            }
            while next < n_local {
                let imp = self.import_at(next);
                if (imp.depth as usize) <= i {
                    break;
                }
                next += 1;
                self.import_binding(imp, name, i + 1, &mut hit, &mut used, span);
            }
        }
        let n_imports = self.import_count();
        for i in n_local..n_imports {
            let imp = self.import_at(i);
            self.import_binding(imp, name, 0, &mut hit, &mut used, span);
        }
        let chain = self.pkg_chain();
        let file = self.env.file;
        if let Some(r) = R::in_pkg(self, chain[0], name) {
            if R::file(self, r) == Some(file) {
                if let Some((h, prev, scope)) = hit {
                    if scope != 0 && scope != usize::MAX && !R::same(self, h, r) {
                        self.ambiguous_reference(name, span, Bound::Pkg(chain[0]), prev);
                    }
                }
                return Some(r);
            }
            if hit.is_none() {
                return Some(r);
            }
        }
        if let Some((h, imp, _)) = hit {
            if self.unused.on() && !self.defined_in_enclosing_package(h, name, &chain) {
                self.mark_import(used.unwrap_or(imp));
            }
            return Some(h);
        }
        for &p in &chain[1..] {
            // `scala.Predef` is imported around everything else, as one of scalac's root imports,
            // unless an explicit import of it takes that away; the lean std defines its members
            // in package `scala`.
            let mut found = None;
            if p == self.b.scala_pkg {
                let predef_member = self.b.predef_names.contains(&name);
                let jar_predef = self.loaded.as_ref().map_or(false, |l| l.predef.is_some());
                let unimported = (predef_member || jar_predef) && self.predef_unimported();
                if !unimported {
                    if let Some(r) = self.predef_import().and_then(|imp| R::via_import(self, imp, name)) {
                        return Some(r);
                    }
                }
                if !(unimported && predef_member) {
                    found = R::in_pkg(self, p, name);
                }
            } else {
                found = R::in_pkg(self, p, name);
            }
            // A top-level package hides a member of the same name in `scala` (the package `util`,
            // scala-library's annotation `main`), which only the root import brings, as in scalac;
            // the standard library is kept away from user packages.
            if p == self.b.scala_pkg && !self.source(file).is_std {
                if let Some(r) = R::root_package(self, name) {
                    return Some(r);
                }
            }
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// Whether what an import brought is a definition of a package the file's clauses enclose
    /// its code in, which CheckUnused takes before any import: a package, or a member defined in
    /// this file (one of another file is below an import).
    #[cold]
    fn defined_in_enclosing_package<R: Binding>(&mut self, h: R, name: Name, chain: &[PkgId]) -> bool {
        if R::package(h).is_some_and(|p| self.enclosing_package_member(p)) {
            return true;
        }
        // Only a definition of this file (the `package p:` blocks of a file are units of their
        // own, of the file's path) is asked of the packages, whose lookup of a name they lack
        // reads the class path.
        let file = self.env.file;
        let Some(defined) = R::file(self, h) else { return false };
        if defined != file && self.source(defined).path != self.source(file).path {
            return false;
        }
        chain[1..].iter().any(|&p| R::in_pkg(self, p, name).is_some_and(|r| R::same(self, r, h)))
    }

    /// An import among the first `upto` of the enclosing scopes that brings an extension method
    /// called `name`, which scalac's imports bring as a term like any other method. The std's
    /// extensions stand in for scala-library's implicit classes and given instances, whose
    /// methods no wildcard import brings.
    #[cold]
    fn inner_import_of_extension(&mut self, name: Name, upto: usize) -> Option<ResolvedImport> {
        let mut found = Vec::new();
        for k in 0..upto {
            let imp = self.import_at(k);
            if self.import_hides(imp, name) {
                continue;
            }
            match imp.target {
                ImportTarget::ClassAll(c) if imp.name.is_none() => self.module_extensions(c, name, &mut found),
                ImportTarget::ValueAll(v) if imp.name.is_none() => self.value_extensions(v, name, &mut found),
                ImportTarget::ValueMember(v, orig) if imp.name == Some(name) => self.value_extensions(v, orig, &mut found),
                ImportTarget::ClassMember(c, orig) if imp.name == Some(name) => self.module_extensions(c, orig, &mut found),
                ImportTarget::PkgAll(p) if imp.name.is_none() => self.pkg_extensions(p, name, &mut found),
                ImportTarget::PkgMember(p, orig) if imp.name == Some(name) => self.pkg_extensions(p, orig, &mut found),
                _ => {}
            }
            found.retain(|&e| !self.source(self.syms.sym(e).file).is_std);
            if !found.is_empty() {
                return Some(imp);
            }
        }
        None
    }

    /// The wildcard import of `scala.Predef` that every file has when a classpath supplies it.
    #[inline]
    pub(super) fn predef_import(&self) -> Option<ResolvedImport> {
        let predef = self.loaded.as_ref()?.predef?;
        Some(ResolvedImport { name: None, target: ImportTarget::ClassAll(predef), hidden: ast::ListRef::EMPTY, bound: None, depth: 0, stmt: u32::MAX, unimports_predef: None, sel: super::unused::SelRef::NONE })
    }

    /// Inlined into `lookup` whatever its size: the per-import loop of every lookup, where an
    /// outlined call cost a check 0.4% of its instructions.
    #[inline(always)]
    fn import_binding<R: Binding>(
        &mut self,
        imp: ResolvedImport,
        name: Name,
        scope: usize,
        hit: &mut Option<(R, ResolvedImport, usize)>,
        used: &mut Option<ResolvedImport>,
        span: Option<Span>,
    ) {
        if self.import_hides(imp, name) {
            if self.unused.on() {
                self.mark_hiding(imp, name);
            }
            return;
        }
        let Some(r) = R::via_import(self, imp, name) else { return };
        let Some((h, prev, prev_scope)) = *hit else {
            *hit = Some((r, imp, scope));
            return;
        };
        if prev_scope == usize::MAX || R::same(self, h, r) {
            // CheckUnused takes a named import over a wildcard whatever their nesting: the
            // first named import that brings what the hit is uses it.
            if self.unused.on() && prev_scope != usize::MAX && imp.name.is_some() && prev.name.is_none() && used.is_none() {
                *used = Some(imp);
            }
            return;
        }
        let (wild, prev_wild) = (imp.name.is_none(), prev.name.is_none());
        let conflict = if wild { prev_wild && prev_scope == scope } else { prev_wild || prev_scope == scope };
        if conflict && span.is_some() {
            self.ambiguous_reference(name, span, Bound::Import(imp), prev);
            // An ambiguity between imports uses both.
            self.mark_import(imp);
            *hit = Some((h, prev, usize::MAX));
        }
    }

    /// Scala 3's rule that a member inherited in an inner class does not shadow a definition of
    /// an enclosing scope: such a reference is ambiguous, as scalac reports it.
    #[cold]
    fn check_inherited_shadowing(&mut self, name: Name, span: Span, inner: usize) {
        for j in (0..inner).rev() {
            let defined = match &self.env.frames[j] {
                Frame::Locals { names, .. } => names.iter().any(|&(n, _)| n == name).then(|| "an enclosing scope".to_string()),
                Frame::Class(k) => {
                    let k = *k;
                    let own = self.syms.class(k).members.get(&name).map_or(false, |&s| self.syms.sym(s).owner == Owner::Class(k))
                        || self.syms.class(k).nested.contains_key(&name);
                    own.then(|| self.class_description(k))
                }
            };
            if let Some(place) = defined {
                let Frame::Class(k) = self.env.frames[inner] else { return };
                let msg = format!(
                    "Reference to {} is ambiguous.\nIt is both defined in {}\nand inherited subsequently in {}",
                    self.name_str(name),
                    place,
                    self.class_description(k)
                );
                self.error(span, msg);
                return;
            }
        }
    }

    /// `outer` was found after the import `inner`, which therefore cannot shadow it. One report
    /// per lookup: a hit whose scope is `usize::MAX` has been reported.
    #[cold]
    fn ambiguous_reference(&mut self, name: Name, span: Option<Span>, outer: Bound, inner: ResolvedImport) {
        let Some(span) = span else { return };
        let outer = match outer {
            Bound::Frame(i) => match self.env.frames[i] {
                Frame::Class(c) => format!("defined in {}", self.class_description(c)),
                Frame::Locals { .. } => "defined in an enclosing scope".to_string(),
            },
            Bound::Pkg(p) => format!("defined in package {}", self.pkg_description(p)),
            Bound::Import(imp) => format!("{} by {}", self.import_kind(imp), self.import_text(imp)),
        };
        let msg = format!(
            "reference to {} is ambiguous: it is both {} and {} subsequently by {}",
            self.name_str(name),
            outer,
            self.import_kind(inner),
            self.import_text(inner)
        );
        self.error(span, msg);
    }

    fn import_kind(&self, imp: ResolvedImport) -> &'static str {
        if imp.name.is_some() { "imported by name" } else { "imported" }
    }

    fn import_text(&self, imp: ResolvedImport) -> String {
        let (prefix, member) = match imp.target {
            ImportTarget::PkgAll(p) => (self.pkg_description(p), "*".to_string()),
            ImportTarget::PkgGivens(p) => (self.pkg_description(p), "given".to_string()),
            ImportTarget::PkgMember(p, n) => (self.pkg_description(p), self.name_str(n)),
            ImportTarget::ClassAll(c) => (self.class_path(c), "*".to_string()),
            ImportTarget::ClassGivens(c) => (self.class_path(c), "given".to_string()),
            ImportTarget::ClassMember(c, n) => (self.class_path(c), self.name_str(n)),
            ImportTarget::ValueAll(v) => (self.sym_path(self.import_values[v.0 as usize].0), "*".to_string()),
            ImportTarget::ValueMember(v, n) => (self.sym_path(self.import_values[v.0 as usize].0), self.name_str(n)),
            ImportTarget::ValueGivens(v) => (self.sym_path(self.import_values[v.0 as usize].0), "given".to_string()),
            ImportTarget::UnimportPredef => ("Predef".to_string(), "_".to_string()),
            ImportTarget::Unresolved => ("<unresolved>".to_string(), "_".to_string()),
        };
        format!("import {}.{}", prefix, member)
    }

    pub(super) fn pkg_description(&self, p: PkgId) -> String {
        if p == ROOT_PKG {
            return "<empty>".to_string();
        }
        let info = self.syms.pkg(p);
        match info.parent.filter(|&parent| parent != ROOT_PKG) {
            Some(parent) => format!("{}.{}", self.pkg_description(parent), self.name_str(info.name)),
            None => self.name_str(info.name),
        }
    }

    pub fn class_path(&self, c: ClassId) -> String {
        let info = self.syms.class(c);
        match info.owner {
            Owner::Class(o) => format!("{}.{}", self.class_path(o), self.name_str(info.name)),
            Owner::Package(p) if p != ROOT_PKG => format!("{}.{}", self.pkg_description(p), self.name_str(info.name)),
            _ => self.name_str(info.name),
        }
    }

    fn is_given_ref(&self, r: TermRef) -> bool {
        r.sym().map_or(false, |s| self.syms.sym(s).kind == SymKind::Given)
    }

    /// Extension methods called `name` that the class `c` defines, inherits (objects) or exports.
    pub fn module_extensions(&mut self, c: ClassId, name: Name, out: &mut Vec<SymId>) {
        self.complete_loaded_module(c);
        let info = self.syms.class(c);
        out.extend(info.extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name));
        if let Some(exports) = self.exports_of(c) {
            if let Some(list) = exports.extensions.get(&name) {
                out.extend(list.iter().copied());
            }
        }
    }

    /// Extension methods called `name` of the class of an imported stable value and its bases.
    fn value_extensions(&mut self, v: ValueImport, name: Name, out: &mut Vec<SymId>) {
        let Some(c) = self.import_value_class(v) else { return };
        // The bases' classes and members alone: the records' ids, read raw (a metadata-only read).
        for i in 0..self.syms.class_raw(c).base_types.len() {
            let b = self.syms.class_raw(c).base_types[i].0;
            self.complete_class(b);
            out.extend(self.syms.class_raw(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name));
        }
    }

    /// The class of an imported stable value's type.
    pub(super) fn import_value_class(&mut self, v: ValueImport) -> Option<ClassId> {
        let ty = self.import_value_ret(v);
        let ty = self.zonk(ty);
        let c = self.class_of(ty)?;
        self.complete_class(c);
        Some(c)
    }

    /// Extension methods named `name` that are lexically visible.
    pub fn lexical_extensions(&mut self, name: Name) -> Vec<SymId> {
        self.lexical_extensions_counted(name).0
    }

    /// `lexical_extensions`, with how many imports brought one: two or more may be alternatives
    /// of dotty's `findRef` (`import_alternatives`).
    pub(super) fn lexical_extensions_counted(&mut self, name: Name) -> (Vec<SymId>, u32) {
        let mut out = Vec::new();
        let mut imports = 0;
        for i in (0..self.env.frames.len()).rev() {
            self.frame_extensions(i, name, &mut out);
        }
        let n_imports = self.import_count();
        for i in 0..n_imports {
            let imp = self.import_at(i);
            if self.import_hides(imp, name) {
                continue;
            }
            let before = out.len();
            self.import_extensions(imp, name, &mut out);
            imports += (out.len() > before) as u32;
        }
        for &p in self.pkg_chain().iter() {
            self.pkg_extensions(p, name, &mut out);
        }
        if let Some(predef) = self.loaded.as_ref().and_then(|l| l.predef) {
            self.module_extensions(predef, name, &mut out);
        }
        // A private or protected extension is a candidate where its member would be reachable.
        out.retain(|&s| self.is_given_accessible(s));
        (out, imports)
    }

    /// The extension methods called `name` that the frame `i` defines. Inlined into the walks,
    /// which ask every frame of every extension selection.
    #[inline(always)]
    fn frame_extensions(&mut self, i: usize, name: Name, out: &mut Vec<SymId>) {
        match &self.env.frames[i] {
            Frame::Class(c) => {
                let c = *c;
                self.module_extensions(c, name, out);
                // A class body sees the extensions its bases declare (scalatest's
                // `shouldBe`, an extension of the `Matchers` trait a suite mixes in).
                if self.syms.class(c).kind != ClassKind::Object {
                    for j in 1..self.syms.class_raw(c).base_types.len() {
                        let b = self.syms.class_raw(c).base_types[j].0;
                        self.complete_class(b);
                        out.extend(self.syms.class_raw(b).extensions.iter().copied().filter(|&s| self.syms.sym(s).name == name));
                    }
                }
            }
            Frame::Locals { names, .. } => {
                for &(n, s) in names.iter() {
                    if n == name && self.syms.sym(s).is_extension {
                        out.push(s);
                    }
                }
            }
        }
    }

    /// dotty's `findRef` for the extension method a selection names (`tryExtension`, which asks
    /// for the flag `ExtensionMethod`): the extensions called `name` of the nearest scope that
    /// defines or imports one, in `lookup`'s order (a frame's definitions, then the imports of
    /// its scope, outwards; the file's package's in the file, the file's imports, the enclosing
    /// packages; `Predef`), with how many
    /// imports of that scope brought one (`import_alternatives`). An outer scope's are hidden
    /// whatever their receivers: where the nearest's do not apply, the receiver's implicit scope
    /// is searched (`inferView`), not the enclosing scopes. The extensions of package `scala`
    /// follow apart: the std's stand there for the members of builtin types and for
    /// scala-library's implicit classes of `Predef`, which that search finds whatever the lexical
    /// scope holds.
    pub(super) fn nearest_extensions_counted(&mut self, name: Name) -> (Vec<SymId>, u32) {
        let mut out = Vec::new();
        let mut imports = 0;
        let n_local = self.env.imports.len();
        let mut next = 0;
        let scala_pkg = self.b.scala_pkg;
        // Whether the walk read package `scala`'s, which then are not added apart.
        let mut scala_read = false;
        'nearest: {
            // Without imports inside the file's scopes the frames alone, the common case: the walk is
            // every missing member's selection.
            let interleaved = if n_local == 0 { 0 } else { self.env.frames.len() };
            for i in (interleaved..self.env.frames.len()).rev() {
                self.frame_extensions(i, name, &mut out);
                if !out.is_empty() {
                    out.retain(|&s| self.is_given_accessible(s));
                    if !out.is_empty() {
                        break 'nearest;
                    }
                }
            }
            for i in (0..interleaved).rev() {
                self.frame_extensions(i, name, &mut out);
                if !out.is_empty() {
                    out.retain(|&s| self.is_given_accessible(s));
                    if !out.is_empty() {
                        break 'nearest;
                    }
                }
                while next < n_local && self.import_at(next).depth as usize > i {
                    let depth = self.import_at(next).depth;
                    let named;
                    (imports, named) = self.imports_extensions_at(next, n_local, depth, name, &mut out);
                    if !out.is_empty() {
                        // An import cannot shadow what an enclosing scope binds with a higher
                        // precedence, a definition or, over a wildcard, a named import, unless it
                        // is the same reference (`checkNewOrShadowed`): the reference is ambiguous
                        // and the lexical attempt fails.
                        if self.extension_defined_outside(i, name, &out) || (!named && self.named_import_outside(depth, name, &out)) {
                            out.clear();
                            imports = 0;
                        }
                        break 'nearest;
                    }
                    while next < n_local && self.import_at(next).depth == depth {
                        next += 1;
                    }
                }
            }
            // The file's own extensions of its package are definitions of the scope its imports
            // are in, which they beat; those of the package's other files come after the imports.
            let chain = self.pkg_chain();
            let file = self.env.file;
            // The file's package's, `out` being empty here, set apart only where there are any.
            self.pkg_extensions(chain[0], name, &mut out);
            let mut first = Vec::new();
            if !out.is_empty() {
                first = std::mem::take(&mut out);
                if first.iter().any(|&s| self.syms.sym(s).file == file) {
                    out.extend(first.iter().copied().filter(|&s| self.syms.sym(s).file == file && self.is_given_accessible(s)));
                    if !out.is_empty() {
                        break 'nearest;
                    }
                }
            }
            let n_imports = self.import_count();
            if n_imports > n_local {
                imports = self.imports_extensions_at(n_local, n_imports, 0, name, &mut out).0;
                if !out.is_empty() {
                    break 'nearest;
                }
            }
            scala_read = chain[0] == scala_pkg;
            if !first.is_empty() {
                out.append(&mut first);
                out.retain(|&s| self.is_given_accessible(s));
                if !out.is_empty() {
                    break 'nearest;
                }
            }
            for k in 1..chain.len() {
                self.pkg_extensions(chain[k], name, &mut out);
                if !out.is_empty() {
                    out.retain(|&s| self.is_given_accessible(s));
                    if !out.is_empty() {
                        scala_read = chain[..=k].contains(&scala_pkg);
                        break 'nearest;
                    }
                }
            }
            // The chain holds `scala` (`pkg_chain`), read in full.
            scala_read = true;
            if let Some(predef) = self.loaded.as_ref().and_then(|l| l.predef) {
                self.module_extensions(predef, name, &mut out);
                out.retain(|&s| self.is_given_accessible(s));
            }
        }
        if !scala_read {
            self.add_scala_extensions(name, &mut out);
        }
        (out, imports)
    }

    /// Package `scala`'s extensions called `name` after the nearest scope's, apart.
    #[cold]
    #[inline(never)]
    fn add_scala_extensions(&mut self, name: Name, out: &mut Vec<SymId>) {
        let mut std = Vec::new();
        self.pkg_extensions(self.b.scala_pkg, name, &mut std);
        for s in std {
            if !out.contains(&s) && self.is_given_accessible(s) {
                out.push(s);
            }
        }
    }

    /// Whether a frame outside the frame `inner`, or the file's own package, defines an extension
    /// method called `name` other than those of `found`.
    #[cold]
    fn extension_defined_outside(&mut self, inner: usize, name: Name, found: &[SymId]) -> bool {
        let mut outer = Vec::new();
        for j in (0..inner).rev() {
            self.frame_extensions(j, name, &mut outer);
            if outer.iter().any(|&s| !found.contains(&s) && self.is_given_accessible(s)) {
                return true;
            }
        }
        let chain = self.pkg_chain();
        let file = self.env.file;
        self.pkg_extensions(chain[0], name, &mut outer);
        outer.iter().any(|&s| self.syms.sym(s).file == file && !found.contains(&s) && self.is_given_accessible(s))
    }

    /// Whether the nearest named import of a scope outside the one at `depth`, the file's among
    /// them, that brings an extension method called `name` brings another than those of `found`.
    /// One further out is shadowed by that one, a binding of the same precedence nearer
    /// (`isPossibleImport`).
    #[cold]
    fn named_import_outside(&mut self, depth: u32, name: Name, found: &[SymId]) -> bool {
        let mut outer = Vec::new();
        for k in 0..self.import_count() {
            let imp = self.import_at(k);
            if imp.depth >= depth || imp.name != Some(name) {
                continue;
            }
            outer.clear();
            self.import_extensions(imp, name, &mut outer);
            outer.retain(|&s| self.is_given_accessible(s));
            if !outer.is_empty() {
                return outer.iter().any(|s| !found.contains(s));
            }
        }
        false
    }

    /// The extensions called `name` that the imports from position `from` up to `to` at depth
    /// `depth` bring, how many of them brought one, and whether those are named imports: the
    /// named imports' where one brings one, which beat the wildcards of their scope
    /// (`checkImportAlternatives`).
    fn imports_extensions_at(&mut self, from: usize, to: usize, depth: u32, name: Name, out: &mut Vec<SymId>) -> (u32, bool) {
        let mut imports = 0;
        let mut named = false;
        for k in from..to {
            let imp = self.import_at(k);
            if imp.depth != depth {
                break;
            }
            if self.import_hides(imp, name) || (named && imp.name.is_none()) {
                continue;
            }
            let before = out.len();
            self.import_extensions(imp, name, out);
            let mut j = before;
            while j < out.len() {
                if self.is_given_accessible(out[j]) {
                    j += 1;
                } else {
                    out.remove(j);
                }
            }
            if out.len() > before {
                if imp.name.is_some() && !named {
                    // A scope lists its named imports first: what came before was named too.
                    named = true;
                }
                imports += 1;
            }
        }
        (imports, named)
    }

    /// The extension methods called `name` that the import `imp` brings.
    #[inline]
    fn import_extensions(&mut self, imp: ResolvedImport, name: Name, out: &mut Vec<SymId>) {
        match (imp.target, imp.name) {
            (ImportTarget::PkgMember(p, orig), Some(n)) if n == name => self.pkg_extensions(p, orig, out),
            (ImportTarget::PkgAll(p), None) => self.pkg_extensions(p, name, out),
            (ImportTarget::ClassMember(c, orig), Some(n)) if n == name => self.module_extensions(c, orig, out),
            (ImportTarget::ClassAll(c), None) => self.module_extensions(c, name, out),
            (ImportTarget::ValueAll(v), None) => self.value_extensions(v, name, out),
            (ImportTarget::ValueMember(v, orig), Some(n)) if n == name => self.value_extensions(v, orig, out),
            _ => {}
        }
    }

    /// dotty's `findRef` for an extension method with its `altImports` (Typer.scala 213, 291 to
    /// 330, `checkImportAlternatives`): the extensions called `name` that the innermost import
    /// bringing one brings, and those of every other import of its scope that brings one, each
    /// import's an alternative of its own (the overloads of one object, an extension two objects
    /// inherit from one trait two references): a named import's in the place of the wildcards',
    /// the wildcards' beside each other. Fewer than two where no two imports of that scope bring
    /// the name.
    pub(super) fn import_alternatives(&mut self, name: Name) -> Vec<ImportAlternative> {
        let mut groups: Vec<ImportAlternative> = Vec::new();
        let mut first: Option<(u32, bool)> = None;
        for i in 0..self.import_count() {
            let imp = self.import_at(i);
            if first.is_some_and(|(depth, _)| depth != imp.depth) {
                break;
            }
            if self.import_hides(imp, name) {
                continue;
            }
            let mut syms = Vec::new();
            self.import_extensions(imp, name, &mut syms);
            syms.retain(|&s| self.is_given_accessible(s));
            if syms.is_empty() {
                continue;
            }
            // The object the import reaches, through a value that is it (`val alias: A.type = A`):
            // two imports of one object are one reference, as `addAltImport`'s `isSameRef` has it.
            let module = match imp.target {
                ImportTarget::ClassAll(c) | ImportTarget::ClassMember(c, _) => Some(c),
                ImportTarget::ValueAll(v) | ImportTarget::ValueMember(v, _) => {
                    self.import_value_class(v).filter(|&c| self.syms.class(c).kind == ClassKind::Object)
                }
                _ => None,
            };
            let found = ImportAlternative { module, syms };
            let named = imp.name.is_some();
            match first {
                Some((_, true)) if !named => {}
                Some((depth, false)) if named => {
                    groups.clear();
                    groups.push(found);
                    first = Some((depth, true));
                }
                Some(_) if groups.contains(&found) => {}
                Some(_) => groups.push(found),
                None => {
                    first = Some((imp.depth, named));
                    groups.push(found);
                }
            }
        }
        groups
    }

    // ---- type expressions ----
    pub fn class_arity(&self, c: ClassId) -> usize {
        if let Some(info) = self.syms.class_done(c) {
            return info.tparams.len();
        }
        let info = self.syms.class(c);
        if info.def.is_none() {
            return info.tparams.len();
        }
        let def = self.ast(info.file).def(info.def.unwrap());
        match &def.kind {
            DefKind::Class(cls) => {
                if cls.kind == ast::ClassKind::EnumCase {
                    if let Owner::Class(companion) = info.owner {
                        let e = self.syms.class(companion).companion;
                        if let Some(copied) = e.and_then(|e| self.copied_enum_tparams(cls, e)) {
                            return copied.len();
                        }
                    }
                }
                cls.tparams.len()
            }
            DefKind::Given(g) => g.tparams.len(),
            DefKind::TypeAlias { tparams, .. } => tparams.len(),
            _ => 0,
        }
    }

    /// What an alias whose right-hand side is a match type is used as: its application by
    /// name, as a lambda over its parameters when it has any.
    pub(super) fn match_alias_ctor(&mut self, a: AliasId) -> TypeId {
        if self.syms.alias(a).state() == Completion::NotStarted {
            self.complete_alias(a);
        }
        let tparams = self.syms.aliases[a.idx()].tparams.clone();
        if tparams.is_empty() {
            return self.types.mk(Type::Alias(a, EMPTY_LIST));
        }
        let ps: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
        let l = self.types.list(&ps);
        let body = self.types.mk(Type::Alias(a, l));
        self.types.mk(Type::Lambda(l, body))
    }

    /// An alias's right-hand side was resolved where the alias is declared; inside an inline
    /// expansion the type parameters it names are the expansion's (`type Typeclass[T] =
    /// TypeClass[T]` of a `Common[TypeClass[_]]`, expanded for a `Common[TC]`).
    fn in_expansion(&mut self, t: TypeId) -> TypeId {
        if self.inline.tparams.is_empty() || self.completing > 0 || t == ERROR {
            t
        } else {
            self.subst_inline_tparams(t)
        }
    }

    pub(super) fn type_ref_to_type(&mut self, r: TypeRef, span: Span, applied: bool) -> TypeId {
        match r {
            TypeRef::Param(p) => match self.inline_tparam(p) {
                Some(t) => t,
                None => {
                    self.types.param(p)
                }
            },
            TypeRef::Class(c) => {
                if self.class_arity(c) == 0 {
                    self.types.class(c, &[])
                } else {
                    let _ = (span, applied);
                    self.types.mk(Type::Ctor(c))
                }
            }
            TypeRef::Alias(a) => {
                if self.is_match_alias(a) {
                    return self.match_alias_ctor(a);
                }
                self.complete_alias(a);
                let info = &self.syms.aliases[a.idx()];
                if info.is_abstract() {
                    return self.types.mk(Type::Decl(a));
                }
                if self.alias_by_name(a) {
                    return self.match_alias_ctor(a);
                }
                let info = &self.syms.aliases[a.idx()];
                let t = if info.tparams.is_empty() || info.rhs == ERROR {
                    info.rhs
                } else {
                    let (tparams, rhs) = (info.tparams.clone(), info.rhs);
                    let ps: Vec<TypeId> = tparams.iter().map(|&p| self.types.param(p)).collect();
                    let l = self.types.list(&ps);
                    self.types.mk(Type::Lambda(l, rhs))
                };
                self.in_expansion(t)
            }
            TypeRef::Member(c, name) => {
                let prefix = match self.inline_this_of(c) {
                    Some(t) => t,
                    None => self.this_prefix(c),
                };
                // A member of the self type is kept by name through `c.this`: an alias of it
                // holds only on a receiver that is the self type, which `c.this` is and a
                // `bus: C` outside the trait is not.
                if self.syms.class(c).declared_self.is_some() && matches!(self.types.get(prefix), Type::This(k) if k == c) && !self.has_type_member(c, name) && self.self_type_member_class(c, name).is_some() {
                    return self.types.mk(Type::Member(prefix, name));
                }
                let t = self.member_type(prefix, name).unwrap_or(ERROR);
                self.in_expansion(t)
            }
            TypeRef::ValueMember(v, name) => {
                let prefix = self.import_value_type(v);
                match self.member_type(prefix, name) {
                    Some(t) => t,
                    None => match self.nested_class_through(prefix, name) {
                        Some(c) => self.type_ref_to_type(TypeRef::Class(c), span, applied),
                        None => ERROR,
                    },
                }
            }
        }
    }

    /// The term member `name` of the value an import opens: named, through its wildcard (no
    /// given) or its given selector (only givens).
    #[cold]
    #[inline(never)]
    fn value_import_term(&mut self, imp: ResolvedImport, name: Name) -> Option<TermRef> {
        match imp.target {
            ImportTarget::ValueMember(v, orig) if imp.name == Some(name) => {
                let ty = self.import_value_ret(v);
                // A member the import cannot reach is none it brings, named or not.
                let Some((m, _)) = self.find_member(ty, orig) else {
                    let ext = if self.extension_call_head { self.value_extension_term(v, orig) } else { None };
                    return ext.or_else(|| self.value_class_term(v, orig));
                };
                self.is_accessible(m).then_some(TermRef::ValueMember(v, m))
            }
            ImportTarget::ValueAll(v) if imp.name.is_none() => {
                let ty = self.import_value_ret(v);
                let ty = self.zonk(ty);
                if matches!(self.types.get(ty), Type::Var(_)) || self.import_hides(imp, name) {
                    return None;
                }
                let Some((m, _)) = self.find_member(ty, name) else {
                    let ext = if self.extension_call_head { self.value_extension_term(v, name) } else { None };
                    return ext.or_else(|| self.value_class_term(v, name));
                };
                (!self.syms.is_given(m) && self.is_accessible(m)).then_some(TermRef::ValueMember(v, m))
            }
            ImportTarget::ValueGivens(v) => {
                let ty = self.import_value_ret(v);
                let ty = self.zonk(ty);
                let (m, _) = self.find_member(ty, name)?;
                (self.syms.is_given(m) && self.is_accessible(m)).then_some(TermRef::ValueMember(v, m))
            }
            _ => None,
        }
    }

    /// The constructor proxy of the class `name` nested in the class of the value an import
    /// opens, where no term of the name is a member: dotty's proxy is a member of that class.
    fn value_class_term(&mut self, v: ValueImport, name: Name) -> Option<TermRef> {
        let prefix = self.import_value_prefix(v);
        let c = self.nested_class_through(prefix, name).filter(|&c| self.needs_constructor_proxy(c))?;
        Some(TermRef::ValueClass(v, c))
    }

    /// The type member `name` of the value an import opens, named or through its wildcard.
    #[cold]
    #[inline(never)]
    fn value_import_type(&mut self, imp: ResolvedImport, name: Name) -> Option<TypeRef> {
        let (v, member) = match imp.target {
            ImportTarget::ValueMember(v, orig) if imp.name == Some(name) => (v, orig),
            ImportTarget::ValueAll(v) if imp.name.is_none() && !self.import_hides(imp, name) => (v, name),
            _ => return None,
        };
        self.value_has_type(v, member).then_some(TypeRef::ValueMember(v, member))
    }

    /// Whether the value an import opens has the type member or nested class `name`.
    fn value_has_type(&mut self, v: ValueImport, name: Name) -> bool {
        let prefix = self.import_value_prefix(v);
        let mark = self.snapshot();
        let found = self.member_type(prefix, name).is_some() || self.nested_class_through(prefix, name).is_some();
        self.rollback(mark);
        found
    }

    pub(super) fn resolve_type_path(&mut self, id: TyExprId) -> Option<TermRef> {
        let ast = self.cur_ast();
        match ast.ty(id) {
            TyExpr::Name(n) => self.lookup_term(n),
            TyExpr::Select(q, n) => match self.resolve_type_path(q)? {
                TermRef::Package(p) => self.pkg_term(p, n),
                TermRef::Global(s) | TermRef::ModuleMember(_, s) | TermRef::This(_, s) => {
                    match self.syms.sym(s).kind {
                        SymKind::Object(c) => self.module_term(c, n),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether the written type's head names an alias of the std's `scala.scalajs.js` package
    /// (`js.Function2`), a JavaScript type a cast leaves untested, before the alias is expanded into
    /// scala's function type.
    pub(super) fn written_js_alias(&mut self, id: TyExprId) -> bool {
        let ast = self.cur_ast();
        let head = match ast.ty(id) {
            TyExpr::Apply(f, _) => f,
            _ => id,
        };
        let found = match ast.ty(head) {
            TyExpr::Name(n) => self.lookup_type(n),
            TyExpr::Select(q, n) => self.lookup_type_in_path(q, n),
            _ => None,
        };
        let Some(TypeRef::Alias(a)) = found else { return false };
        let Some(js) = self.scalajs_pkg() else { return false };
        matches!(self.syms.alias(a).owner, Owner::Package(p) if p == js)
    }

    /// The type `n` selected from the package or object that the path `q` names.
    pub fn lookup_type_in_path(&mut self, q: TyExprId, n: Name) -> Option<TypeRef> {
        match self.resolve_type_path(q)? {
            TermRef::Package(p) => self.pkg_type(p, n),
            TermRef::Global(s) | TermRef::ModuleMember(_, s) | TermRef::This(_, s) => {
                match self.syms.sym(s).kind {
                    SymKind::Object(c) => self.module_type(c, n),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// `p.T` for a path `p` that is a val or a parameter, `this.T` and `C.this.T`: the type
    /// member through the path. None where the path names a package or an object, whose
    /// members `lookup_type_in_path` finds.
    fn dependent_type(&mut self, q: TyExprId, n: Name, span: Span) -> Option<TypeId> {
        let ast = self.cur_ast();
        // In an inline expansion `this` and `C.this` are the receiver.
        let prefix = match ast.ty(q) {
            TyExpr::Name(names::THIS) => {
                let c = self.this_class()?;
                self.complete_class(c);
                self.inline_this_of(c).unwrap_or_else(|| self.this_prefix(c))
            }
            TyExpr::Select(k, names::THIS) => {
                let TyExpr::Name(class_name) = ast.ty(k) else { return None };
                let c = self.qualified_this_class(class_name)?;
                self.complete_class(c);
                self.inline_this_of(c).unwrap_or_else(|| self.this_prefix(c))
            }
            _ => match {
                let r = self.resolve_type_path(q);
                if r.map_or(false, |r| matches!(r, TermRef::Package(_)) || r.sym().map_or(false, |s| matches!(self.syms.sym(s).kind, SymKind::Object(_)))) {
                    return None;
                }
                self.type_path(q)?
            } {
                Ok(p) => p,
                Err(s) => match self.def_prefix(q, s) {
                    Some(p) => p,
                    None => {
                    let ty = self.sig_of(s).ret;
                    let what = if self.syms.sym(s).kind == SymKind::Def { "=> " } else { "" };
                    let msg = format!(
                        "({} : {}{}) is not a valid type prefix, since it is not an immutable path",
                        self.name_str(self.syms.sym(s).name),
                        what,
                        self.show(ty)
                    );
                    self.error(ast.ty_spans[q.idx()], msg);
                    return Some(ERROR);
                    }
                },
            },
        };
        let outer_use = self.cycle_at_use.replace(false);
        let found = self.member_type(prefix, n);
        if std::mem::replace(&mut self.cycle_at_use, outer_use) == Some(true) {
            self.report_recursion_at_use(span, prefix, n);
            return Some(ERROR);
        }
        match found {
            Some(t) => {
                if self.deps.is_some() {
                    self.deps_member_alias(prefix, n);
                }
                Some(t)
            }
            None => match self.nested_class_through(prefix, n) {
                Some(c) => Some(self.type_ref_to_type(TypeRef::Class(c), span, false)),
                None => {
                    let msg = format!("type {} is not a member of {}", self.name_str(n), self.show(prefix));
                    self.error(span, msg);
                    Some(ERROR)
                }
            },
        }
    }

    pub fn resolve_type(&mut self, id: TyExprId) -> TypeId {
        let t = self.resolve_type_ctor(id);
        if let Type::Ctor(c) = self.types.get(t) {
            // A bare generic class outside of a constructor position gets unknown arguments.
            let n = self.class_arity(c);
            let args = vec![ERROR; n];
            return self.types.class(c, &args);
        }
        let t = self.normalize(t);
        self.seen_from_receiver_paths(t)
    }

    /// A type a signature declares (a result's, a value's): as `resolve_type` gives it, but
    /// for a cons tuple whose tail is written as a name of the empty tuple
    /// (`Int *: String *: EmptyTuple`), which stays the `*:` chain it is spelled as, as a
    /// library's signature does when read, where elsewhere it is the tuple class of its
    /// elements. scalac erases such a chain to `Product` and pickles it as written.
    pub fn resolve_declared_type(&mut self, id: TyExprId) -> TypeId {
        let t = self.resolve_type(id);
        self.spelled_cons(id, t).unwrap_or(t)
    }

    /// The `*:` chain `id` spells where `t`, its resolved type, is the tuple class of the chain's
    /// heads alone, an `Array` of one included.
    pub(super) fn spelled_cons(&mut self, id: TyExprId, t: TypeId) -> Option<TypeId> {
        let ast = self.cur_ast();
        let cons_of = |e: TyExprId| match ast.ty(e) {
            TyExpr::Apply(f, args) if args.len == 2 && matches!(ast.ty(f), TyExpr::Name(names::CONS_TUPLE)) => Some(ast.ty_list(args)[1]),
            _ => None,
        };
        let Type::Class(c, targs) = self.types.get(t) else { return None };
        if c == self.b.array {
            let TyExpr::Apply(_, args) = ast.ty(id) else { return None };
            let (&[elem_expr], &[elem]) = (ast.ty_list(args), self.types.items(targs)) else { return None };
            let spelled = self.spelled_cons(elem_expr, elem)?;
            return Some(self.types.class(c, &[spelled]));
        }
        let mut tail = cons_of(id)?;
        let mut heads = 1;
        while let Some(next) = cons_of(tail) {
            tail = next;
            heads += 1;
        }
        let elems = self.types.items(targs).to_vec();
        if !matches!(ast.ty(tail), TyExpr::Name(_) | TyExpr::Select(..)) || !self.is_tuple_class(c) || elems.len() != heads {
            return None;
        }
        let (cons, empty) = (self.b.cons_tuple?, self.b.empty_tuple?);
        let mut chain = self.types.class(empty, &[]);
        for &e in elems.iter().rev() {
            chain = self.types.class(cons, &[e, chain]);
        }
        Some(chain)
    }

    /// Like `resolve_type` but leaves an unapplied generic class as a type constructor.
    pub fn resolve_type_ctor(&mut self, id: TyExprId) -> TypeId {
        let ast = self.cur_ast();
        let span = ast.ty_spans[id.idx()];
        match ast.ty(id) {
            TyExpr::Name(n) => {
                if let Some(r) = self.lookup_type_at(n, span) {
                    if self.index.is_some() {
                        self.index_type(id, r);
                    }
                    if self.deps.is_some() {
                        self.deps_type_ref(r);
                        if n == names::ANY_REF {
                            self.deps_written_name("AnyRef");
                        }
                    }
                    return self.type_ref_to_type(r, span, false);
                }
                match n {
                    names::ANY => ANY,
                    // The compiler-defined `Matchable` stands as `Any` here, as the loader reads a
                    // jar's: every value but an `Any` is one.
                    names::MATCHABLE => ANY,
                    names::NOTHING => NOTHING,
                    _ => {
                        let msg = format!("type {} not found", self.name_str(n));
                        self.not_found_error(n, span, msg);
                        ERROR
                    }
                }
            }
            TyExpr::Select(q, n) => {
                if let Some(t) = self.dependent_type(q, n, span) {
                    // `C.this.T` in an inline body is the receiver's `T`, as a library body's
                    // resolved types are.
                    return self.seen_from_receiver_paths(t);
                }
                let found = self.lookup_type_in_path(q, n);
                if let (Some(r), true) = (found, self.index.is_some()) {
                    self.index_type(id, r);
                }
                if let (Some(r), true) = (found, self.deps.is_some()) {
                    self.deps_type_ref(r);
                    self.deps_type_path(q);
                    self.deps_export_path(q, n);
                }
                match found {
                    Some(r) => self.type_ref_to_type(r, span, false),
                    // `scala.Any` from a package whose own `Any` shadows it (`scala.scalajs.js`).
                    None if matches!(n, names::ANY | names::NOTHING)
                        && matches!(self.resolve_type_path(q), Some(TermRef::Package(p)) if p == self.b.scala_pkg) =>
                    {
                        if n == names::ANY { ANY } else { NOTHING }
                    }
                    None => {
                        let msg = format!("type {} not found", self.name_str(n));
                        self.error(span, msg);
                        ERROR
                    }
                }
            }
            TyExpr::Apply(f, args) => {
                let mut operands = None;
                if matches!(ast.ty(f), TyExpr::Name(names::CONS_TUPLE)) && args.len == 2 {
                    match self.tuple_cons_type(args) {
                        Ok(t) => return t,
                        Err(resolved) => operands = Some(resolved),
                    }
                }
                let ctor = self.resolve_type_ctor(f);
                if let Type::Ctor(c) = self.types.get(ctor) {
                    if self.types.is_op_class(c) && Some(c) != self.b.cons_tuple {
                        return self.type_level_op(c, args);
                    }
                }
                let params: Vec<TParamId> = match self.types.get(ctor) {
                    Type::Ctor(c) => {
                        self.complete_class_tparams(c);
                        self.syms.class(c).tparams.clone()
                    }
                    Type::Lambda(l, _) => self
                        .types
                        .items(l)
                        .iter()
                        .filter_map(|&p| match self.types.get(p) {
                            Type::Param(p) => Some(p),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let arg_ids: Vec<TyExprId> = ast.ty_list(args).to_vec();
                let mut resolved = Vec::with_capacity(arg_ids.len());
                for (i, &a) in arg_ids.iter().enumerate() {
                    if let Some(ops) = operands {
                        resolved.push(ops[i]);
                        continue;
                    }
                    if matches!(ast.ty(a), TyExpr::Wildcard) || self.unbound_type_var(a) {
                        resolved.push(match params.get(i).map_or(0, |&p| self.syms.tparam(p).variance) {
                            1 => ANY,
                            -1 => NOTHING,
                            _ => WILD,
                        });
                    } else if let TyExpr::BoundedWildcard(lo, hi) = ast.ty(a) {
                        resolved.push(match params.get(i).map_or(0, |&p| self.syms.tparam(p).variance) {
                            1 => self.resolve_type(hi),
                            -1 => self.resolve_type(lo),
                            _ => {
                                let (lo, hi) = (self.resolve_type(lo), self.resolve_type(hi));
                                self.types.bounded_wild(lo, hi)
                            }
                        });
                    } else {
                        resolved.push(self.resolve_type_ctor(a));
                    }
                }
                if ctor == ERROR {
                    return ERROR;
                }
                if let Type::Ctor(c) = self.types.get(ctor) {
                    let expected = self.class_arity(c);
                    if expected != resolved.len() {
                        let msg = format!(
                            "{} takes {} type argument(s), {} given",
                            self.name_str(self.syms.class(c).name),
                            expected,
                            resolved.len()
                        );
                        // A broken argument may be one too many.
                        let args = resolved.clone();
                        self.error_unless_unknown(span, msg, &args);
                        return ERROR;
                    }
                }
                let t = self.types.apply_ctor(ctor, &resolved);
                if t == ERROR {
                    self.error(span, "this type does not take type arguments");
                } else if params.len() == resolved.len() {
                    let through = match self.types.get(ctor) {
                        Type::Lambda(_, body) => match self.types.get(body) {
                            Type::Nested(p, class) => self.types.named_class(class).map(|c| (p, c)),
                            _ => None,
                        },
                        _ => None,
                    };
                    self.check_type_args_through(&params, &resolved, &arg_ids, through);
                }
                t
            }
            TyExpr::Fun(params, ret) | TyExpr::CtxFun(params, ret) if ast.fun_param_names(id).is_some() => {
                let names = ast.fun_param_names(id).unwrap_or_default();
                let ctx = matches!(ast.ty(id), TyExpr::CtxFun(..));
                self.resolve_named_fun(params, names, ret, ctx, span)
            }
            TyExpr::Fun(params, ret) => {
                let ps: Vec<TypeId> = ast
                    .ty_list(params)
                    .to_vec()
                    .iter()
                    .map(|&p| match ast.ty(p) {
                        TyExpr::ByName(inner) => {
                            let t = self.resolve_type(inner);
                            self.by_name_type(t)
                        }
                        _ => self.resolve_type(p),
                    })
                    .collect();
                let r = self.resolve_type(ret);
                self.fun_type(&ps, r)
            }
            TyExpr::CtxFun(params, ret) => {
                let ps: Vec<TypeId> =
                    ast.ty_list(params).to_vec().iter().map(|&p| self.resolve_type(p)).collect();
                let r = self.resolve_type(ret);
                self.ctx_fun_type(&ps, r)
            }
            TyExpr::Tuple(elems) => {
                // `(=> A)` that no `=>` makes a function's parameters (a self type `self: (=> A) =>`).
                if let Some(&p) = ast.ty_list(elems).iter().find(|&&p| matches!(ast.ty(p), TyExpr::ByName(_))) {
                    let span = ast.ty_spans[p.idx()];
                    self.error(span, "a by-name type can only be the type of a parameter");
                    return ERROR;
                }
                let es: Vec<TypeId> =
                    ast.ty_list(elems).to_vec().iter().map(|&p| self.resolve_type(p)).collect();
                if es.is_empty() {
                    return self.b.t_unit;
                }
                self.tuple_type(&es)
            }
            TyExpr::NamedTuple(names, elems) => {
                let names: Vec<Name> = ast.name_lists[names.range()].to_vec();
                let es: Vec<TypeId> =
                    ast.ty_list(elems).to_vec().iter().map(|&p| self.resolve_type(p)).collect();
                self.named_tuple_type(&names, &es, span)
            }
            TyExpr::Union(a, b) => {
                if self.deps.is_some() {
                    self.deps_written_name("|");
                }
                let (x, y) = (self.resolve_type(a), self.resolve_type(b));
                self.types.union(x, y)
            }
            TyExpr::Inter(a, b) => {
                if self.deps.is_some() {
                    self.deps_written_name("&");
                }
                let (x, y) = (self.resolve_type(a), self.resolve_type(b));
                self.types.inter(x, y)
            }
            TyExpr::ByName(t) | TyExpr::Repeated(t) | TyExpr::Unchecked(t) => self.resolve_type(t),
            TyExpr::UncheckedVariance(t) => {
                let inner = self.resolve_type(t);
                self.unchecked_variance.insert((self.env.file, id), inner);
                if self.forked && !self.unchecked_variance.to_shared {
                    self.pending_shared.push(super::PendingShared::Variance((self.env.file, id), inner));
                }
                inner
            }
            TyExpr::Wildcard => ANY,
            TyExpr::BoundedWildcard(_, hi) => self.resolve_type(hi),
            TyExpr::TypeVar(n) => match self.case_binders.iter().rev().find(|(m, _)| *m == n) {
                Some(&(_, b)) => b,
                None => match self.inline.pat_vars.iter().rev().find(|(m, _)| *m == n) {
                    Some(&(_, v)) => v,
                    None => ANY,
                },
            },
            // dotty's `Desugar.makePolyFunctionType`: the `apply` of the function type's
            // parameters, `x$1`.. where it names none, checked as any `PolyFunction`'s
            // (`Checking.checkPolyFunctionType`).
            TyExpr::PolyFun(params, fun) => {
                let span = ast.ty_spans[id.idx()];
                let names: Vec<Name> = ast.name_lists[params.range()].to_vec();
                let bounds = ast.lambda_bounds.iter().find(|(l, _)| *l == id).map(|&(_, bl)| bl);
                let (ids, ps) = self.poly_params(&names, bounds);
                let f = self.resolve_type(fun);
                let f = self.poly_function_apply(f, span);
                self.env.frames.pop();
                let _ = ids;
                if let Some(sig) = self.named_apply_sig(f) {
                    self.check_poly_function_apply(&sig, span);
                }
                self.poly_type(&ps, f)
            }
            TyExpr::Match(scrut, cases) => self.match_type_expr(scrut, cases),
            TyExpr::MatchCase(..) => ERROR,
            TyExpr::Resolved(t) => {
                // A type the loader mapped into a library body, the base's: in the reader's view.
                let t = self.types.import(t);
                self.subst_inline_tparams(t)
            }
            TyExpr::Lambda(params, body) => {
                let names: Vec<Name> = ast.name_lists[params.range()].to_vec();
                let mut frame_tparams = Vec::new();
                let mut ps = Vec::new();
                let mut ids = Vec::new();
                for n in names {
                    let p = self.syms.new_tparam(n, 0);
                    frame_tparams.push((n, p));
                    ps.push(self.types.param(p));
                    ids.push(p);
                }
                self.env.frames.push(Frame::Locals {
                    names: Vec::new(),
                    tparams: frame_tparams,
                    givens: Vec::new(),
                    classes: Vec::new(),
                    aliases: Vec::new(),
                    owner: self.sites.owners.len() as u32,
                });
                // The bounds of the parameters (`[x <: Node] =>> SynEv[x]`), which may name
                // each other, so they are resolved with the parameters in scope.
                if let Some(&(_, bl)) = ast.lambda_bounds.iter().find(|(l, _)| *l == id) {
                    self.resolve_lambda_bounds(bl, &ids);
                }
                let b = self.resolve_type(body);
                self.env.frames.pop();
                let l = self.types.list(&ps);
                self.types.mk(Type::Lambda(l, b))
            }
            TyExpr::Singleton(path) => self.singleton_type(path, span),
            TyExpr::Project(q, n) => {
                // A type constructor has no members (dotty's E007 against `?{ T: ? }`).
                let prefix = self.resolve_type_ctor(q);
                if let Type::Ctor(c) = self.types.get(prefix) {
                    let msg = format!("type mismatch: found {}, required ?{{ {}: ? }}", self.name_str(self.syms.class(c).name), self.name_str(n));
                    self.error(self.cur_ast().ty_spans[q.idx()], msg);
                    return ERROR;
                }
                let prefix = self.normalize(prefix);
                let prefix = self.seen_from_receiver_paths(prefix);
                if prefix == ERROR {
                    return ERROR;
                }
                // `p.type#T` is `p.T`, dotty's `TypeRef(p.type, T)`.
                if self.types.is_path(prefix) {
                    if let Some(t) = self.member_type(prefix, n) {
                        return t;
                    }
                }
                let prefix = self.dealias(prefix);
                // A refined type projects its refinement's member (`Zippable.Out[A, B, C]#Out`).
                if !matches!(self.types.get(prefix), Type::Class(..) | Type::Refined(..) | Type::Nested(..)) {
                    let msg = format!("{} is not a legal path\nsince it is not a concrete type", self.show(prefix));
                    self.error(span, msg);
                    return ERROR;
                }
                match self.member_type(prefix, n) {
                    Some(t) => t,
                    None => {
                        let msg = format!("type {} is not a member of {}", self.name_str(n), self.show(prefix));
                        self.error(span, msg);
                        ERROR
                    }
                }
            }
            TyExpr::Refined(parent, members) => self.refined_type(parent, members, span),
            TyExpr::Lit(e) => self.resolve_literal_type(e, span),
            TyExpr::Error => {
                self.error_nodes += 1;
                ERROR
            }
        }
    }

    /// `T { type A = X; def m: R }`: the parent with each member as a refinement, a term
    /// member as a symbol carrying its signature.
    #[cold]
    fn refined_type(&mut self, parent: TyExprId, members: ListRef, span: Span) -> TypeId {
        let head = self.resolve_type_ctor(parent);
        if let Type::Ctor(c) = self.types.get(head) {
            let msg = format!("Missing type parameter for {}", self.name_str(self.syms.class(c).name));
            self.error(self.cur_ast().ty_spans[parent.idx()], msg);
            return ERROR;
        }
        let mut t = self.resolve_type(parent);
        if t == ERROR {
            return ERROR;
        }
        let parent_ty = t;
        let ast = self.cur_ast();
        let file = self.env.file;
        let mut terms: Vec<(Name, SymId, Span)> = Vec::new();
        for &d in ast.def_list(members) {
            let def = ast.def(d);
            let r = match &def.kind {
                DefKind::TypeAlias { tparams, rhs, lower, upper } => {
                    let ids = self.make_tparams(tparams);
                    self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
                    self.resolve_tparam_bounds(tparams, &ids);
                    let over = |t: &mut Self, ty: TypeId| {
                        if ids.is_empty() {
                            return ty;
                        }
                        let ps: Vec<TypeId> = ids.iter().map(|&(_, p)| t.types.param(p)).collect();
                        let l = t.types.list(&ps);
                        t.types.mk(Type::Lambda(l, ty))
                    };
                    let r = match rhs {
                        Some(rhs) => {
                            let ty = self.resolve_type_ctor(*rhs);
                            Refinement::Alias(def.name, over(self, ty))
                        }
                        None => {
                            let lo = lower.map_or(NOTHING, |b| self.resolve_type(b));
                            let hi = upper.map_or(ANY, |b| self.resolve_type(b));
                            let (lo, hi) = (if lo == NOTHING { lo } else { over(self, lo) }, if hi == ANY { hi } else { over(self, hi) });
                            Refinement::Bounds(def.name, lo, hi)
                        }
                    };
                    self.env.frames.pop();
                    r
                }
                DefKind::Val { .. } | DefKind::Fun(_) => {
                    let kind = match &def.kind {
                        DefKind::Fun(_) => SymKind::Def,
                        _ => SymKind::Val,
                    };
                    let sym = self.syms.new_sym(def.name, kind, def.mods | mods::ABSTRACT, Owner::Local, file, Some(d), def.span);
                    self.def_syms.insert(file.0 as usize, d, sym);
                    terms.push((def.name, sym, def.span));
                    let sig = self.sig_arc(sym);
                    if kind == SymKind::Val {
                        let r = self.types.refine(Refinement::Val(def.name, sym, sig.ret));
                        t = self.types.mk(Type::Refined(t, r));
                        continue;
                    }
                    Refinement::Term(def.name, sym, self.sig_types(&sig))
                }
                _ => {
                    self.error(def.span, "refinement cannot have a body");
                    continue;
                }
            };
            let _ = span;
            let r = self.types.refine(r);
            t = self.types.mk(Type::Refined(t, r));
        }
        self.check_refinement_members(parent_ty, &terms);
        t
    }

    /// The term members of a refinement of `parent` as dotty's `Typer.typedRefinedTypeTree`
    /// checks the class it makes of them: a generic member refines a member of the parent (but
    /// an `apply` where the parent is `PolyFunction` itself, the parent's type symbol), and no
    /// member's name is overloaded there, the refinement's members with the parent's that none
    /// of them matches (`refinement_matches`); and, where the parent derives from
    /// `PolyFunction` (through an intersection or an alias too), as
    /// `Checking.checkPolyFunctionType` checks its method refinements: a `def`, and a `var`'s
    /// getter and setter (`Desugar.refinedTypeToClass`), a `val` left alone.
    fn check_refinement_members(&mut self, parent: TypeId, terms: &[(Name, SymId, Span)]) {
        let class = self.class_of(parent);
        let poly_function_class = self.std_class("PolyFunction");
        let exact_poly_function = matches!(self.types.get(parent), Type::Class(c, _) if Some(c) == poly_function_class);
        let derives_poly_function = poly_function_class.map_or(false, |c| self.base_type(parent, c).is_some());
        let alternatives = |w: &mut Self, name: Name| -> Vec<(SymId, TypeId)> {
            match w.find_member(parent, name) {
                Some((m, owner)) => match w.syms.alternatives(m) {
                    Some(alts) => alts.to_vec().into_iter().map(|a| (a, owner)).collect(),
                    None => vec![(m, owner)],
                },
                None => Vec::new(),
            }
        };
        for &(name, sym, span) in terms {
            let sig = self.sig_arc(sym);
            let theirs = alternatives(self, name);
            let refines = theirs.iter().any(|&(m, owner)| self.refinement_matches(m, owner, &sig));
            if !sig.tparams.is_empty() && !refines && !(exact_poly_function && name == names::APPLY) {
                let parent_name = match class {
                    Some(c) if c == self.b.any_ref => "type AnyRef".to_string(),
                    Some(c) => format!("{} {}", if self.syms.class(c).kind == ClassKind::Trait { "trait" } else { "class" }, self.name_str(self.syms.class(c).name)),
                    None => format!("type {}", self.show(parent)),
                };
                let msg = format!("Polymorphic refinement method {} without matching type in parent {} is no longer allowed", self.name_str(name), parent_name);
                self.error(span, msg);
                continue;
            }
            let own: Vec<SymId> = terms.iter().filter(|&&(n, ..)| n == name).map(|&(_, s, _)| s).collect();
            let mut unmatched = 0;
            for &(m, owner) in &theirs {
                let mut matched = false;
                for &s in &own {
                    let other = self.sig_arc(s);
                    if self.refinement_matches(m, owner, &other) {
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    unmatched += 1;
                }
            }
            if own.len() + unmatched > 1 {
                self.error(span, "Refinements cannot introduce overloaded definitions");
                continue;
            }
            // A `var` is its getter, then its setter, whose message the getter's hides at their
            // one position (`UniqueMessagePositions`).
            let info = self.syms.sym(sym);
            let (def, var) = (info.kind == SymKind::Def, info.kind != SymKind::Def && info.mods & mods::MUTABLE != 0);
            if derives_poly_function && (def || var) {
                if name != names::APPLY {
                    self.error(span, "PolyFunction only supports apply method refinements");
                } else {
                    let getter = MethodSig::value(sig.ret);
                    self.check_poly_function_apply(if var { &getter } else { &sig }, span);
                }
            }
        }
    }

    /// A `PolyFunction`'s `apply` takes exactly one parameter list, after its type parameters,
    /// of no by-name or repeated parameter (dotty's `Definitions.isValidPolyFunctionInfo`, which
    /// `Checking.checkPolyFunctionType` checks of the refinements a source writes or desugars).
    pub(super) fn check_poly_function_apply(&mut self, sig: &MethodSig, span: Span) {
        let by_name = |w: &Self, p: &ParamSig| p.by_name || matches!(w.types.get(p.ty), Type::Class(c, _) if Some(c) == w.b.by_name);
        let valid = matches!(sig.clauses.as_slice(), [clause] if !clause.params.iter().any(|p| p.repeated || by_name(self, p)));
        if !valid {
            self.error(span, "Implementation restriction: PolyFunction apply must have exactly one parameter list and optionally type arguments. No by-name nor varags are allowed.");
        }
    }

    /// The type parameters of a polymorphic function type or literal, pushed as a frame the
    /// caller pops, with the bounds `bounds` (`Ast::lambda_bounds`, `Ast::poly_lambda_bounds`)
    /// resolved in their scope, as dotty's `typeParamClause` types each `TypeBoundsTree`.
    pub(super) fn poly_params(&mut self, names: &[Name], bounds: Option<ListRef>) -> (Vec<TParamId>, Vec<TypeId>) {
        let mut frame_tparams = Vec::with_capacity(names.len());
        let mut ids = Vec::with_capacity(names.len());
        let mut ps = Vec::with_capacity(names.len());
        for &n in names {
            let p = self.syms.new_tparam(n, 0);
            frame_tparams.push((n, p));
            ps.push(self.types.param(p));
            ids.push(p);
        }
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: frame_tparams, givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
        if let Some(bl) = bounds {
            self.resolve_lambda_bounds(bl, &ids);
        }
        (ids, ps)
    }

    /// Each parameter's lower and upper bound of the list `bl` (`Ast::lambda_bounds`'s layout),
    /// resolved where the parameters `ids` are in scope.
    fn resolve_lambda_bounds(&mut self, bl: ListRef, ids: &[TParamId]) {
        let bounds: Vec<TyExprId> = self.cur_ast().ty_lists[bl.range()].to_vec();
        for (&id, pair) in ids.iter().zip(bounds.chunks(2)) {
            if let [lower, upper] = *pair {
                if lower != crate::ast::NO_BOUND {
                    let l = self.resolve_type(lower);
                    self.syms.tparams[id.idx()].lower = l;
                }
                if upper != crate::ast::NO_BOUND {
                    let u = self.resolve_type(upper);
                    self.syms.tparams[id.idx()].upper = u;
                }
            }
        }
    }

    /// A type variable of a pattern that no `inline match` is binding reads as a wildcard.
    fn unbound_type_var(&self, a: TyExprId) -> bool {
        match self.cur_ast().ty(a) {
            TyExpr::TypeVar(n) => self.unbound_type_var_name(n),
            _ => false,
        }
    }

    pub(super) fn unbound_type_var_name(&self, n: Name) -> bool {
        !self.inline.pat_vars.iter().any(|(m, _)| *m == n) && !self.case_binders.iter().any(|(m, _)| *m == n)
    }

    /// Each argument of a type application has the kind of its parameter, by arity, and lies
    /// within the parameter's bounds; a wildcard is not checked. The bounds are checked once
    /// every class is complete, as scalac does after typing, since a parent clause names the
    /// class being completed (`class Bar extends Foo[Bar]` for `Foo[F <: Foo[F]]`). The bounds
    /// of a class nested in a class applied through the prefix `through` gives
    /// (`Ext.this.Visitor[T]`) are seen from that prefix, as dotty sees a `TypeRef`'s class.
    fn check_type_args_through(&mut self, params: &[TParamId], args: &[TypeId], arg_asts: &[TyExprId], through: Option<(TypeId, ClassId)>) {
        let ast = self.cur_ast();
        let subst: Subst = params.iter().copied().zip(args.iter().copied()).collect();
        let seen = |t: &mut Self, b: TypeId| match through {
            Some((p, c)) if t.types.has_paths(b) => t.outer_seen_from(b, p, c),
            _ => b,
        };
        for (i, (&p, &a)) in params.iter().zip(args).enumerate() {
            if matches!(ast.ty(arg_asts[i]), TyExpr::Wildcard | TyExpr::BoundedWildcard(..)) || self.unbound_type_var(arg_asts[i]) || self.types.contains_error(a) {
                continue;
            }
            let span = ast.ty_spans[arg_asts[i].idx()];
            let arity = self.param_arity(p);
            // A library body's kinds were checked by scalac, against scala-library's parameters
            // (`TypeRepr.of[T <: AnyKind]`), which the std may declare of one kind only.
            if a != NOTHING && self.type_arity(a) != arity && !self.is_body_file(self.env.file) {
                let msg = self.kind_mismatch(a, p, arity);
                self.error(span, msg);
                continue;
            }
            let (upper, lower) = (self.syms.tparam(p).upper, self.syms.tparam(p).lower);
            // A higher-kinded argument is held to its bound at the end as well (`is_sub` applies
            // both to the bound's parameters), when the bounds of a parameter given as the
            // argument (`Attr[E]` in a class over `E[x] <: Event[x]`) are resolved.
            if arity > 0 {
                if upper != ANY && !self.is_body_file(self.env.file) {
                    let u = self.types.subst(upper, &subst);
                    let (file, transparent) = (self.env.file, self.transparent.clone());
                    let of_body = self.diags.of_bodies;
                    self.note_rare(super::state::Rare::DeferredBounds, self.deferred_bounds.len());
                    self.deferred_bounds.push(DeferredBound { arg: a, upper: Some(u), lower: None, span, file, transparent, of_body, definition: None });
                }
                continue;
            }
            if upper == ANY && lower == NOTHING {
                continue;
            }
            let upper = (upper != ANY).then(|| {
                let u = self.types.subst(upper, &subst);
                seen(self, u)
            });
            let lower = (lower != NOTHING).then(|| {
                let l = self.types.subst(lower, &subst);
                seen(self, l)
            });
            let file = self.env.file;
            let transparent = self.transparent.clone();
            // A library body was checked by scalac; a later build of a watch session would
            // otherwise meet its checks.
            if !self.is_body_file(file) {
                let of_body = self.diags.of_bodies;
                self.note_rare(super::state::Rare::DeferredBounds, self.deferred_bounds.len());
                self.deferred_bounds.push(DeferredBound { arg: a, upper, lower, span, file, transparent, of_body, definition: None });
            }
        }
    }

    pub(super) fn kind_mismatch(&mut self, arg: TypeId, p: TParamId, arity: usize) -> String {
        let shape = if arity == 0 { String::new() } else { format!("[{}]", vec!["_"; arity].join(", ")) };
        format!(
            "type argument {} does not have the same kind as its parameter {}{}",
            self.show(arg),
            self.name_str(self.syms.tparam(p).name),
            shape
        )
    }

    #[cold]
    pub(super) fn check_deferred_bounds(&mut self) {
        let outer = self.diags.of_bodies;
        for check in std::mem::take(&mut self.deferred_bounds) {
            self.transparent = check.transparent;
            self.diags.of_bodies = check.of_body;
            if let Some(u) = check.upper {
                if !self.is_sub(check.arg, u) {
                    let msg = format!("type argument {} does not conform to upper bound {}", self.show(check.arg), self.show(u));
                    self.diags.error(check.file, check.span, msg);
                }
            }
            if let Some(l) = check.lower {
                if !self.is_sub(l, check.arg) {
                    let msg = format!("type argument {} does not conform to lower bound {}", self.show(check.arg), self.show(l));
                    self.diags.error(check.file, check.span, msg);
                }
            }
        }
        self.diags.of_bodies = outer;
        self.transparent.clear();
    }

    /// How many type arguments a type takes: a type constructor, a type lambda or a
    /// higher-kinded parameter take some, a type none.
    pub(super) fn type_arity(&self, t: TypeId) -> usize {
        match self.types.get(t) {
            Type::Ctor(c) => self.class_arity(c),
            Type::Lambda(l, _) => self.types.items(l).len(),
            // A curried alias applied to its own parameters stands for its lambda.
            Type::Alias(a, args) => {
                if self.types.items(args).len() != self.syms.aliases[a.idx()].tparams.len() {
                    return 0;
                }
                match self.types.get(self.alias_body(a)) {
                    Type::Lambda(l, _) => self.types.items(l).len(),
                    _ => 0,
                }
            }
            Type::Param(p) => self.param_arity(p),
            Type::Decl(a) => self.syms.aliases[a.idx()].tparams.len(),
            Type::Member(prefix, name) => self.type_member_arity(prefix, name),
            _ => 0,
        }
    }

    /// The type parameters a type member is declared with (`type Typeclass[T]` of a trait,
    /// seen as `C.this.Typeclass`), found in the prefix's class or one of its bases.
    fn type_member_arity(&self, prefix: TypeId, name: Name) -> usize {
        self.type_member_alias(prefix, name).map_or(0, |a| self.syms.aliases[a.idx()].tparams.len())
    }

    /// The declaration of the type member `name` in the prefix's class or one of its bases, or
    /// in a class of its self type (`Existentials.this.Type` of `_: Types & Exprs =>`).
    pub(super) fn type_member_alias(&self, prefix: TypeId, name: Name) -> Option<AliasId> {
        let c = self.declaring_class(prefix, 0)?;
        let in_class = |k: ClassId| {
            let info = self.syms.class(k);
            std::iter::once(k).chain(info.base_types.iter().map(|&(b, _)| b)).find_map(|b| self.syms.class(b).type_aliases.get(&name).copied())
        };
        if let Some(a) = in_class(c) {
            return Some(a);
        }
        let mut parts = vec![self.syms.class(c).declared_self?];
        while let Some(t) = parts.pop() {
            match self.types.get(t) {
                Type::Class(k, _) => {
                    if let Some(a) = in_class(k) {
                        return Some(a);
                    }
                }
                Type::Inter(a, b) => parts.extend([a, b]),
                _ => {}
            }
        }
        None
    }

    /// The variances an abstract type constructor's parameters are declared with
    /// (`type Type[+A]` of cats' newtypes), by which its applications are compared.
    pub(super) fn member_variances(&self, m: TypeId) -> Vec<i8> {
        let alias = match self.types.get(m) {
            Type::Decl(a) => Some(a),
            Type::Member(prefix, name) => self.type_member_alias(prefix, name),
            _ => None,
        };
        alias.map_or(Vec::new(), |a| self.syms.aliases[a.idx()].tparams.iter().map(|&p| self.syms.tparam(p).variance).collect())
    }

    /// The class a prefix's type members are declared in: the class of `C.this` or `C[..]`,
    /// of a path's declared type (`P.F` for a parameter `P: Parallel[M]`), of a refinement's
    /// parent; read without completing anything.
    fn declaring_class(&self, prefix: TypeId, depth: u32) -> Option<ClassId> {
        if depth > 8 {
            return None;
        }
        match self.types.get(prefix) {
            Type::This(_) | Type::Class(..) | Type::Nested(..) => self.types.named_class(prefix),
            Type::Refined(base, _) => self.declaring_class(base, depth + 1),
            // The head of an alias's right-hand side names the class whatever the arguments.
            Type::Alias(a, _) if self.syms.alias(a).state() == Completion::Done => self.declaring_class(self.alias_body(a), depth + 1),
            Type::Term(s) | Type::Select(_, s) => {
                let ret = self.syms.sym(s).sig.as_ref()?.ret;
                self.declaring_class(ret, depth + 1)
            }
            _ => None,
        }
    }

    /// A parameter written `F[_]`, or one bounded by a type lambda (`F <: [X] =>> Any`), takes
    /// type arguments.
    pub(super) fn param_arity(&self, p: TParamId) -> usize {
        let info = self.syms.tparam(p);
        match self.types.get(info.upper) {
            Type::Lambda(l, _) if info.arity == 0 => self.types.items(l).len(),
            _ => info.arity as usize,
        }
    }

    /// The numeric bound of an opaque type outside its scope (`opaque type Pos <: Int`), for
    /// the primitive operators.
    pub(super) fn numeric_bound(&mut self, t: TypeId) -> Option<TypeId> {
        self.opaque_bound(t).filter(|&b| self.is_numeric(b).is_some())
    }

    /// Outside its scope, an opaque type with a bound (`opaque type Name <: String`) is seen as
    /// that bound where the primitive operators are dispatched; its members and supertypes come
    /// through the bound's base types.
    pub(crate) fn opaque_bound(&mut self, t: TypeId) -> Option<TypeId> {
        let Type::Class(c, args) = self.types.get(t) else { return None };
        if self.syms.class(c).kind != ClassKind::Opaque || self.transparent.contains(&c) {
            return None;
        }
        // Another module's opaque type has its bound once its pickle is read.
        self.complete_class(c);
        let info = self.syms.class(c);
        let bound = *info.parents.first()?;
        let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
        Some(self.types.subst(bound, &subst))
    }

    #[cold]
    fn resolve_literal_type(&mut self, e: ast::ExprId, span: Span) -> TypeId {
        let ast = self.cur_ast();
        let value = match ast.expr(e) {
            ast::Expr::IntLit(v) if v >= i32::MIN as i64 && v <= i32::MAX as i64 => LitVal::Int(v as i32),
            ast::Expr::IntLit(_) => {
                self.error(span, "number too large for Int; use an L suffix for Long");
                return ERROR;
            }
            ast::Expr::LongLit(v) => LitVal::Long(v),
            ast::Expr::DoubleLit(v) => LitVal::Double(v.to_bits()),
            // A literal type is never an untyped number (dotty's `Parsers.literal`, `inTypeOrSingleton`).
            ast::Expr::DecimalLit(s) => LitVal::Double(ast.str(s).parse::<f64>().unwrap_or(0.0).to_bits()),
            ast::Expr::FloatLit(_) => return self.b.t_float,
            ast::Expr::CharLit(c) => LitVal::Char(c as u16),
            ast::Expr::BoolLit(b) => LitVal::Bool(b),
            ast::Expr::StringLit(s) => LitVal::Str(self.interner.intern(ast.str(s))),
            _ => return ERROR,
        };
        self.types.lit(value)
    }

    /// A member of a term path whose alias comes back to itself (`z.A` for a `z: X & Y` whose
    /// sides define `A` and `B` through each other), reported where the type is written.
    #[cold]
    fn report_recursion_at_use(&mut self, span: Span, prefix: TypeId, name: Name) {
        let member = self.types.mk(Type::Member(prefix, name));
        let msg = format!(
            "Recursion limit exceeded.\nMaybe there is an illegal cyclic reference?\nIf that's not the case, you could also try to increase the stacksize using the -Xss JVM option.\nA recurring operation is (inner to outer):\n\n  find-member {}",
            self.show(member)
        );
        self.error(span, msg);
    }

    /// `x.type`: the singleton type of the path; `Obj.type` is the type of the object.
    #[cold]
    fn singleton_type(&mut self, path: TyExprId, span: Span) -> TypeId {
        let this_class = match self.cur_ast().ty(path) {
            TyExpr::Name(names::THIS) => self.this_class(),
            TyExpr::Select(k, names::THIS) => match self.cur_ast().ty(k) {
                TyExpr::Name(n) => self.qualified_this_class(n),
                _ => None,
            },
            _ => None,
        };
        if let Some(c) = this_class {
            self.complete_class(c);
            return self.this_prefix(c);
        }
        match self.type_path(path) {
            // A local object stands for itself as its class type, as a top-level one does.
            Some(Ok(p)) => match self.types.get(p) {
                Type::Term(s) => match self.local_module_of_sym(s) {
                    Some(c) => self.types.class(c, &[]),
                    None => p,
                },
                _ => p,
            },
            found => match found.and_then(|r| r.err()).and_then(|s| self.given_singleton(s, span)) {
                Some(p) => p,
                None => {
                    self.error(span, "a singleton type needs a val, a parameter or an object");
                    ERROR
                }
            },
        }
    }

    /// `quotes.type` for a `def quotes(using q: Quotes): q.type`: dotc types the reference as
    /// its application to the given in scope, whose path is the singleton.
    fn given_singleton(&mut self, s: SymId, span: Span) -> Option<TypeId> {
        if self.syms.sym(s).kind != SymKind::Def {
            return None;
        }
        let sig = self.sig_arc(s);
        let [clause] = sig.clauses.as_slice() else { return None };
        let [param] = clause.params.as_slice() else { return None };
        if !clause.is_using || !sig.tparams.is_empty() || !matches!(self.types.get(sig.ret), Type::Term(p) if p == param.sym) {
            return None;
        }
        // A probe whose tree goes: its path, read before the retraction, is what is kept.
        let mark = self.attempt();
        let found = self.resolve_given_typed(param.ty, span);
        let path = found.and_then(|(te, _)| self.path_of(te));
        self.given_ambiguity = None;
        self.retract(mark);
        let path = path?;
        matches!(self.types.get(path), Type::Term(_) | Type::Select(..)).then_some(path)
    }

    /// The singleton type of the path a type expression names: a val, a parameter or an object,
    /// or a val selected from such a path (`c.x.type`). `Err` names the symbol that is no path
    /// (a def or a var), None what is no term at all, a package or an object among them.
    fn type_path(&mut self, q: TyExprId) -> Option<Result<TypeId, SymId>> {
        let ast = self.cur_ast();
        let via_ref = |t: &mut Self, r: TermRef| {
            if let TermRef::SelfAlias(_) = r {
                return t.path_of_ref(r).ok().map(Ok);
            }
            let s = r.sym()?;
            Some(t.path_of_ref(r).map_err(|_| s))
        };
        match ast.ty(q) {
            TyExpr::Name(names::THIS) => {
                let c = self.this_class()?;
                self.complete_class(c);
                Some(Ok(self.inline_this_of(c).unwrap_or_else(|| self.this_prefix(c))))
            }
            TyExpr::Select(k, names::THIS) => {
                let TyExpr::Name(n) = ast.ty(k) else { return None };
                let c = self.qualified_this_class(n)?;
                self.complete_class(c);
                Some(Ok(self.inline_this_of(c).unwrap_or_else(|| self.this_prefix(c))))
            }
            TyExpr::Name(n) => {
                let r = self.lookup_term(n)?;
                via_ref(self, r)
            }
            TyExpr::Select(inner, n) => {
                if let Some(r) = self.resolve_type_path(q) {
                    if let Some(found) = via_ref(self, r) {
                        return Some(found);
                    }
                }
                let p = match self.type_path(inner)? {
                    Ok(p) => p,
                    Err(s) => return Some(Err(s)),
                };
                let under = self.widen_path(p);
                let (s, _) = self.find_member(under, n)?;
                let info = self.syms.sym(s);
                Some(match info.kind {
                    SymKind::Val | SymKind::Given if !info.by_name => Ok(self.types.mk(Type::Select(p, s))),
                    SymKind::Object(c) => Ok(self.types.class(c, &[])),
                    _ => Err(s),
                })
            }
            _ => None,
        }
    }

    // ---- completion ----

    pub fn make_tparams(&mut self, tps: &[ast::TypeParam]) -> Vec<(Name, TParamId)> {
        let ids: Vec<(Name, TParamId)> = tps
            .iter()
            .map(|tp| {
                let p = self.syms.new_tparam(tp.name, tp.variance);
                self.syms.tparams[p.idx()].arity = tp.arity;
                if !tp.hk_variances.is_empty() {
                    self.syms.tparams[p.idx()].hk_variances = tp.hk_variances.clone();
                }
                (tp.name, p)
            })
            .collect();
        if self.index.is_some() {
            self.index_tparams(tps, &ids);
        }
        ids
    }

    /// `opaque type Name <: String = String`: outside its scope the opaque type has the members
    /// and the supertypes of its bound.
    fn set_opaque_bound(&mut self, c: ClassId, under: TypeId, bound: TypeId, span: Span) {
        if !self.is_sub(under, bound) {
            let msg = format!("the underlying type {} does not conform to the bound {}", self.show(under), self.show(bound));
            self.error(span, msg);
        }
        let Type::Class(bc, bargs) = self.types.get(bound) else { return };
        self.syms.class_mut(c).parents = vec![bound];
        self.complete_class(bc);
        let subst: Subst =
            self.syms.class(bc).tparams.iter().copied().zip(self.types.items(bargs).iter().copied()).collect();
        let inherited: Vec<(ClassId, TypeId)> = self.syms.class(bc).base_types.clone();
        for (x, xt) in inherited {
            let t = self.types.subst(xt, &subst);
            self.syms.class_mut(c).base_types.push((x, t));
        }
    }

    pub fn resolve_tparam_bounds(&mut self, tps: &[ast::TypeParam], ids: &[(Name, TParamId)]) {
        self.resolve_tparam_bounds_of(tps, ids, |_| true);
    }

    fn resolve_tparam_bounds_of(&mut self, tps: &[ast::TypeParam], ids: &[(Name, TParamId)], which: impl Fn(&ast::TypeParam) -> bool) {
        for (tp, &(_, id)) in tps.iter().zip(ids).filter(|(tp, _)| which(tp)) {
            let itself = self.types.param(id);
            for (bound, upper) in [(tp.upper, true), (tp.lower, false)] {
                let Some(b) = bound else { continue };
                let t = self.resolve_type(b);
                if t == itself || self.eta_reduce(t) == itself {
                    let msg = format!("cyclic reference involving type {}", self.name_str(tp.name));
                    self.error(tp.span, msg);
                    continue;
                }
                let info = &mut self.syms.tparams[id.idx()];
                if upper {
                    info.upper = t;
                } else {
                    info.lower = t;
                }
            }
        }
    }

    /// Resolves parameter clauses. `existing` holds pre-created symbols (constructor params);
    /// context bounds become a trailing using clause.
    pub fn resolve_clauses(
        &mut self,
        clauses: &[ast::ParamClause],
        existing: Option<Vec<Vec<SymId>>>,
        tps: &[ast::TypeParam],
        ids: &[(Name, TParamId)],
        class_owner: Option<ClassId>,
    ) -> Vec<ClauseSig> {
        let file = self.env.file;
        let ast = self.cur_ast();
        let mut out = Vec::with_capacity(clauses.len() + 1);
        for (ci, clause) in clauses.iter().enumerate() {
            let mut sig = ClauseSig { params: Vec::with_capacity(clause.params.len()), is_using: clause.is_using, is_implicit: clause.is_implicit };
            for (pi, p) in clause.params.iter().enumerate() {
                let repeated = matches!(ast.ty(p.ty), TyExpr::Repeated(_));
                let by_name = match ast.ty(p.ty) {
                    TyExpr::Repeated(inner) => matches!(ast.ty(inner), TyExpr::ByName(_)),
                    other => matches!(other, TyExpr::ByName(_)),
                };
                let resolved = self.resolve_type(p.ty);
                let spelled = self.spelled_cons(p.ty, resolved);
                // A tuple parameter spelled as a `*:` chain is the chain to its body and its
                // callers, as scalac has it: a selection of its tuple class's members casts it
                // to that class (`apply_member`, `Typer.trySmallGenericTuple`); an array of
                // tuples is `Product[]` where the chain is `Product`.
                let ty = spelled.unwrap_or(resolved);
                let local_ty = if repeated {
                    match self.seq_class() {
                        Some(seq) => self.types.class(seq, &[ty]),
                        None => ERROR,
                    }
                } else {
                    ty
                };
                let taken = out.iter().chain([&sig]).any(|c: &ClauseSig| c.params.iter().any(|q| q.name == p.name));
                if taken && existing.is_none() {
                    let n = self.name_str(p.name);
                    self.error(p.span, format!("{} is already defined as parameter {}", n, n));
                }
                let sym = match &existing {
                    Some(syms) => syms[ci][pi],
                    None => {
                        let sym = self.syms.new_sym(p.name, SymKind::Param, p.mods, Owner::Local, file, None, p.span);
                        if self.index.is_some() {
                            self.index_param(sym);
                        }
                        sym
                    }
                };
                {
                    let mut s = self.syms.sym_mut(sym);
                    s.sig = Some(Arc::new(MethodSig::value(local_ty)));
                    s.state().set(Completion::Done);
                    s.by_name = by_name;
                }
                // Later parameters and the result type can name it (`y: x.type`), and a using
                // parameter is the given of `quotes.type` there.
                self.bind_local(p.name, sym);
                if clause.is_using {
                    self.bind_given(sym);
                }
                let ty = spelled.unwrap_or(ty);
                sig.params.push(ParamSig {
                    name: p.name,
                    ty,
                    by_name,
                    repeated,
                    has_default: p.default.is_some(),
                    sym,
                });
            }
            out.push(sig);
        }
        let mut evidence = ClauseSig { params: Vec::new(), is_using: true, is_implicit: false };
        for (index, (tp, &(_, id))) in tps.iter().zip(ids).enumerate() {
            for (i, &bound) in tp.context_bounds.iter().enumerate() {
                let ctor = self.resolve_type_ctor(bound);
                let arg = self.types.param(id);
                let ty = self.types.apply_ctor(ctor, &[arg]);
                if ty == ERROR && ctor != ERROR {
                    self.error(tp.span, "a context bound needs a type constructor with one parameter");
                }
                // A tag that is erased here is no parameter: a call finds it and drops it.
                if let Some(tags) = self.erased_tags.filter(|_| index < 32) {
                    if self.is_class_tag(ty) {
                        self.erased_tags = Some(tags | 1 << index);
                        continue;
                    }
                }
                // scalac's `evidence$1`, `evidence$2`, ... numbered within the definition.
                let name = match tp.evidence_names.get(i).copied().filter(|&n| n != names::EMPTY) {
                    Some(written) => written,
                    None => {
                        let n = evidence.params.len() + 1;
                        self.interner.intern(&format!("evidence${}", n))
                    }
                };
                let (kind, owner, m) = match class_owner {
                    Some(c) => (SymKind::Val, Owner::Class(c), mods::PRIVATE),
                    None => (SymKind::Param, Owner::Local, 0),
                };
                let sym = self.syms.new_sym(name, kind, m, owner, file, None, tp.span);
                {
                    let mut s = self.syms.sym_mut(sym);
                    s.sig = Some(Arc::new(MethodSig::value(ty)));
                    s.state().set(Completion::Done);
                }
                if let Some(c) = class_owner {
                    self.syms.add_member(c, sym);
                    self.syms.class_mut(c).givens.push(sym);
                }
                evidence.params.push(ParamSig {
                    name,
                    ty,
                    by_name: false,
                    repeated: false,
                    has_default: false,
                    sym,
                });
            }
        }
        // The evidence joins a trailing using clause, in front of its parameters.
        match out.last_mut() {
            Some(last) if last.is_using && !evidence.params.is_empty() => {
                evidence.params.append(&mut last.params);
                last.params = evidence.params;
            }
            _ if !evidence.params.is_empty() => out.push(evidence),
            _ => {}
        }
        out
    }

    pub fn complete_alias(&mut self, a: AliasId) {
        let state = self.syms.alias(a).state().get();
        if state == Completion::Done {
            return;
        }
        if state == Completion::InProgress && !self.syms.alias(a).state().mine() {
            self.wait_cell(crate::shared::CellKey::new(super::check::CELL_ALIAS, a.0), |w| w.syms.alias(a).state() == Completion::Done);
            return;
        }
        let p = self.phase(super::profile::Phase::Alias);
        self.completing += 1;
        // A shared alias is every worker's: completed under the loader's lock.
        let complete = |t: &mut Self| {
            let outer = if t.deps.is_some() { Some(t.deps_enter_alias(a)) } else { None };
            if t.forked && a.0 < crate::arena::LOCAL_BASE {
                t.with_loader_for(crate::measure::Hold::AliasCompletion, |w| w.complete_alias_now(a))
            } else {
                t.complete_alias_now(a)
            }
            if let Some(outer) = outer {
                t.deps_leave(outer);
            }
        };
        if self.made_in_block(self.syms.aliases[a.idx()].owner) {
            complete(self);
        } else {
            self.once(|t| complete(t));
        }
        self.leave_completion();
        self.phase_end(p);
        self.outermost_completion_done();
    }

    fn complete_alias_now(&mut self, a: AliasId) {
        let (state, file, def_id, owner) = {
            let i = self.syms.alias(a);
            (i.state().get(), i.file, i.def, i.owner)
        };
        match state {
            Completion::Done => return,
            Completion::InProgress => {
                let span = def_id.map_or(Span::default(), |d| self.ast(file).def(d).span);
                let name = self.name_str(self.syms.aliases[a.idx()].name);
                let msg = format!("illegal cyclic type reference: alias {} of type {} refers back to the type itself", name, name);
                self.diags.error(file, span, msg);
                self.syms.alias(a).state().set(Completion::Done);
                return;
            }
            Completion::NotStarted => {}
        }
        self.syms.alias(a).state().set(Completion::InProgress);
        if self.forked {
            self.hold_ordered_alias(a);
        }
        let Some(def_id) = def_id else {
            self.complete_loaded_alias(a);
            return;
        };
        let def = self.ast(file).def(def_id);
        let DefKind::TypeAlias { tparams, rhs, lower, upper } = &def.kind else { return };
        let env = self.env_at(file, owner, def.span.start);
        let (ids, ty, bounds) = self.with_env(env, |t| t.resolve_alias_def(a, tparams, *rhs, *lower, *upper));
        self.finish_alias(a, ids, ty, bounds);
    }

    /// `TEQ_ALIAS_ORDER`'s readers' side: a worker asking for the member while it is unstarted
    /// waits until another worker claims it, unless it is the first to ask or holds the loader's
    /// lock, which the claimant needs; one that finds another worker's claim tells the holder.
    #[cold]
    #[inline(never)]
    pub(super) fn order_alias_readers(&self, a: AliasId) {
        let Some(order) = alias_order().filter(|o| self.name_str(self.syms.aliases[a.idx()].name) == o.name) else { return };
        if self.syms.alias(a).state() == Completion::NotStarted {
            if !order.first.swap(true, std::sync::atomic::Ordering::AcqRel) || crate::shared::lock_depth() > 0 {
                return;
            }
            ordered_wait(|| self.syms.alias(a).state() != Completion::NotStarted);
        }
        let state = self.syms.alias(a).state();
        if state == Completion::InProgress && !state.mine() {
            order.met.store(true, std::sync::atomic::Ordering::Release);
        }
    }

    /// `TEQ_ALIAS_ORDER`'s holder's side: the member just claimed stays in progress until
    /// another worker has met it so, and half a second more, for what that worker reads of the
    /// member after its first look (the bound a check asks for after the type is resolved).
    #[cold]
    #[inline(never)]
    fn hold_ordered_alias(&self, a: AliasId) {
        if let Some(order) = alias_order().filter(|o| self.name_str(self.syms.aliases[a.idx()].name) == o.name) {
            ordered_wait(|| order.met.load(std::sync::atomic::Ordering::Acquire));
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    /// A type alias local to a block, resolved where it stands and bound in the block's scope.
    pub(super) fn enter_local_alias(&mut self, file: FileId, d: ast::DefId) {
        let def = self.ast(file).def(d);
        let DefKind::TypeAlias { tparams, rhs, lower, upper } = &def.kind else { return };
        if rhs.is_none() {
            self.error(def.span, "only classes can have declared but undefined members");
            return;
        }
        let a = AliasId(self.syms.aliases.len() as u32);
        self.syms.aliases.push(AliasInfo {
            name: def.name,
            owner: Owner::Local,
            file,
            def: Some(d),
            tparams: Vec::new(),
            rhs: ERROR,
            bounds: None,
        });
        self.syms.alias_cells.set(a.0, Completion::InProgress);
        let depth = self.env.frames.len();
        let (ids, ty, bounds) = self.resolve_alias_def(a, tparams, *rhs, *lower, *upper);
        self.env.frames.truncate(depth);
        self.finish_alias(a, ids, ty, bounds);
        if let Some(Frame::Locals { aliases, .. }) = self.env.frames.last_mut() {
            aliases.push((def.name, a));
        }
    }

    fn resolve_alias_def(
        &mut self,
        a: AliasId,
        tparams: &[ast::TypeParam],
        rhs: Option<TyExprId>,
        lower: Option<TyExprId>,
        upper: Option<TyExprId>,
    ) -> (Vec<(Name, TParamId)>, TypeId, Option<(TypeId, TypeId)>) {
        let t = self;
        {
            let ids = t.make_tparams(tparams);
            // A match type names the alias itself in its cases, by these parameters.
            t.syms.aliases[a.idx()].tparams = ids.iter().map(|&(_, p)| p).collect();
            t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
            t.resolve_tparam_bounds(tparams, &ids);
            let over = |t: &mut Self, ty: TypeId| {
                if ids.is_empty() || ty == ANY || ty == NOTHING {
                    return ty;
                }
                let ps: Vec<TypeId> = ids.iter().map(|&(_, p)| t.types.param(p)).collect();
                let l = t.types.list(&ps);
                t.types.mk(Type::Lambda(l, ty))
            };
            match rhs {
                Some(rhs) => {
                    let mut ty = t.resolve_type_ctor(rhs);
                    if let (Some(u), Type::Match(scrut, m)) = (upper, t.types.get(ty)) {
                        let bound = t.resolve_type(u);
                        let cases = t.types.match_info(m).cases.to_vec();
                        ty = t.types.match_type(scrut, &cases, bound);
                    }
                    (ids, ty, None)
                }
                None => {
                    let lo = lower.map_or(NOTHING, |b| t.resolve_type(b));
                    let hi = upper.map_or(ANY, |b| t.resolve_type(b));
                    let (lo, hi) = (over(t, lo), over(t, hi));
                    (ids, ERROR, Some((lo, hi)))
                }
            }
        }
    }

    fn finish_alias(&mut self, a: AliasId, ids: Vec<(Name, TParamId)>, ty: TypeId, bounds: Option<(TypeId, TypeId)>) {
        let mut info = self.syms.alias_mut(a);
        info.tparams = ids.iter().map(|&(_, p)| p).collect();
        info.rhs = ty;
        info.bounds = bounds;
        info.state().set(Completion::Done);
        if bounds.is_some() {
            self.check_member_cycle(a);
        }
    }

    /// `type T <: T`, or through other members: scalac's illegal cyclic reference. The bound
    /// is dropped so that nothing follows it again.
    #[cold]
    fn check_member_cycle(&mut self, a: AliasId) {
        let (file, def_id, name, bounds) = {
            let i = &self.syms.aliases[a.idx()];
            (i.file, i.def, i.name, i.bounds)
        };
        let Some((_, upper)) = bounds else { return };
        let Some(def_id) = def_id else { return };
        // Only the bounds followed count against the budget; the parts of an intersection, a
        // union or a refinement are the bound's own structure.
        let mut pending = vec![upper];
        let mut links = 0;
        while let Some(t) = pending.pop() {
            let (prefix, n) = match self.types.get(t) {
                Type::Member(p, n) => (Some(p), n),
                Type::Decl(b) if b == a => (None, name),
                Type::Decl(_) if links >= 32 => continue,
                Type::Decl(b) => {
                    links += 1;
                    self.complete_alias(b);
                    if let Some((_, hi)) = self.syms.aliases[b.idx()].bounds {
                        pending.push(hi);
                    }
                    continue;
                }
                Type::Inter(l, r) | Type::Union(l, r) => {
                    pending.push(r);
                    pending.push(l);
                    continue;
                }
                Type::Refined(parent, _) => {
                    pending.push(parent);
                    continue;
                }
                _ => continue,
            };
            // Through a path (`t.b.M` for a `b: t.T` whose `T <: a.T`) the member is the one
            // declared where the path's class declares it.
            let declares = |t: &Self, c: ClassId| {
                t.syms.class(c).type_aliases.get(&n) == Some(&a)
                    || t.syms.class(c).base_types.iter().any(|&(b, _)| t.syms.class(b).type_aliases.get(&n) == Some(&a))
            };
            let same_scope = prefix.map_or(true, |p| match self.types.get(p) {
                Type::This(c) | Type::Class(c, _) => declares(self, c),
                _ => self.class_of(p).map_or(false, |c| declares(self, c)),
            });
            if n == name && same_scope {
                let span = self.ast(file).def(def_id).span;
                let msg = format!(
                    "illegal cyclic type reference: upper bound {} of type {} refers back to the type itself",
                    self.show(upper),
                    self.name_str(name)
                );
                self.diags.error(file, span, msg);
                self.syms.aliases[a.idx()].bounds = Some((NOTHING, ANY));
                return;
            }
            let Some(p) = prefix else { continue };
            if links >= 32 {
                continue;
            }
            links += 1;
            if let Some(super::members::MemberInfo::Bounds(_, hi)) = self.type_member(p, n) {
                pending.push(hi);
            }
        }
    }

    /// Creates the type parameters of a class without resolving anything else, so that
    /// variances are available while other headers are still being completed; a class another
    /// thread is completing (a library class, under the loader's lock) is waited for, where its
    /// record would be read half filled.
    pub fn complete_class_tparams(&mut self, c: ClassId) {
        let info = self.syms.class(c);
        match info.state().get() {
            // A class without a definition has its type parameters from the start.
            Completion::NotStarted if info.def.is_some() => self.complete_class(c),
            Completion::InProgress if self.forked && !info.state().mine() => self.complete_class(c),
            _ => {}
        }
    }

    /// Waits out another thread's completion of `c` (a library class's, under the loader's lock)
    /// before a worker reads the record, which it reads whole or as it stood before; off the fork
    /// nothing.
    #[inline]
    pub fn settle_class(&mut self, c: ClassId) {
        if self.forked && self.syms.class_cells.get(c.0) == Completion::InProgress && !self.syms.class_cells.mine(c.0) {
            self.complete_class(c);
        }
    }

    /// The check is what a caller inlines, the completion a call: most calls find the class
    /// complete.
    #[inline]
    pub fn complete_class(&mut self, c: ClassId) {
        if self.syms.class_cells.get(c.0) != Completion::Done {
            self.complete_class_uncompleted(c);
        }
    }

    #[inline(never)]
    fn complete_class_uncompleted(&mut self, c: ClassId) {
        let state = self.syms.class(c).state().get();
        if state == Completion::Done {
            return;
        }
        // A class of the shared region after the fork is the std's or a library's (the
        // program's complete in the signature phase): claimed and completed within one hold
        // of the loader's lock, whose holder then never meets another thread's claim and so
        // never waits under the lock. A thread outside the lock that
        // finds it claimed waits for the holder.
        if self.forked && self.shared_class(c) && crate::shared::lock_depth() == 0 {
            if state == Completion::InProgress {
                self.wait_cell(crate::shared::CellKey::new(super::check::CELL_CLASS, c.0), |w| w.syms.class(c).state() == Completion::Done);
                return;
            }
            return self.with_loader_for(crate::measure::Hold::ClassCompletion, |w| w.complete_class_uncompleted(c));
        }
        // The completion cell: claimed by the first thread to find it
        // unclaimed; another thread finding it claimed waits for the completion, unless the wait
        // would close a cycle, in which case it reads the class as far as it is, as one thread would.
        if state == Completion::InProgress || !self.syms.class_cells.claim(c.0) {
            if !self.syms.class(c).state().mine() {
                self.wait_cell(crate::shared::CellKey::new(super::check::CELL_CLASS, c.0), |w| w.syms.class(c).state() == Completion::Done);
            }
            return;
        }
        {
            let p = self.phase_split(super::profile::Phase::Complete, super::profile::Phase::LibraryComplete, |t| t.is_library_class(c));
            self.complete_class_now(c);
            self.phase_end(p);
            // The children of a sealed std class stand in its file: they complete with it, so
            // that the class knows them whenever it is complete. Once the outermost completion
            // is done, so that a child never meets its parent half complete.
            let info = self.syms.class(c);
            let sealed = info.kind == ClassKind::Enum || info.mods & mods::SEALED != 0;
            if sealed && info.def.is_some() && self.std.slot_of(info.file).is_some() {
                self.sealed_std_files.push(info.file);
            }
            self.outermost_completion_done();
        }
    }

    /// The sealed std files whose classes a completion left to its outermost one, which the
    /// typing's end came before (`outermost_completion_done`; a signature's completion leaves
    /// without it): completed once every body is typed, after the workers' merge or one worker's
    /// walk alike, so that a sealed std class knows its children before a match is checked
    /// against it.
    pub(super) fn complete_pending_sealed_files(&mut self) {
        self.sealed_std_files.sort_unstable();
        self.sealed_std_files.dedup();
        self.outermost_completion_done();
    }

    /// Once no completion is under way, the files of the sealed std classes completed meanwhile
    /// complete their classes, so that the sealed ones know their children.
    pub(super) fn outermost_completion_done(&mut self) {
        if self.completing == 0 {
            while let Some(file) = self.sealed_std_files.pop() {
                self.complete_file_classes(file);
            }
            if let Some(from) = self.late_naming_from.take() {
                self.name_late_alternatives(from);
            }
        }
    }

    fn complete_class_now(&mut self, c: ClassId) {
        if self.shared_class(c) {
            return self.with_loader_for(crate::measure::Hold::ClassCompletion, |w| w.complete_class_now_unlocked(c));
        }
        self.complete_class_now_unlocked(c)
    }

    fn complete_class_now_unlocked(&mut self, c: ClassId) {
        if super::prep::capturing() {
            self.prep_note_class('C', c);
        }
        self.completing += 1;
        let outer = if self.deps.is_some() { Some(self.deps_enter_class(c)) } else { None };
        if self.made_in_block(self.syms.class(c).owner) {
            self.complete_class_inner(c);
        } else {
            self.once(|t| t.complete_class_inner(c));
        }
        if let Some(outer) = outer {
            self.deps_leave(outer);
        }
        self.leave_completion();
    }

    /// The end of a completion; the outermost one settles the output names of the library
    /// classes completed under it.
    pub(super) fn leave_completion(&mut self) {
        self.completing -= 1;
        self.settle_pending_names();
    }

    /// Names the members of the library classes whose completion ended under another
    /// completion or under a naming, once neither is under way.
    pub(super) fn settle_pending_names(&mut self) {
        if self.completing > 0 || self.naming_library > 0 {
            return;
        }
        while let Some(c) = self.pending_library_names.pop() {
            self.name_library_members(c);
        }
    }

    fn complete_class_inner(&mut self, c: ClassId) {
        let (file, def_id, owner, kind) = {
            let i = self.syms.class(c);
            (i.file, i.def, i.owner, i.kind)
        };
        self.syms.class_mut(c).state().set(Completion::InProgress);
        let self_args_done = |t: &mut Self| {
            let targs: Vec<TypeId> =
                t.syms.class(c).tparams.clone().iter().map(|&p| t.types.param(p)).collect();
            t.types.class(c, &targs)
        };
        let Some(def_id) = def_id else {
            // A class read from a jar, or a synthesized enum companion.
            if self.loaded.is_some() {
                self.complete_loaded_class(c);
                return;
            }
            let self_ty = self_args_done(self);
            let mut info = self.syms.class_mut(c);
            info.base_types.push((c, self_ty));
            info.state().set(Completion::Done);
            return;
        };
        let def = self.ast(file).def(def_id);
        let env = match owner {
            Owner::Local => self.anon_env(c),
            _ => self.env_at(file, owner, def.span.start),
        };
        self.with_env(env, |t| match &def.kind {
            DefKind::Class(cls) => {
                t.complete_class_def(c, kind, cls, owner);
                t.seal_primary_ctor(c);
            }
            DefKind::Given(g) => {
                let ids = t.make_tparams(&g.tparams);
                t.syms.class_mut(c).tparams = ids.iter().map(|&(_, p)| p).collect();
                t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
                t.resolve_tparam_bounds(&g.tparams, &ids);
                let existing = t.syms.class(c).ctor_syms.clone();
                let ctor = t.resolve_clauses(&g.clauses, Some(existing), &g.tparams, &ids, Some(c));
                t.syms.class_mut(c).ctor = ctor;
                let parent = t.resolve_type(g.ty);
                t.set_parents(c, vec![(parent, t.cur_ast().ty_spans[g.ty.idx()])]);
            }
            DefKind::TypeAlias { tparams, rhs, upper, .. } => {
                let ids = t.make_tparams(tparams);
                t.syms.class_mut(c).tparams = ids.iter().map(|&(_, p)| p).collect();
                t.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: t.sites.owners.len() as u32 });
                t.resolve_tparam_bounds(tparams, &ids);
                let under = rhs.map_or(ERROR, |r| t.resolve_type(r));
                t.syms.class_mut(c).underlying = Some(under);
                t.set_parents(c, Vec::new());
                if let Some(b) = *upper {
                    let bt = t.resolve_type(b);
                    t.set_opaque_bound(c, under, bt, t.cur_ast().ty_spans[b.idx()]);
                }
            }
            _ => {}
        });
        self.syms.class_mut(c).state().set(Completion::Done);
    }

    fn complete_class_def(&mut self, c: ClassId, kind: ClassKind, cls: &'a ast::ClassDef, owner: Owner) {
        let enum_class = match (kind, owner) {
            (ClassKind::EnumCase, Owner::Class(companion)) => self.syms.class(companion).companion,
            _ => None,
        };
        let mut tparam_asts: &[ast::TypeParam] = &cls.tparams;
        let mut inherited = false;
        if let Some(e) = enum_class {
            self.complete_class(e);
            if let Some(copied) = self.copied_enum_tparams(cls, e) {
                tparam_asts = copied;
                inherited = true;
            }
        }
        let ids = self.make_tparams(tparam_asts);
        self.syms.class_mut(c).tparams = ids.iter().map(|&(_, p)| p).collect();
        self.env.frames.push(Frame::Locals { names: Vec::new(), tparams: ids.clone(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new(), owner: self.sites.owners.len() as u32 });
        // The bound of a higher-kinded parameter names a class that may extend this one
        // (`FromIterator[+C[X] <: Iterable[X]]`, `Iterable extends FromIterator[Iterable]`),
        // so it is resolved once the class is complete enough to be extended.
        self.resolve_tparam_bounds_of(tparam_asts, &ids, |tp| tp.arity == 0);
        let existing = self.syms.class(c).ctor_syms.clone();
        // Scala copies the type parameters of the enum to a case without their context bounds.
        let bounds_for_ctor: &[ast::TypeParam] = if inherited { &[] } else { tparam_asts };
        let ctor = self.resolve_clauses(&cls.clauses, Some(existing), bounds_for_ctor, &ids, Some(c));
        if kind == ClassKind::Trait {
            if let Some(p) = cls.clauses.iter().flat_map(|cl| cl.params.iter()).find(|p| {
                matches!(self.cur_ast().ty(p.ty), TyExpr::ByName(_))
            }) {
                self.error(p.span, "implementation restriction: traits cannot have by name parameters");
            }
        }
        if kind == ClassKind::Enum {
            self.syms.class_mut(c).stateful = !ctor.is_empty() || self.body_has_initializers(&cls.body);
        }
        self.syms.class_mut(c).ctor = ctor;

        let mut parents: Vec<(TypeId, Span)> = Vec::new();
        let mut trait_calls: Vec<(ClassId, crate::tir::ParentCall)> = Vec::new();
        for (i, p) in cls.parents.iter().enumerate() {
            let t = match self.bare_generic_class(p).filter(|_| i == 0 && enum_class.is_none()) {
                // `extends Box(7)` leaves the type arguments of the superclass to inference.
                Some(parent) => match self.type_parent_ctor(c, Some(p), parent, None, false) {
                    Some((call, ty)) => {
                        self.inferred_parent_args.insert(c, call);
                        ty
                    }
                    None => ERROR,
                },
                // So does `extends Plain(3)` those of a parameterized trait, as scalac's
                // `typedParent` types the application; its arguments are passed as typed here.
                None => match self.bare_generic_trait(p).filter(|_| kind != ClassKind::Trait) {
                    Some(tr) => match self.type_parent_ctor(c, Some(p), tr, None, false) {
                        Some((call, ty)) => {
                            trait_calls.push((tr, call));
                            ty
                        }
                        None => ERROR,
                    },
                    None => self.resolve_type(p.ty),
                },
            };
            parents.push((t, self.cur_ast().ty_spans[p.ty.idx()]));
        }
        // A parent's type names no constructor parameter (`extends outer.Foo(outer)`), as dotty's
        // `checkParentType` refuses it: the parent is constructed before the parameters are the
        // class's fields.
        let params: Vec<SymId> = self.syms.class(c).ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        if !params.is_empty() {
            for &(t, span) in &parents {
                if let Some(&p) = params.iter().find(|&&p| self.types.names_term(t, &[p])) {
                    let msg = format!(
                        "The type of a class parent cannot refer to constructor parameters, but {} refers to {}",
                        self.show(t),
                        self.name_str(self.syms.sym(p).name)
                    );
                    self.error(span, msg);
                }
            }
        }
        if !trait_calls.is_empty() {
            self.inferred_trait_args.insert(c, std::sync::Arc::from(trait_calls));
        }
        if let Some(e) = enum_class {
            // `case A extends E(1)` leaves the type arguments of a generic enum to inference.
            let bare = self.enum_parent_clause(self.env.file, cls, e).filter(|p| {
                !p.args.is_empty()
                    && !matches!(self.cur_ast().ty(p.ty), TyExpr::Apply(..))
                    && !self.syms.class(e).tparams.is_empty()
            });
            if let Some(clause) = bare {
                if let Some((args, ty)) = self.type_enum_parent(c, cls, e, None) {
                    let span = self.cur_ast().ty_spans[clause.ty.idx()];
                    if let Some(slot) = parents.iter_mut().find(|(_, s)| *s == span) {
                        slot.0 = ty;
                    }
                    self.inferred_parent_args.insert(c, args);
                }
            }
            let has_enum_parent = parents.iter().any(|&(t, _)| self.class_of(t) == Some(e));
            if !has_enum_parent {
                let etparams = self.syms.class(e).tparams.clone();
                let args: Vec<TypeId> = if inherited {
                    ids.iter().map(|&(_, p)| self.types.param(p)).collect()
                } else {
                    etparams
                        .iter()
                        .map(|&p| match self.syms.tparam(p).variance {
                            1 => NOTHING,
                            -1 => ANY,
                            _ => {
                                let span = self.syms.class(c).span;
                                self.error(span, "an enum case of an invariant enum needs an explicit extends clause");
                                ERROR
                            }
                        })
                        .collect()
                };
                let t = self.types.class(e, &args);
                parents.insert(0, (t, self.syms.class(c).span));
            }
        }
        self.set_parents(c, parents);
        // The self type may name a class that extends this one (tapir's
        // `EndpointTransputMacros[T] { this: EndpointTransput[T] => }`), so it is resolved once
        // this class is complete enough to be extended.
        if let Some(t) = cls.self_type {
            self.syms.class_mut(c).state().set(Completion::Done);
            let declared = self.resolve_type(t);
            let self_ty = self.syms.class(c).base_types[0].1;
            let mut info = self.syms.class_mut(c);
            info.declared_self = Some(declared);
            info.this_type = Some(self.types.inter(self_ty, declared));
        }
        if self.syms.class(c).value_class {
            self.check_value_class(c, cls);
        }
        if kind == ClassKind::Enum
            && (self.has_evidence_traits(c) || self.inherits_trait_statements(c) || self.syms.class(c).superclass.is_some())
        {
            self.syms.class_mut(c).stateful = true;
        }
        if let Some(sym) = self.syms.class(c).singleton {
            let ty = self.syms.class(c).parents.first().copied().unwrap_or(ERROR);
            let mut s = self.syms.sym_mut(sym);
            s.sig = Some(Arc::new(MethodSig::value(ty)));
            s.state().set(Completion::Done);
        }
        if tparam_asts.iter().any(|tp| tp.arity > 0) {
            self.syms.class_mut(c).state().set(Completion::Done);
            self.resolve_tparam_bounds_of(tparam_asts, &ids, |tp| tp.arity > 0);
        }
    }

    /// The signature of the primary constructor as an alternative next to the secondaries:
    /// the class's own type parameters, which an application instantiates afresh.
    pub(super) fn seal_primary_ctor(&mut self, c: ClassId) {
        let Some(sym) = self.syms.class(c).primary_ctor else { return };
        let info = self.syms.class(c);
        let sig = MethodSig { tparams: info.tparams.clone(), clauses: info.ctor.clone(), ret: info.base_types[0].1 };
        let mut s = self.syms.sym_mut(sym);
        s.sig = Some(Arc::new(sig));
        s.state().set(Completion::Done);
    }

    /// scalac's restrictions on a value class, in its words: one parameter, which is no var,
    /// and a body without fields, nested classes or objects.
    #[cold]
    fn check_value_class(&mut self, c: ClassId, cls: &'a ast::ClassDef) {
        let span = self.syms.class(c).span;
        if self.syms.class(c).mods & mods::ABSTRACT != 0 {
            self.error(span, "Value classes may not be abstract");
        }
        let params: Vec<&ast::Param> = cls.clauses.iter().filter(|cl| !cl.is_using).flat_map(|cl| cl.params.iter()).collect();
        match params.as_slice() {
            [] => self.error(span, "Value class needs one val parameter"),
            [p, ..] => {
                if p.mods & mods::MUTABLE != 0 {
                    self.error(p.span, "A value class parameter may not be a var");
                }
                if let Some(extra) = params.get(1) {
                    self.error(extra.span, "value class can only have one non `erased` parameter");
                }
                let field = self.syms.class(c).ctor.first().and_then(|cl| cl.params.first()).map(|f| f.ty);
                match field.and_then(|t| self.class_of(t)) {
                    Some(k) if k == c => self.error(p.span, "value class cannot wrap itself"),
                    Some(k) if self.syms.class(k).value_class => {
                        self.error(p.span, "value class may not wrap another user-defined value class")
                    }
                    _ => {}
                }
            }
        }
        let ast = self.cur_ast();
        for stmt in &cls.body {
            let ast::Stmt::Def(d) = stmt else { continue };
            let def = ast.def(*d);
            let problem = match &def.kind {
                DefKind::Fun(_) if def.name == names::INIT => Some("Value classes may not define a secondary constructor"),
                DefKind::Val { .. } => Some("Value classes may not define non-parameter field"),
                DefKind::Class(inner) if inner.kind == ast::ClassKind::Object => {
                    Some("Value classes may not define non-parameter field")
                }
                DefKind::Class(_) => Some("Value classes may not define an inner class"),
                _ => None,
            };
            if let Some(msg) = problem {
                self.error(def.span, msg);
            }
        }
    }

    /// The generic class that a parent clause names without type arguments while passing
    /// constructor arguments.
    /// A type prefix that starts at a def taking only using clauses, as `quotes.reflect.Position`
    /// does under scalac through `transparent inline def quotes`: the objects selected from its
    /// result are the same whatever the def returns, so the prefix is the last of them.
    fn def_prefix(&mut self, q: TyExprId, s: SymId) -> Option<TypeId> {
        let is_def = self.syms.sym(s).kind == SymKind::Def;
        let sig = self.sig_of(s);
        if !is_def || !sig.clauses.iter().all(|c| c.is_using) {
            return None;
        }
        let ret = sig.ret;
        let ast = self.cur_ast();
        let mut names = Vec::new();
        let mut cur = q;
        while let TyExpr::Select(inner, n) = ast.ty(cur) {
            names.push(n);
            cur = inner;
        }
        if !matches!(ast.ty(cur), TyExpr::Name(head) if head == self.syms.sym(s).name) {
            return None;
        }
        let mut ty = ret;
        for n in names.into_iter().rev() {
            let (member, _) = self.find_member(ty, n)?;
            let member_ty = self.sig_of(member).ret;
            let member_ty = self.deref(member_ty);
            match self.types.get(member_ty) {
                Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object => ty = member_ty,
                Type::Term(m) => match self.syms.sym(m).kind {
                    SymKind::Object(c) => ty = self.types.class(c, &[]),
                    _ => return None,
                },
                _ => return None,
            }
        }
        matches!(self.types.get(ty), Type::Class(c, _) if self.syms.class(c).kind == ClassKind::Object).then_some(ty)
    }

    /// A generic trait with ordinary parameters named without type arguments in a parent clause
    /// that passes it arguments.
    fn bare_generic_trait(&mut self, p: &ast::Parent) -> Option<ClassId> {
        if p.args.is_empty() || matches!(self.cur_ast().ty(p.ty), TyExpr::Apply(..)) {
            return None;
        }
        let ctor = self.resolve_type_ctor(p.ty);
        let Type::Ctor(parent) = self.types.get(ctor) else { return None };
        self.complete_class(parent);
        let info = self.syms.class(parent);
        let generic = info.kind == ClassKind::Trait && !info.tparams.is_empty() && info.js == JsKind::Scala;
        (generic && !info.ctor.is_empty() && !self.is_evidence_trait(parent)).then_some(parent)
    }

    fn bare_generic_class(&mut self, p: &ast::Parent) -> Option<ClassId> {
        if p.args.is_empty() || matches!(self.cur_ast().ty(p.ty), TyExpr::Apply(..)) {
            return None;
        }
        let ctor = self.resolve_type_ctor(p.ty);
        let Type::Ctor(parent) = self.types.get(ctor) else { return None };
        self.complete_class(parent);
        let info = self.syms.class(parent);
        (info.kind == ClassKind::Class && info.js != JsKind::Native).then_some(parent)
    }

    /// A class case without type parameters copies those of its enum, unless it has an extends
    /// clause and mentions none of them.
    pub(super) fn copied_enum_tparams(&self, case: &ast::ClassDef, e: ClassId) -> Option<&'a [ast::TypeParam]> {
        if !case.tparams.is_empty() || case.clauses.is_empty() {
            return None;
        }
        let info = self.syms.class(e);
        let ast = self.ast(info.file);
        let DefKind::Class(ecls) = &ast.def(info.def?).kind else { return None };
        if ecls.tparams.is_empty() {
            return None;
        }
        let mentioned = |ty: TyExprId| ecls.tparams.iter().any(|tp| ty_mentions(ast, ty, tp.name));
        let copies = case.parents.is_empty()
            || case.parents.iter().any(|p| mentioned(p.ty))
            || case.clauses.iter().any(|cl| cl.params.iter().any(|p| mentioned(p.ty)));
        copies.then_some(&ecls.tparams[..])
    }

    /// Whether an ancestor trait has parameters, using clauses or context bounds for `c` to fill in.
    pub(super) fn has_evidence_traits(&self, c: ClassId) -> bool {
        self.syms.class(c).base_types.iter().skip(1).any(|&(b, _)| self.has_trait_params(b))
    }

    pub(super) fn is_evidence_trait(&self, c: ClassId) -> bool {
        let info = self.syms.class(c);
        info.kind == ClassKind::Trait && !info.ctor.is_empty() && info.ctor.iter().all(|cl| cl.is_using)
    }

    fn body_has_initializers(&self, body: &[ast::Stmt]) -> bool {
        let ast = self.cur_ast();
        body.iter().any(|stmt| match *stmt {
            ast::Stmt::Expr(_) => true,
            ast::Stmt::Def(d) => {
                let def = ast.def(d);
                match &def.kind {
                    DefKind::Val { rhs, .. } => rhs.is_some() && def.mods & mods::LAZY == 0,
                    DefKind::Given(g) => !(g.tparams.is_empty() && g.clauses.is_empty()),
                    _ => false,
                }
            }
            ast::Stmt::Import(_) => false,
        })
    }

    pub(super) fn set_parents(&mut self, c: ClassId, parents: Vec<(TypeId, Span)>) {
        let targs: Vec<TypeId> =
            self.syms.class(c).tparams.clone().iter().map(|&p| self.types.param(p)).collect();
        let self_ty = self.types.class(c, &targs);
        let mut linearisations: Vec<Vec<(ClassId, TypeId)>> = Vec::with_capacity(parents.len());
        let mut parent_types = Vec::new();
        let (file, kind, is_anon) = {
            let i = self.syms.class(c);
            (i.file, i.kind, i.kind == ClassKind::Anon)
        };
        let is_case_of_enum = kind == ClassKind::EnumCase;
        let mut superclass: Option<ClassId> = None;
        let mut trait_superclasses: Vec<(ClassId, ClassId, Span)> = Vec::new();
        for (pt, span) in parents {
            let pt = self.deref_alias(pt);
            // A class nested in a class, through its prefix (`extends H.o.I`, an alias of it): its
            // base types are seen from the prefix, as dotty's `baseType` of a parent `TypeRef`.
            let (class_pt, prefix) = match self.types.get(pt) {
                Type::Nested(p, class) => (class, Some(p)),
                _ => (pt, None),
            };
            let Type::Class(pc, pargs) = self.types.get(class_pt) else {
                if pt == ANY && kind != ClassKind::Trait {
                    self.error(span, "Any does not have a constructor");
                } else if pt != ERROR && pt != ANY {
                    self.error(span, "a parent has to be a class or a trait");
                }
                continue;
            };
            // `AnyRef` as a parent (`new AnyRef { ... }`, `given AnyRef with ...`) adds nothing.
            if pc == self.b.any_ref {
                continue;
            }
            if pc == self.b.any_val {
                if kind == ClassKind::Class && !is_anon {
                    self.syms.class_mut(c).value_class = true;
                } else {
                    self.error(span, "only a class can extend AnyVal");
                }
                continue;
            }
            self.complete_class(pc);
            let pinfo = self.syms.class(pc);
            // Nothing is emitted for a native JS class, so extending one costs no inheritance.
            let is_class = pinfo.kind == ClassKind::Class && pinfo.js != JsKind::Native;
            let allowed = matches!(pinfo.kind, ClassKind::Trait)
                || (pinfo.kind == ClassKind::Enum && is_case_of_enum)
                || pinfo.kind == ClassKind::Class;
            if !allowed {
                self.error(span, "this type cannot be extended");
                continue;
            }
            if pinfo.state() == Completion::InProgress {
                let msg = format!("Cyclic inheritance: {} extends itself", self.class_description(c));
                self.error(span, msg);
                continue;
            }
            if is_class {
                if let Some(problem) = self.class_parent_problem(c, pc, !parent_types.is_empty()) {
                    let at = if problem.1 { self.syms.class(c).span } else { span };
                    self.error(at, problem.0);
                    continue;
                }
                superclass = Some(pc);
            } else if pinfo.kind == ClassKind::Enum && is_case_of_enum && pinfo.superclass.is_some() {
                // The cases of an enum that extends a class are subclasses of the enum, which
                // passes its arguments on to that class.
                superclass = Some(pc);
            } else if let Some(sc) = self.syms.class(pc).superclass {
                trait_superclasses.push((pc, sc, span));
            }
            // The JVM runtime (std/jvm.scala) holds the one class behind every literal.
            if self.is_partial_function(pc) && !(self.jvm && self.source(self.env.file).is_std) {
                let msg = "PartialFunction cannot be extended; write a { case ... } literal, PartialFunction.fromFunction or Function.unlift";
                self.error(span, msg);
                continue;
            }
            let pinfo = self.syms.class(pc);
            let (sealed, pfile) = (pinfo.mods & mods::SEALED != 0, pinfo.file);
            // A subclass may use a protected constructor; a private one stays with its class.
            // With secondary constructors the one the parent call picks is checked there.
            if pinfo.mods & mods::PRIVATE_CTOR != 0 && pinfo.ctors.is_empty() {
                self.check_ctor_access(pc, span);
            }
            // A std class extending a sealed std class of another std file is a representation
            // the runtime gives the parent (`scala.runtime.TupleXXL` of `NonEmptyTuple`), not
            // one of its cases: no match is checked for it, so the children a program sees do
            // not depend on whether that file entered.
            let representation = sealed && pfile != file && self.source(file).is_std && self.source(pfile).is_std;
            if sealed && !representation {
                // The classes of one TASTy file are converted one at a time (`compile.rs`).
                if pfile != file && !self.is_body_file(file) {
                    let msg = format!("Cannot extend sealed {} in a different source file", self.class_description(pc));
                    self.error(span, msg);
                }
                if !is_anon && !self.syms.class(pc).children.contains(&c) {
                    if self.shared_class(pc) {
                        self.with_loader(|w| w.syms.class_mut(pc).children.push(c));
                    } else {
                        self.syms.class_mut(pc).children.push(c);
                    }
                }
            }
            parent_types.push(pt);
            let subst: Subst = self
                .syms
                .class(pc)
                .tparams
                .iter()
                .copied()
                .zip(self.types.items(pargs).iter().copied())
                .collect();
            let inherited: Vec<(ClassId, TypeId)> = self.syms.class(pc).base_types.clone();
            let mut seen: Vec<(ClassId, TypeId)> = inherited.into_iter().map(|(bc, bt)| (bc, self.types.subst(bt, &subst))).collect();
            if let Some(p) = prefix {
                for (_, bt) in seen.iter_mut() {
                    *bt = self.outer_seen_from(*bt, p, pc);
                }
            }
            linearisations.push(seen);
        }
        if !trait_superclasses.is_empty() {
            superclass = self.superclass_with_traits(superclass, &trait_superclasses);
        }
        // The standard library picks the collection type of `SeqOps` through its first parent.
        let checked = linearisations.len() > 1 && !self.source(file).is_std;
        let (base, conflicting) = self.linearise(c, self_ty, linearisations, checked);
        let has_native_base = base.iter().skip(1).any(|&(b, _)| self.syms.class(b).js == JsKind::Native);
        if let Some(parent) = superclass.filter(|_| kind != ClassKind::Trait) {
            if !self.syms.class(parent).subclasses.contains(&c) {
                if self.shared_class(parent) {
                    self.with_loader(|w| w.syms.class_mut(parent).subclasses.push(c));
                } else {
                    self.syms.class_mut(parent).subclasses.push(c);
                }
            }
        }
        let inherits_types = base.iter().skip(1).any(|&(b, _)| {
            let bi = self.syms.class(b);
            !bi.type_aliases.is_empty()
                || bi.inherits_types
                || bi.nested.values().any(|&n| self.is_opaque_path_class(n))
                || (bi.kind != ClassKind::Object && !bi.nested.is_empty())
        });
        let mut info = self.syms.class_mut(c);
        info.parents = parent_types;
        info.base_types = base;
        info.superclass = superclass;
        info.inherits_types = inherits_types;
        if has_native_base && info.js == JsKind::Scala {
            info.js = JsKind::Object;
        }
        if let Some(declared) = info.declared_self {
            info.this_type = Some(self.types.inter(self_ty, declared));
        }
        if info.base_types.len() > 1 {
            self.mark_members_meeting_inherited(c);
            self.check_self_types(c);
        }
        if checked && !conflicting {
            self.check_conflicting_members(c);
        }
    }

    /// scalac's E058: what `this` is in `c` has to conform to the self type each parent
    /// declares, under the type arguments `c` gives it.
    fn check_self_types(&mut self, c: ClassId) {
        let this = self.syms.this_type(c);
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types[1..].to_vec();
        for (b, bt) in bases {
            let Some(declared) = self.syms.class(b).declared_self else { continue };
            let subst = self.owner_subst(bt);
            let required = self.types.subst(declared, &subst);
            // Seen from `c`'s `this`, as dotty's `checkSelfAgainstParents` sees the parent's
            // self type (`asSeenFrom(cls.thisType, parent)`): an outer class's `this` in it is
            // the one `c` is nested in.
            let required = if self.types.has_paths(required) {
                let prefix = self.this_prefix(c);
                self.as_seen_from(required, prefix, c)
            } else {
                required
            };
            let mark = self.snapshot();
            let ok = self.is_sub(this, required);
            self.rollback(mark);
            if !ok {
                let (span, is_anon) = {
                    let i = self.syms.class(c);
                    (i.span, i.kind == ClassKind::Anon)
                };
                // An anonymous class is described by its parents, as scalac describes it.
                let described = if is_anon {
                    let parents: Vec<String> = self.syms.class(c).parents.clone().iter().map(|&p| self.show(p)).collect();
                    format!("Object with {} {{...}}", parents.join(" with "))
                } else {
                    self.name_str(self.syms.class(c).name)
                };
                let owner = if is_anon { format!("anonymous class {}", described) } else { self.class_description(c) };
                let msg = format!(
                    "illegal inheritance: self type {} of {} does not conform to self type {} of parent {}",
                    described,
                    owner,
                    self.show(required),
                    self.class_description(b)
                );
                self.error(span, msg);
                return;
            }
        }
    }

    /// Why `c` cannot have the class `pc` as a parent, and whether the message belongs to `c`
    /// rather than to the parent clause.
    #[cold]
    fn class_parent_problem(&mut self, c: ClassId, pc: ClassId, has_earlier_parent: bool) -> Option<(String, bool)> {
        let (kind, is_case) = {
            let i = self.syms.class(c);
            (i.kind, i.mods & mods::CASE != 0)
        };
        let parent = self.class_description(pc);
        if has_earlier_parent {
            return Some((format!("{} is not a trait", parent), false));
        }
        if kind == ClassKind::EnumCase {
            return Some(("an enum case cannot extend a class".to_string(), false));
        }
        if self.syms.class(pc).mods & mods::FINAL != 0 {
            return Some((format!("{} cannot extend final {}", self.class_description(c), parent), true));
        }
        let case_ancestor = is_case
            .then(|| self.syms.class(pc).base_types.iter().map(|&(b, _)| b).find(|&b| {
                let i = self.syms.class(b);
                i.kind == ClassKind::Class && i.mods & mods::CASE != 0
            }))
            .flatten();
        if let Some(ancestor) = case_ancestor {
            let msg = format!(
                "case {} has case ancestor {}, but case-to-case inheritance is prohibited",
                self.class_description(c),
                self.class_description(ancestor)
            );
            return Some((msg, false));
        }
        None
    }

    /// A trait that extends a class hands that class on as the superclass of whatever mixes the
    /// trait in, and the superclass the class names itself has to derive from it.
    #[cold]
    fn superclass_with_traits(
        &mut self,
        explicit: Option<ClassId>,
        of_traits: &[(ClassId, ClassId, Span)],
    ) -> Option<ClassId> {
        let derives = |t: &Self, sub: ClassId, sup: ClassId| t.syms.class(sub).base_types.iter().any(|&(b, _)| b == sup);
        let mut superclass = explicit;
        for &(tr, sc, span) in of_traits {
            match superclass {
                Some(current) if derives(self, current, sc) => {}
                Some(current) if explicit.is_none() && derives(self, sc, current) => superclass = Some(sc),
                Some(current) => {
                    let msg = format!(
                        "illegal trait inheritance: super{} does not derive from {}'s super{}",
                        self.class_description(current),
                        self.class_description(tr),
                        self.class_description(sc)
                    );
                    self.error(span, msg);
                }
                None => superclass = Some(sc),
            }
        }
        superclass
    }

    /// Flags the methods of `c` whose name an ancestor has a method for. Whether such a member
    /// overrides the inherited one or is overloaded with it takes their signatures, which the
    /// first lookup compares (`Worker::merge_inherited`). The standard library hides an inherited
    /// method by name unless that one is overloaded.
    pub(super) fn mark_members_meeting_inherited(&mut self, c: ClassId) {
        let file = self.syms.class(c).file;
        let is_std = self.source(file).is_std;
        let in_jar = self.in_jar(file);
        let inherits_overloads = self.syms.class(c).base_types[1..].iter().any(|&(b, _)| self.syms.class(b).has_overloads);
        self.syms.class_mut(c).has_overloads |= inherits_overloads;
        for i in 0..self.syms.class(c).member_order.len() {
            let info = self.syms.class(c);
            let m = info.member_order[i];
            let (kind, name) = (self.syms.sym(m).kind, self.syms.sym(m).name);
            // A val meets what it implements only where an ancestor overloads the name: the
            // other alternatives are members of `c` next to it.
            let is_val = matches!(kind, SymKind::Val | SymKind::Var) && inherits_overloads;
            if kind != SymKind::Def && !is_val {
                continue;
            }
            let meets = info.base_types[1..].iter().any(|&(b, _)| {
                self.syms.class(b).members.get(&name).map_or(false, |&p| match self.syms.sym(p).kind {
                    SymKind::Overloaded(_) => true,
                    SymKind::Def => (!is_std || in_jar) && !is_val,
                    _ => false,
                })
            });
            if meets {
                let entry = info.members[&name];
                let mut flags = self.syms.sym_mut(entry);
                flags.meets_inherited = true;
                flags.merge_pending = true;
                if is_std && !in_jar {
                    self.unsettled_std_entries.push((c, entry));
                }
            }
        }
    }

    /// SLS 5.1.2: `L(C) = C, L(Pn) +: ... +: L(P1)`, where a class named on both sides keeps
    /// the place it has on the right. The result is the order in which members are looked up,
    /// and whether one class was inherited at two types.
    pub(super) fn linearise(
        &mut self,
        c: ClassId,
        self_ty: TypeId,
        parents: Vec<Vec<(ClassId, TypeId)>>,
        checked: bool,
    ) -> (Vec<(ClassId, TypeId)>, bool) {
        let mut base: Vec<(ClassId, TypeId)> = vec![(c, self_ty)];
        for lin in parents.iter().rev() {
            base.extend_from_slice(lin);
        }
        // The linearisation of a single parent names no class twice.
        if parents.len() < 2 {
            return (base, false);
        }
        let mut conflict = None;
        let mut i = 1;
        while i < base.len() {
            let (bc, bt) = base[i];
            match base[i + 1..].iter().find(|&&(x, _)| x == bc) {
                Some(&(_, later)) => {
                    if later != bt && conflict.is_none() && self.conflicting_instantiations(bc, bt, later) {
                        conflict = Some((bt, later));
                    }
                    base.remove(i);
                }
                None => i += 1,
            }
        }
        let Some((a, b)) = conflict.filter(|_| checked && self.syms.class(c).kind != ClassKind::Trait) else {
            return (base, false);
        };
        if !self.types.contains_error(a) && !self.types.contains_error(b) {
            let (span, file) = (self.syms.class(c).span, self.syms.class(c).file);
            let msg =
                format!("{} has conflicting base types {} and {}", self.class_description(c), self.show(b), self.show(a));
            self.diags.error(file, span, msg);
        }
        (base, true)
    }

    /// Two instantiations of one generic ancestor conflict where an invariant parameter differs;
    /// the arguments of a variant one would meet in their glb or lub.
    fn conflicting_instantiations(&mut self, c: ClassId, a: TypeId, b: TypeId) -> bool {
        // An inner class through two prefixes that are not one path (`H.a.I` and `H.b.I`):
        // dotty's base type is their intersection, a `HasProblemBase` (`CheckRealizable`).
        let (a, b) = match (self.types.get(a), self.types.get(b)) {
            (Type::Nested(p, x), Type::Nested(q, y)) => {
                if p != q && !self.is_same(p, q) {
                    return true;
                }
                (x, y)
            }
            (Type::Nested(..), _) | (_, Type::Nested(..)) => return true,
            _ => (a, b),
        };
        let (Type::Class(_, aa), Type::Class(_, ba)) = (self.types.get(a), self.types.get(b)) else { return false };
        let tparams = self.syms.class(c).tparams.clone();
        let (aa, ba) = (self.types.items(aa).to_vec(), self.types.items(ba).to_vec());
        tparams
            .iter()
            .zip(aa.iter().zip(&ba))
            .any(|(&p, (&x, &y))| self.syms.tparam(p).variance == 0 && x != y && !self.is_same(x, y))
    }

    /// Two ancestors that define one concrete member: the earlier one in the linearisation has
    /// to override it, and override a member both inherit, unless the class defines it itself.
    /// A pair that a direct parent inherits as well was checked with that parent. Two methods
    /// conflict only if they take the same parameters, which is known once signatures can be
    /// completed (`check_inherited_conflicts`); with different ones the name is overloaded in
    /// `c`, which gets an entry that joins the alternatives of the two ancestors.
    #[cold]
    fn check_conflicting_members(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let direct: Vec<ClassId> = self.syms.class(c).parents.clone().iter().filter_map(|&p| self.class_of(p)).collect();
        let extends = |t: &Self, sub: ClassId, sup: ClassId| t.syms.class(sub).base_types.iter().any(|&(x, _)| x == sup);
        let mut seen: crate::intern::FxMap<Name, ClassId> = Default::default();
        let mut first_method: crate::intern::FxMap<Name, (ClassId, SymId)> = Default::default();
        let mut joined: Vec<SymId> = Vec::new();
        let mut conflicts: Vec<(Name, ClassId, ClassId, bool)> = Vec::new();
        for &b in &bases {
            for m in self.syms.class(b).member_order.clone() {
                let info = self.syms.sym(m);
                // A concrete var's setter goes with its var, which a conflict names (scalac's
                // "other members with override errors" lists the setter beside it).
                if info.mods & mods::PRIVATE != 0 || super::setters::is_concrete_setter(&self.syms, m) {
                    continue;
                }
                let name = info.name;
                if info.kind == SymKind::Def {
                    match first_method.get(&name) {
                        None => {
                            first_method.insert(name, (b, m));
                        }
                        Some(&(first, like)) => {
                            let apart = first != b && !extends(self, first, b);
                            if apart && !self.syms.class(c).members.contains_key(&name) && !joined.contains(&like) {
                                joined.push(like);
                            }
                        }
                    }
                }
                if self.is_abstract_member(m) {
                    continue;
                }
                let Some(&earlier) = seen.get(&name) else {
                    seen.insert(name, b);
                    continue;
                };
                if earlier == b || conflicts.iter().any(|&(n, ..)| n == name) || self.syms.class(c).members.contains_key(&name) {
                    continue;
                }
                if extends(self, earlier, b) || direct.iter().any(|&p| extends(self, p, earlier) && extends(self, p, b)) {
                    continue;
                }
                let winner = self.syms.class(earlier).members[&name];
                let marked = self.syms.sym(winner).mods & mods::OVERRIDE != 0;
                let of_any = matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS);
                let shared = of_any
                    || self.syms.class(earlier).base_types.iter().skip(1).any(|&(x, _)| {
                        extends(self, b, x) && self.syms.class(x).members.contains_key(&name)
                    });
                if !(marked && shared) {
                    conflicts.push((name, earlier, b, marked));
                }
            }
        }
        for like in joined {
            self.add_joined_entry(c, like);
        }
        for (name, first, second, marked) in conflicts {
            let is_method = |t: &Self, k: ClassId| {
                let entry = t.syms.sym(t.syms.class(k).members[&name]);
                matches!(entry.kind, SymKind::Def | SymKind::Overloaded(_))
            };
            // A given whose right-hand side is written `deferred` is abstract if that names the
            // marker, which its signature's completion decides (`Worker::is_deferred_given`).
            let candidate = |t: &Self, k: ClassId| t.deferred_candidate(t.syms.class(k).members[&name]);
            if (is_method(self, first) && is_method(self, second)) || candidate(self, first) || candidate(self, second) {
                self.inherited_pairs.push((c, name, first, second, marked));
            } else {
                self.report_conflicting_members(c, name, first, second, marked);
            }
        }
    }

    /// The pairs of methods, and of givens that may be deferred ones, that
    /// `check_conflicting_members` left open: a conflict if a concrete one of `first` takes the
    /// parameters of a concrete one of `second`.
    pub(super) fn check_inherited_conflicts(&mut self) {
        for (c, name, first, second, marked) in std::mem::take(&mut self.inherited_pairs) {
            let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
            let Some(&(_, second_ty)) = bases.iter().find(|&&(b, _)| b == second) else { continue };
            let mut conflict = false;
            let mut i = 0;
            while let Some(a) = self.own_alternative(first, name, i) {
                i += 1;
                let mut j = 0;
                while let Some(b) = self.own_alternative(second, name, j) {
                    j += 1;
                    conflict |= !self.is_deferred_given(a)
                        && !self.is_abstract_member(a)
                        && !self.is_deferred_given(b)
                        && !self.is_abstract_member(b)
                        && self.same_parameters(c, &bases, a, b, second_ty);
                }
            }
            if conflict {
                self.report_conflicting_members(c, name, first, second, marked);
            }
        }
    }

    fn report_conflicting_members(&mut self, c: ClassId, name: Name, first: ClassId, second: ClassId, marked: bool) {
        let (span, file) = (self.syms.class(c).span, self.syms.class(c).file);
        let msg = if marked {
            format!(
                "{} in {} cannot override the concrete {} in {} without a member that both override",
                self.name_str(name),
                self.class_description(first),
                self.name_str(name),
                self.class_description(second)
            )
        } else {
            format!(
                "{} inherits conflicting members: {} in {} and {} in {}; declare an override in {}",
                self.class_description(c),
                self.name_str(name),
                self.class_description(second),
                self.name_str(name),
                self.class_description(first),
                self.class_description(c)
            )
        };
        self.diags.error(file, span, msg);
    }

    pub(super) fn class_description(&self, c: ClassId) -> String {
        let info = self.syms.class(c);
        let kind = match info.kind {
            ClassKind::Trait => "trait",
            ClassKind::Object => "object",
            ClassKind::Enum => "enum",
            ClassKind::EnumCase => "case",
            ClassKind::Anon => "anonymous class",
            _ => "class",
        };
        format!("{} {}", kind, self.name_str(info.name))
    }
}

enum Bound {
    Frame(usize),
    Pkg(PkgId),
    Import(ResolvedImport),
}

/// A type argument to check against its parameter's bounds once every class is complete.
pub struct DeferredBound {
    arg: TypeId,
    upper: Option<TypeId>,
    lower: Option<TypeId>,
    span: Span,
    file: FileId,
    transparent: Vec<ClassId>,
    /// Whether the type application stands in a body (`Diagnostics::of_bodies` when it was
    /// resolved), as what the check reports then does.
    of_body: bool,
    /// As `DeferredMatch::definition`.
    pub(super) definition: Option<SymId>,
}

impl DeferredBound {
    /// Where the bound was deferred, for the merge's order.
    pub fn site(&self) -> (FileId, u32, u32) {
        (self.file, self.span.start, self.span.end)
    }
}

impl DeferredBound {
    /// The types it holds, for the measurement of what outlives the body phase.
    pub(super) fn types(&self) -> impl Iterator<Item = TypeId> + '_ {
        std::iter::once(self.arg).chain(self.upper).chain(self.lower)
    }

    pub(super) fn remap(&mut self, class: impl Fn(ClassId) -> ClassId, ty: impl Fn(TypeId) -> TypeId) {
        for c in &mut self.transparent {
            *c = class(*c);
        }
        self.arg = ty(self.arg);
        self.upper = self.upper.map(&ty);
        self.lower = self.lower.map(&ty);
    }
}

/// What `Worker::lookup` finds at each stage of the binding precedence.
trait Binding: Copy {
    fn in_frame(t: &mut Worker, frame: usize, innermost_class: bool, name: Name) -> Option<Self>;
    fn via_import(t: &mut Worker, imp: ResolvedImport, name: Name) -> Option<Self>;
    /// Whether the binding is a term's, which an imported extension method can make ambiguous.
    const IS_TERM: bool = false;
    fn in_pkg(t: &mut Worker, p: PkgId, name: Name) -> Option<Self>;
    /// A top-level package, visible from every file; the other members of the empty package are
    /// only visible to files without a package clause, through `in_pkg`.
    fn root_package(t: &mut Worker, name: Name) -> Option<Self>;
    fn same(t: &Worker, a: Self, b: Self) -> bool;
    /// The file of a definition, for the same-compilation-unit rule of package members.
    fn file(t: &Worker, r: Self) -> Option<FileId>;
    /// Whether a binding found in the class frame `frame` is a member the class inherits.
    fn inherited(_t: &Worker, _r: Self, _frame: usize) -> bool {
        false
    }
    /// The package the binding names, if it names one.
    fn package(_r: Self) -> Option<PkgId> {
        None
    }
}

impl Binding for TermRef {
    const IS_TERM: bool = true;

    fn package(r: TermRef) -> Option<PkgId> {
        match r {
            TermRef::Package(p) => Some(p),
            _ => None,
        }
    }

    #[inline]
    fn in_frame(t: &mut Worker, frame: usize, innermost_class: bool, name: Name) -> Option<TermRef> {
        match &t.env.frames[frame] {
            Frame::Locals { names, classes, .. } => names
                .iter()
                .rev()
                .find(|(n, _)| *n == name)
                .map(|&(_, s)| match t.syms.sym(s).kind {
                    SymKind::Object(_) => TermRef::Global(s),
                    _ => TermRef::Local(s),
                })
                .or_else(|| classes.iter().rev().find(|(n, _)| *n == name).map(|&(_, c)| TermRef::Class(c))),
            Frame::Class(c) => {
                let c = *c;
                if t.syms.class(c).self_alias == name {
                    return Some(TermRef::SelfAlias(c));
                }
                let kind = t.syms.class(c).kind;
                if kind == ClassKind::Object {
                    let found = t.module_term(c, name).or_else(|| t.inherited_protected(c, name).map(|s| TermRef::ModuleMember(c, s)));
                    return found.map(|r| match r {
                        TermRef::ModuleMember(m, s) if m == c && innermost_class => TermRef::This(c, s),
                        other => other,
                    });
                }
                if t.parent_args_of == Some(c) || t.sam_classes.contains_key(&c) {
                    return None;
                }
                t.complete_class(c);
                let self_ty = t.syms.this_type(c);
                let bare = std::mem::replace(&mut t.bare_lookup, true);
                let member = t.find_member(self_ty, name);
                t.bare_lookup = bare;
                if let Some((s, _)) = member {
                    return Some(TermRef::This(c, s));
                }
                if let Some(n) = t.syms.class(c).nested.get(&name).copied().or_else(|| t.inherited_nested_class(c, name)) {
                    return Some(TermRef::Class(n));
                }
                if kind == ClassKind::Enum {
                    let companion = t.syms.class(c).companion;
                    if let Some(r) = companion.and_then(|co| t.module_term(co, name)) {
                        return Some(r);
                    }
                }
                if let Some(r) = t.exports_of(c).as_ref().and_then(|e| e.terms.get(&name)).copied() {
                    return Some(r);
                }
                if t.extension_call_head {
                    return t.class_extension_term(c, name);
                }
                None
            }
        }
    }

    #[inline]
    fn via_import(t: &mut Worker, imp: ResolvedImport, name: Name) -> Option<TermRef> {
        match imp.target {
            ImportTarget::PkgMember(p, orig) if imp.name == Some(name) => t.pkg_term(p, orig),
            ImportTarget::PkgAll(p) if imp.name.is_none() => t.pkg_term(p, name).filter(|&r| !t.is_given_ref(r)),
            ImportTarget::PkgGivens(p) => t.pkg_term(p, name).filter(|&r| t.is_given_ref(r)),
            ImportTarget::ClassMember(c, orig) if imp.name == Some(name) => t.module_term(c, orig),
            ImportTarget::ClassAll(c) if imp.name.is_none() => {
                t.module_term(c, name).filter(|&r| !t.is_given_ref(r))
            }
            ImportTarget::ClassGivens(c) => t.module_term(c, name).filter(|&r| t.is_given_ref(r)),
            ImportTarget::ValueMember(..) | ImportTarget::ValueAll(_) | ImportTarget::ValueGivens(_) => t.value_import_term(imp, name),
            _ => None,
        }
    }

    #[inline]
    fn in_pkg(t: &mut Worker, p: PkgId, name: Name) -> Option<TermRef> {
        t.pkg_term(p, name)
    }

    fn root_package(t: &mut Worker, name: Name) -> Option<TermRef> {
        t.demand_pkg(ROOT_PKG, name).map(TermRef::Package)
    }

    fn same(_: &Worker, a: TermRef, b: TermRef) -> bool {
        super::exports::same_term(a, b)
    }

    fn file(t: &Worker, r: TermRef) -> Option<FileId> {
        match r {
            TermRef::Class(c) => Some(t.syms.class(c).file),
            TermRef::Package(_) => None,
            other => other.sym().map(|s| t.syms.sym(s).file),
        }
    }

    fn inherited(t: &Worker, r: TermRef, frame: usize) -> bool {
        let (TermRef::This(k, s) | TermRef::ModuleMember(k, s)) = r else { return false };
        let info = t.syms.sym(s);
        // A val declared with a singleton type is the path it names, which scalac takes for a
        // compatible path rather than another binding.
        let names_a_path = info.def.map_or(false, |d| {
            matches!(&t.ast(info.file).def(d).kind, ast::DefKind::Val { ty: Some(ty), .. } if matches!(t.ast(info.file).ty(*ty), TyExpr::Singleton(_)))
        });
        matches!(t.env.frames.get(frame), Some(Frame::Class(f)) if *f == k)
            && info.owner != Owner::Class(k)
            && !matches!(info.kind, SymKind::Param)
            && !names_a_path
    }
}

impl Binding for TypeRef {
    #[inline]
    fn in_frame(t: &mut Worker, frame: usize, _: bool, name: Name) -> Option<TypeRef> {
        match &t.env.frames[frame] {
            Frame::Locals { tparams, classes, aliases, .. } => classes
                .iter()
                .rev()
                .find(|(n, _)| *n == name)
                .map(|&(_, c)| TypeRef::Class(c))
                .or_else(|| aliases.iter().rev().find(|(n, _)| *n == name).map(|&(_, a)| TypeRef::Alias(a)))
                .or_else(|| tparams.iter().rev().find(|(n, _)| *n == name).map(|&(_, p)| TypeRef::Param(p))),
            Frame::Class(c) => {
                let c = *c;
                t.class_scope_type(c, name)
            }
        }
    }

    #[inline]
    fn via_import(t: &mut Worker, imp: ResolvedImport, name: Name) -> Option<TypeRef> {
        match imp.target {
            ImportTarget::PkgMember(p, orig) if imp.name == Some(name) => t.pkg_type(p, orig),
            ImportTarget::PkgAll(p) if imp.name.is_none() => t.pkg_type(p, name),
            ImportTarget::ClassMember(c, orig) if imp.name == Some(name) => t.module_type(c, orig),
            ImportTarget::ClassAll(c) if imp.name.is_none() => t.module_type(c, name),
            ImportTarget::ValueMember(..) | ImportTarget::ValueAll(_) => t.value_import_type(imp, name),
            _ => None,
        }
    }

    #[inline]
    fn in_pkg(t: &mut Worker, p: PkgId, name: Name) -> Option<TypeRef> {
        t.pkg_type(p, name)
    }

    fn root_package(_: &mut Worker, _: Name) -> Option<TypeRef> {
        None
    }

    fn same(_: &Worker, a: TypeRef, b: TypeRef) -> bool {
        a == b
    }

    fn file(t: &Worker, r: TypeRef) -> Option<FileId> {
        match r {
            TypeRef::Class(c) => Some(t.syms.class(c).file),
            TypeRef::Alias(a) => Some(t.syms.aliases[a.idx()].file),
            TypeRef::Member(c, _) => Some(t.syms.class(c).file),
            TypeRef::ValueMember(v, _) => Some(t.syms.sym(t.import_values[v.0 as usize].0).file),
            TypeRef::Param(_) => None,
        }
    }
}

fn ty_mentions(ast: &ast::Ast, ty: TyExprId, name: Name) -> bool {
    let any = |l: ast::ListRef| ast.ty_list(l).iter().any(|&t| ty_mentions(ast, t, name));
    match ast.ty(ty) {
        TyExpr::Name(n) => n == name,
        TyExpr::Apply(f, args) => ty_mentions(ast, f, name) || any(args),
        TyExpr::Fun(params, ret) | TyExpr::CtxFun(params, ret) => any(params) || ty_mentions(ast, ret, name),
        TyExpr::Tuple(items) | TyExpr::NamedTuple(_, items) => any(items),
        TyExpr::Union(a, b) | TyExpr::Inter(a, b) | TyExpr::BoundedWildcard(a, b) => ty_mentions(ast, a, name) || ty_mentions(ast, b, name),
        TyExpr::ByName(t) | TyExpr::Repeated(t) | TyExpr::Unchecked(t) | TyExpr::UncheckedVariance(t) | TyExpr::Lambda(_, t) | TyExpr::PolyFun(_, t) => {
            ty_mentions(ast, t, name)
        }
        TyExpr::Project(t, _) | TyExpr::Refined(t, _) => ty_mentions(ast, t, name),
        TyExpr::Match(s, cases) => ty_mentions(ast, s, name) || any(cases),
        TyExpr::MatchCase(p, b) => ty_mentions(ast, p, name) || ty_mentions(ast, b, name),
        TyExpr::Select(..) | TyExpr::Singleton(_) | TyExpr::Wildcard | TyExpr::Lit(_) | TyExpr::TypeVar(_) | TyExpr::Resolved(_) | TyExpr::Error => {
            false
        }
    }
}

/// `TEQ_ALIAS_ORDER=<member>`, a test's switch (`tests/workers/alias_in_progress`): orders the
/// workers on the abstract type member of that name. The first worker to ask for it completes
/// it and holds it in progress until another worker has met it so; every other worker asking
/// while it is unstarted waits until it is claimed. Read once per process, and only where the
/// member is not done.
struct AliasOrder {
    name: String,
    first: std::sync::atomic::AtomicBool,
    met: std::sync::atomic::AtomicBool,
}

fn alias_order() -> Option<&'static AliasOrder> {
    static ORDER: std::sync::OnceLock<Option<AliasOrder>> = std::sync::OnceLock::new();
    ORDER
        .get_or_init(|| {
            let name = std::env::var("TEQ_ALIAS_ORDER").ok().filter(|n| !n.is_empty())?;
            Some(AliasOrder { name, first: false.into(), met: false.into() })
        })
        .as_ref()
}

/// A wait of `TEQ_ALIAS_ORDER`'s until `done`, bounded at 2 s.
fn ordered_wait(done: impl Fn() -> bool) {
    for _ in 0..2000 {
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
