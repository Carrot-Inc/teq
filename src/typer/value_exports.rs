//! Export clauses whose qualifier is a value: `export info.{ply, color}`, `export chess.*`, `export
//! env.net.baseUrl`. An object's or a package's export is an alias (`exports`), which a reference
//! resolves to the original; a value's member is reached through the value, so such a clause makes
//! forwarders, as dotty's `Namer.exportForwarders` does: final members of the class, a method's
//! `final def m[T](a: A): R = path.m[T](a)` and a value's `final def v: V = path.v`, with the
//! member's signature as the path sees it. They are made once every class of the program is
//! complete (the signature phase's end, before the bodies fork), and their bodies when the class
//! is checked, so that every backend and the interpreter find them as ordinary methods, an
//! abstract member they implement included.

use super::{ValueImport, Worker};
use crate::ast::{mods, DefKind, Import, ImportSel};
use crate::intern::Name;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::{TExpr, TExprId, TFun};
use crate::types::*;
use std::sync::Arc;

/// Where a clause's forwarders go: the members of a class, or the top-level definitions of a file,
/// the members of its `<source>$package` (dotty's `Desugar.packageDef` moves a top-level `Export`
/// there), which teq enters as the package's terms of that file as it enters the file's own.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Home {
    Class(ClassId),
    File(PkgId, FileId),
}

/// The selector a member's forwarders come from, which picks its alternatives as dotty's do: a name
/// all of its alternatives (`addForwardersNamed`), a wildcard the ones that are no givens
/// (`addWildcardForwardersNamed`, `excluded = Private|Given`), `given` the givens alone
/// (`addWildcardForwarders`).
#[derive(Clone, Copy, PartialEq)]
enum Selected {
    Named,
    Wildcard,
    Givens,
}

/// The qualifier of a value export: the value it is typed as and its path type (`C.this.info`,
/// `O.v`, `Outer.this.colls.api`).
pub(super) struct ValuePath {
    value: ValueImport,
    path: TypeId,
    /// Whether the qualifier is an object, whose term members alone the clause forwards: its
    /// types, objects and classes, its inline members and the names only its own exports give
    /// stay its class's export table's.
    object: bool,
    /// The package a clause names as its qualifier (`export lib.internal.{Version, show}`), whose
    /// members are its classes, its aliases and its files' top-level definitions, named
    /// statically: no value (`Forward::STATIC`), `path` unread.
    pkg: Option<PkgId>,
}

/// The qualifier of an export clause that is no value: an object or a package.
#[derive(Clone, Copy)]
pub(super) enum StaticPath {
    Object(ClassId),
    Package(PkgId),
}

/// The case class an export's type forwarder `a` names whose companion is synthetic, which
/// scalac's export forwards as a term too, `final def N: q.N.type` (`Namer.addForwarder` of the
/// companion module, `Exported | Final | StableRealizable`): teq's typer has no symbol for such a
/// companion, the class standing for it (`TermRef::Class`, `Worker::exported_alias_class`), so the
/// pickle and the JVM write this forwarder from the alias. A plain class's companion is a
/// constructor proxy, of no definition (`NamerOps.typeConstructorCompanion`).
pub fn exported_companion(syms: &Symbols, types: &TypeStore, a: crate::types::AliasId) -> Option<ClassId> {
    let info = &syms.aliases[a.idx()];
    if !info.exported || info.def.is_some() {
        return None;
    }
    // Through the forwarders it relays, `export lib.Version` of lib's own forwarder of the class.
    let mut rhs = info.rhs;
    let mut hops = 0;
    let k = loop {
        hops += 1;
        if hops > 16 {
            return None;
        }
        match types.get(rhs) {
            Type::Class(k, _) | Type::Ctor(k) => break k,
            Type::Alias(b, args) if types.items(args).is_empty() && syms.aliases[b.idx()].exported => rhs = syms.aliases[b.idx()].rhs,
            Type::Lambda(_, body) => match types.get(body) {
                Type::Class(k, _) => break k,
                Type::Alias(b, _) if syms.aliases[b.idx()].exported => rhs = syms.aliases[b.idx()].rhs,
                _ => return None,
            },
            _ => return None,
        }
    };
    let class = syms.class(k);
    let case = class.mods & mods::CASE != 0 && class.kind == ClassKind::Class || class.kind == ClassKind::EnumCase && class.singleton.is_none();
    (case && class.companion.is_none()).then_some(k)
}

impl<'a> Worker<'a> {
    /// The export clauses of a class's definition.
    fn export_clauses(&self, c: ClassId) -> Vec<Import> {
        let info = self.syms.class(c);
        let Some(d) = info.def else { return Vec::new() };
        let ast = self.ast(info.file);
        let DefKind::Class(cls) = &ast.def(d).kind else { return Vec::new() };
        ast.exports[cls.exports.start as usize..(cls.exports.start + cls.exports.len) as usize].to_vec()
    }

    /// The qualifier of an export clause that goes through a value, typed as a path where the
    /// clause stands (dotty's `Namer.exportForwarders`, `typedAheadExpr(expr, AnySelectionProto)`
    /// in the clause's context, with the import qualifier's resolution): a member of the class or
    /// of an enclosing class (`info`, `colls`), a val of an object or a package (`Config.default`),
    /// a local val, an imported value's member, then the stable vals and objects selected on it.
    /// Its errors are reported; `None` for a path that is no value's.
    pub(super) fn value_export_path(&mut self, path: &[Name], span: Span) -> Option<ValuePath> {
        match self.resolve_qualifier(path, span, true)? {
            super::ImportTarget::ValueAll(value) => Some(ValuePath { value, path: self.import_value_type(value), object: false, pkg: None }),
            _ => None,
        }
    }

    /// The object or the package an export clause names as its qualifier, a Scala object of the
    /// program or a library (not a Java class's statics or a value), decided on the path's
    /// symbols as `export_path_through_value` decides.
    pub(super) fn export_static_path(&mut self, path: &[Name]) -> Option<StaticPath> {
        let (&first, rest) = path.split_first()?;
        let mut target = match self.lookup_term(first)? {
            super::TermRef::Package(p) => super::ImportTarget::PkgAll(p),
            r => match self.syms.sym(r.sym()?).kind {
                SymKind::Object(o) => super::ImportTarget::ClassAll(o),
                _ => return None,
            },
        };
        for &seg in rest {
            target = match target {
                super::ImportTarget::PkgAll(p) => self.import_step_pkg(p, seg)?,
                super::ImportTarget::ClassAll(o) => self.import_step_class(o, seg)?,
                _ => return None,
            };
        }
        let o = match target {
            super::ImportTarget::ClassAll(o) => o,
            super::ImportTarget::PkgAll(p) => return Some(StaticPath::Package(p)),
            _ => return None,
        };
        let info = self.syms.class(o);
        let module = info.module_sym?;
        (info.kind == ClassKind::Object && !self.syms.sym(module).java_defined).then_some(StaticPath::Object(o))
    }

    /// Whether an export clause's qualifier is a case class's companion that teq leaves
    /// synthetic: the path's last name, through the objects and packages before it, is a case
    /// class without a companion object, the term `module_term` and `pkg_term` give as the class.
    fn synthetic_companion_path(&mut self, path: &[Name]) -> bool {
        let Some((&last, init)) = path.split_last() else { return false };
        let found = match init {
            [] => self.lookup_term(last),
            _ => match self.export_static_path(init) {
                Some(StaticPath::Object(o)) => self.module_term(o, last),
                Some(StaticPath::Package(p)) => self.pkg_term(p, last),
                None => None,
            },
        };
        let Some(super::TermRef::Class(k)) = found else { return false };
        let info = self.syms.class(k);
        info.kind == ClassKind::Class && info.mods & mods::CASE != 0 && info.companion.is_none()
    }

    /// Whether a value path reads a private val on its way (dotty's `refersToPrivate`); a
    /// package's path reads none.
    fn path_refers_to_private(&self, v: ValueImport) -> bool {
        if v.0 == Forward::STATIC {
            return false;
        }
        let mut at = Some(v);
        while let Some(x) = at {
            let (val, _, prev) = self.import_values[x.0 as usize];
            if self.syms.sym(val).mods & mods::PRIVATE != 0 {
                return true;
            }
            at = prev;
        }
        false
    }

    /// The path of an object as a qualifier: its module val, read as a stable path is. A static
    /// object's path is the object, teq's type of its module's `TermRef` (dotty's `TermRef(p, O)`,
    /// the prefix `pathType.select(sym)` keeps; `object_member_path`): a type forwarder's prefix
    /// is then the module where a constructor's outer argument is made of it (`prefix_value`)
    /// and where it is pickled (`TERMREF O p`), not a top-level val of the file (a `Term` of the
    /// module val, read as `F$package.O` and called as `O()`). An object of a class's instance
    /// keeps its val's path.
    fn object_value_path(&mut self, o: ClassId) -> Option<ValuePath> {
        let m = self.syms.class(o).module_sym?;
        let value = self.with_loader(|w| ValueImport(w.import_values.push((m, None, None)) as u32));
        let path = if self.static_object(o) { self.types.class(o, &[]) } else { self.import_value_type(value) };
        Some(ValuePath { value, path, object: true, pkg: None })
    }

    /// The path of package `p`'s package object, whose members are the package's.
    fn package_object_path(&mut self, p: PkgId) -> Option<ValuePath> {
        let obj = self.syms.pkg(p).package_object?;
        self.object_value_path(obj)
    }

    /// The path of a package as a qualifier: its members named statically.
    fn package_value_path(&mut self, p: PkgId) -> ValuePath {
        ValuePath { value: ValueImport(Forward::STATIC), path: crate::types::ANY, object: false, pkg: Some(p) }
    }

    /// Whether an export clause goes through a value, the clauses `value_export_path` resolves and
    /// the export table leaves: its head looked up where the clause stands is a value, or a path
    /// of objects and packages reaches a val. Decided on the path's symbols, without a signature,
    /// so that the table a type lookup builds while a val's type is inferred infers no val (`export
    /// foo.pin.*` beside `val foo = new Foo: ...`, as dotty's class completer sees `Foo` before the
    /// forwarders).
    pub(super) fn export_path_through_value(&mut self, path: &[Name]) -> bool {
        let Some((&first, rest)) = path.split_first() else { return false };
        let Some(head) = self.lookup_term(first) else { return false };
        // A package or an object opens; any other head is a value, the module val of an object
        // nested in a class or a block among them (`Foo.this.MyEnum`), as the qualifier's
        // resolution reads an export's head (`resolve_qualifier`).
        let (mut module, mut pkg): (Option<ClassId>, Option<PkgId>) = match head {
            super::TermRef::Package(p) => (None, Some(p)),
            r => match r.sym().map(|s| self.syms.sym(s).kind) {
                Some(SymKind::Object(o)) => (Some(o), None),
                Some(SymKind::Overloaded(_)) | None => return false,
                Some(_) => return true,
            },
        };
        for &seg in rest {
            let next = match (module, pkg) {
                (Some(o), _) => self.import_step_class(o, seg),
                (None, Some(p)) => self.import_step_pkg(p, seg),
                _ => None,
            };
            match next {
                Some(super::ImportTarget::ClassAll(o)) => (module, pkg) = (Some(o), None),
                Some(super::ImportTarget::PkgAll(p)) => (module, pkg) = (None, Some(p)),
                _ => {
                    let found = match (module, pkg) {
                        (Some(o), _) => self.module_term(o, seg),
                        (None, Some(p)) => self.pkg_term(p, seg),
                        _ => None,
                    };
                    return found.and_then(|r| r.sym()).is_some_and(|s| !matches!(self.syms.sym(s).kind, SymKind::Object(_) | SymKind::Overloaded(_)));
                }
            }
        }
        false
    }

    /// The forwarders of every program class's value exports, and of every file's top-level
    /// ones, at the end of the signature phase. Each owner's are made once, where they are first
    /// needed: an owner whose clause's path reads another owner's forwarders completes that one
    /// first (`complete_class_forwarders`, `complete_file_forwarders`), as a lookup completes the
    /// owner whose exports `Namer.processExports` enters, whatever order the owners stand in.
    pub fn add_value_forwarders(&mut self) {
        for i in 0..self.syms.classes.len() as u32 {
            self.complete_class_forwarders(ClassId(i));
        }
        for i in 0..self.syms.pkgs.len() as u32 {
            let p = PkgId(i);
            let files: Vec<FileId> = self.syms.pkg(p).export_files.clone();
            for file in files {
                self.complete_file_forwarders(p, file);
            }
        }
        self.value_forwarders_made = true;
        self.forget_tables_built_before_forwarders();
    }

    /// The forwarders of a program class's export clauses, made once, its parents' first. The
    /// top-level forwarders a clause's path names are a package's, made where the lookup reads its
    /// entries (`complete_package_forwarders`), as dotty completes a package object a lookup
    /// reaches. A class in the middle of making its own is a cycle, which finds none of them
    /// (dotty's cyclic reference).
    pub(super) fn complete_class_forwarders(&mut self, c: ClassId) {
        // A class with no export clause, the most, is passed over first, by what the namer read
        // of its body when it entered it (`ClassInfo::has_exports`).
        if !self.syms.class(c).has_exports || self.syms.class(c).forwarders_made || self.forwarders_completion.contains_key(&(u32::MAX, c.0)) {
            return;
        }
        let file = self.syms.class(c).file;
        if self.in_jar(file) || self.std.slot_of(file).is_some() {
            return;
        }
        self.forwarders_completion.insert((u32::MAX, c.0), false);
        self.syms.class_mut(c).forwarders_made = true;
        // The parents' forwarders first, members the class inherits, which a clause's path may
        // name (`export Colors.red` beside a parent's `export lib.internal.Colors`), as the
        // class completer completes the parents it types before the class's exports.
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).filter(|&b| b != c).collect();
        for b in bases {
            self.complete_class_forwarders(b);
        }
        let clauses = self.export_clauses(c);
        if !clauses.is_empty() {
            self.class_value_forwarders(c, &clauses);
            self.mark_forwarders_meeting_inherited(c);
            self.forwarders_entered(c);
        }
        self.forwarders_completion.insert((u32::MAX, c.0), true);
    }

    /// What the classes deriving from `c` that completed before its forwarders read of their
    /// bases' declarations (`inherits_types`, `no_inherited_extensions`), now that `c` declares
    /// them: dotty's `invalidateMemberCaches` of a class whose declarations grew.
    pub(super) fn forwarders_entered(&mut self, c: ClassId) {
        let (types, extensions) = {
            let info = self.syms.class(c);
            (!info.type_aliases.is_empty() || info.inherits_types, !info.extensions.is_empty())
        };
        if !types && !extensions {
            return;
        }
        for i in 0..self.syms.classes.len() as u32 {
            let k = ClassId(i);
            if k == c || self.syms.class(k).state() != Completion::Done || !self.syms.class(k).base_types.iter().any(|&(b, _)| b == c) {
                continue;
            }
            let mut info = self.syms.class_mut(k);
            info.inherits_types |= types;
            if extensions {
                info.no_inherited_extensions = false;
            }
        }
    }

    /// The top-level forwarders of the files of package `p`, during the signature phase, before a
    /// lookup reads its entries: they are top-level definitions of the files.
    #[inline]
    pub(super) fn complete_package_forwarders(&mut self, p: PkgId, name: Name) {
        if self.forwarders_on_completion && !self.value_forwarders_made && !self.syms.pkg(p).forwarders_done && !self.syms.pkg(p).export_files.is_empty() {
            self.complete_package_forwarders_now(p, name);
        }
    }

    /// The forwarders of the package's files whose clauses may forward `name`: dotty's lookup of a
    /// name in a package reads the package objects' members of that name alone
    /// (`PackageClassDenotation.computeMembersNamed`), so a lookup of another name makes no
    /// forwarder whose member's type its inference would read (a class's parameter type looked up
    /// while the class completes, the forwarded member's body creating that class).
    #[inline(never)]
    fn complete_package_forwarders_now(&mut self, p: PkgId, name: Name) {
        if self.probing_forward_names {
            return;
        }
        let files: Vec<FileId> = self.syms.pkg(p).export_files.clone();
        for &f in &files {
            if self.forwarders_completion.contains_key(&(p.0, f.0)) || !self.file_may_forward(p, f, name) {
                continue;
            }
            self.complete_file_forwarders(p, f);
        }
        // Once every file's are made, the package's mark answers.
        if files.iter().all(|f| self.forwarders_completion.get(&(p.0, f.0)) == Some(&true)) {
            self.syms.pkgs[p.idx()].forwarders_done = true;
        }
    }

    /// Whether a top-level export clause of `file` may forward `name`: a selector naming it, or a
    /// wildcard whose qualifier, an object or a package, has a forwardable member of the name; a
    /// value's path may forward any. Computed once per file, its qualifiers resolved with no
    /// package's forwarders made and no report kept.
    fn file_may_forward(&mut self, p: PkgId, file: FileId, name: Name) -> bool {
        if !self.forward_names.contains_key(&(p.0, file.0)) {
            let names = self.file_forward_names(p, file);
            self.forward_names.insert((p.0, file.0), names);
        }
        match &self.forward_names[&(p.0, file.0)] {
            Some(names) => names.contains(&name),
            None => true,
        }
    }

    #[cold]
    #[inline(never)]
    fn file_forward_names(&mut self, p: PkgId, file: FileId) -> Option<Vec<Name>> {
        let clauses: Vec<Import> = self.ast(file).top_exports.clone();
        let outer = std::mem::replace(&mut self.probing_forward_names, true);
        let mark = self.diags.items.len();
        let mut names: Vec<Name> = Vec::new();
        let mut any = false;
        for clause in Self::split_clauses(&clauses) {
            let mut wildcard = false;
            for imp in &clause {
                match imp.sel {
                    ImportSel::Name(n, rename) => {
                        names.push(n);
                        names.extend(rename);
                    }
                    _ => wildcard = true,
                }
            }
            if !wildcard || clause[0].path.is_empty() {
                continue;
            }
            let env = self.env_for(file, Owner::Package(p));
            let vp = self.with_env(env, |t| {
                if t.export_scope(|t| t.export_path_through_value(&clause[0].path)) {
                    return None;
                }
                match t.export_scope(|t| t.export_static_path(&clause[0].path)) {
                    Some(StaticPath::Object(o)) => t.object_value_path(o),
                    Some(StaticPath::Package(q)) => Some(t.package_value_path(q)),
                    None => None,
                }
            });
            match vp {
                // Givens, which a search reads without a lookup by name, are made on any lookup.
                Some(vp) if clause.iter().any(|imp| matches!(imp.sel, ImportSel::Given)) || !self.forwardable_names(&vp, true).is_empty() => any = true,
                Some(vp) => {
                    names.extend(self.forwardable_names(&vp, false));
                    names.extend(self.forwardable_type_names(&vp));
                }
                None => any = true,
            }
        }
        self.diags.items.truncate(mark);
        self.probing_forward_names = outer;
        (!any).then_some(names)
    }

    /// The top-level forwarders of a file, made once, the other files' of its package first.
    fn complete_file_forwarders(&mut self, p: PkgId, file: FileId) {
        if self.forwarders_completion.contains_key(&(p.0, file.0)) || self.in_jar(file) || self.std.slot_of(file).is_some() {
            return;
        }
        // A file whose imports are being resolved, a lookup of their qualifiers reaching its
        // package, is passed over, its forwarders made where a later lookup reaches it: as
        // dotty's `computeMembersNamed` passes over a package object under completion, whose
        // clauses' paths read those imports.
        if self.importing_files.contains(&file) {
            return;
        }
        self.forwarders_completion.insert((p.0, file.0), false);
        let others: Vec<FileId> = self.syms.pkg(p).export_files.iter().copied().filter(|&f| f != file).collect();
        for f in others {
            self.complete_file_forwarders(p, f);
        }
        self.top_value_forwarders(p, file);
        self.forwarders_completion.insert((p.0, file.0), true);
    }

    /// The forwarders of the class a path's type has and of its ancestors, during the signature
    /// phase's pass: a member looked up on the path may be one.
    pub(super) fn complete_forwarders_of_type(&mut self, t: TypeId) {
        if self.value_forwarders_made {
            return;
        }
        let t = self.widen_path(t);
        let Some(k) = self.class_of(t) else { return };
        let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
        for b in bases {
            self.complete_class_forwarders(b);
        }
    }

    /// The forwarders of a class completed after the signature phase's pass, a local or an
    /// anonymous class, made where its completion ends, as dotty's class completer runs
    /// `Namer.processExports` for every class: its clauses typed where they stand, the block's
    /// values in scope.
    pub(super) fn late_value_forwarders(&mut self, c: ClassId) {
        self.class_value_forwarders_and_table(c);
    }

    /// The forwarders of a class's export clauses, and its table, where one was built before them,
    /// left without the names they take.
    fn class_value_forwarders_and_table(&mut self, c: ClassId) {
        if self.syms.class(c).forwarders_made {
            return;
        }
        self.syms.class_mut(c).forwarders_made = true;
        let file = self.syms.class(c).file;
        if self.in_jar(file) || self.std.slot_of(file).is_some() {
            return;
        }
        let clauses = self.export_clauses(c);
        if clauses.is_empty() {
            return;
        }
        self.class_value_forwarders(c, &clauses);
        self.mark_forwarders_meeting_inherited(c);
        self.drop_forwarded_from_built(c);
    }

    /// A forwarder of a name an ancestor defines is an alternative beside the inherited ones, as
    /// a written member is (`mark_members_meeting_inherited`): `f(Int)` exported next to an
    /// inherited `f(String)` overloads it (dotty's `canForward` refuses only what overrides).
    fn mark_forwarders_meeting_inherited(&mut self, c: ClassId) {
        let names: Vec<Name> = self.syms.class(c).member_order.iter().filter(|&&m| self.syms.is_forwarder(m)).map(|&m| self.syms.sym(m).name).collect();
        for name in names {
            let meets = self.syms.class(c).base_types[1..].iter().any(|&(b, _)| {
                self.syms.class(b).members.get(&name).is_some_and(|&p| matches!(self.syms.sym(p).kind, SymKind::Overloaded(_) | SymKind::Def))
            });
            if meets {
                let entry = self.syms.class(c).members[&name];
                let mut flags = self.syms.sym_mut(entry);
                flags.meets_inherited = true;
                flags.merge_pending = true;
            }
        }
    }

    fn class_value_forwarders(&mut self, c: ClassId, clauses: &[Import]) {
        self.clauses_in_order(Home::Class(c), Self::split_clauses(clauses));
    }

    /// The clauses of an owner's selectors: a clause's selectors share its path and the start
    /// of its span.
    fn split_clauses(clauses: &[Import]) -> Vec<Vec<Import>> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < clauses.len() {
            let start = clauses[i].span.start;
            let mut j = i;
            while j < clauses.len() && clauses[j].span.start == start && clauses[j].path == clauses[i].path {
                j += 1;
            }
            out.push(clauses[i..j].to_vec());
            i = j;
        }
        out
    }

    /// An owner's clauses in their order, each typed where it stands, as `processExports` takes
    /// them: one that waits for a class's completion (`clause_value_forwarders`) holds the
    /// ones after it, whose paths may name its forwarders, until that completion ends.
    fn clauses_in_order(&mut self, home: Home, clauses: Vec<Vec<Import>>) {
        for (i, clause) in clauses.iter().enumerate() {
            if let Some(k) = self.clause_where_it_stands(home, clause) {
                self.blocked_clauses.push((k, home, clauses[i..].to_vec()));
                return;
            }
        }
    }

    /// A clause's forwarders, the clause typed where it stands: its class's frames and its outer
    /// classes', or its file's top level, and the imports before it, from which its qualifier
    /// and its members' access are read.
    fn clause_where_it_stands(&mut self, home: Home, clause: &[Import]) -> Option<ClassId> {
        let start = clause[0].span.start;
        let (file, owner) = match home {
            Home::Class(c) => (self.syms.class(c).file, Owner::Class(c)),
            Home::File(p, f) => (f, Owner::Package(p)),
        };
        let env = self.export_scope(|t| t.env_at(file, owner, start));
        self.with_env(env, |t| t.clause_value_forwarders(home, clause))
    }

    /// The clauses that waited for class `c`'s completion, which has ended.
    pub(super) fn blocked_clauses_of(&mut self, c: ClassId) {
        let mut i = 0;
        while i < self.blocked_clauses.len() {
            if self.blocked_clauses[i].0 != c {
                i += 1;
                continue;
            }
            let (_, home, clauses) = self.blocked_clauses.remove(i);
            self.clauses_in_order(home, clauses);
            if let Home::Class(h) = home {
                self.forwarders_entered(h);
            }
        }
    }

    /// The forwarders of a file's top-level export clauses through a value, each clause typed
    /// where it stands (the file's imports before it).
    fn top_value_forwarders(&mut self, p: PkgId, file: FileId) {
        let clauses: Vec<Import> = self.ast(file).top_exports.clone();
        self.clauses_in_order(Home::File(p, file), Self::split_clauses(&clauses));
    }

    /// `f` with the program's class export tables not built yet read as none
    /// (`Worker::making_value_forwarders`): the lookups that make a clause's environment (its
    /// class's imports) and decide its head, which would otherwise build a table inside the
    /// resolution of the imports that table reads.
    fn export_scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let outer = std::mem::replace(&mut self.making_value_forwarders, true);
        let r = f(self);
        self.making_value_forwarders = outer;
        r
    }

    /// The names of a clause in the index, as an import's are (`index_import`): the object or
    /// the package its path names, and the member each selector names. A value's path is not
    /// indexed, as the export tables' never were.
    #[cold]
    #[inline(never)]
    fn index_export_clause(&mut self, home: Home, vp: &ValuePath, clause: &[Import]) {
        let target = match vp.pkg {
            Some(p) => super::ImportTarget::PkgAll(p),
            None if vp.object => {
                let q = self.widen_path(vp.path);
                match self.class_of(q) {
                    Some(c) => super::ImportTarget::ClassAll(c),
                    None => return,
                }
            }
            None => return,
        };
        let file = self.home_file(home);
        for imp in clause {
            self.index_import_in(file, imp, target);
        }
    }

    fn home_file(&self, home: Home) -> FileId {
        match home {
            Home::Class(c) => self.syms.class(c).file,
            Home::File(_, f) => f,
        }
    }

    fn home_owner(&self, home: Home) -> Owner {
        match home {
            Home::Class(c) => Owner::Class(c),
            Home::File(p, _) => Owner::Package(p),
        }
    }

    /// The class whose members the forwarders join, as `canForward` reads it: a file's
    /// `<source>$package` is an object of `AnyRef`, which is all it has.
    fn home_class(&self, home: Home) -> ClassId {
        match home {
            Home::Class(c) => c,
            Home::File(..) => self.b.any_ref,
        }
    }

    /// The name of the class the forwarders join, as dotty's messages give it.
    /// The home as scalac's messages locate it (`class B`, `object O`, `object F$package`).
    fn home_description(&self, home: Home) -> String {
        match home {
            Home::Class(c) => self.class_description(c),
            Home::File(..) => format!("object {}", self.home_name(home)),
        }
    }

    fn home_name(&self, home: Home) -> String {
        match home {
            Home::Class(c) => self.name_str(self.syms.class(c).name).to_string(),
            Home::File(_, f) => format!("{}$package", crate::tasty::write::file_stem(&self.files.as_slice()[f.0 as usize].path)),
        }
    }

    /// The forwarders of one clause, its selectors sharing its path; the class its path's class
    /// is where that class's completion is under way on this thread, the clause made where it ends.
    fn clause_value_forwarders(&mut self, home: Home, clause: &[Import]) -> Option<ClassId> {
        // dotty's "Export selector must have prefix and `.`" (`exportForwarders`).
        if clause[0].path.is_empty() {
            let file = self.home_file(home);
            self.diags.error(file, clause[0].span, "an export needs a qualifier, as in `export obj.member`".to_string());
            return None;
        }
        let through_value = self.export_scope(|t| t.export_path_through_value(&clause[0].path));
        // A class's export of an object forwards its term members as a value's, as dotty makes
        // forwarders of every clause (`Namer.exportForwarders`): they are members of the class,
        // which implement and hide what it inherits; the rest stays the class's export table's.
        let object = match through_value {
            false => self.export_scope(|t| t.export_static_path(&clause[0].path)),
            true => None,
        };
        // A case class's companion teq leaves synthetic (no object of it, its `apply` the class's
        // creator) has synthetic members alone, which a wildcard passes over (dotty's
        // `addWildcardForwarders`: `membersBasedOnFlags(excluded = PrivateOrSynthetic)`, default
        // getters forwarded with their members): the clause forwards nothing, as scalac's
        // `export lila.core.user.Plan.*` of a case class `Plan` without a written companion.
        if !through_value && object.is_none() && clause.iter().all(|imp| matches!(imp.sel, ImportSel::Wildcard | ImportSel::Given)) && self.export_scope(|t| t.synthetic_companion_path(&clause[0].path)) {
            return None;
        }
        let vp = match (through_value, object) {
            (true, _) => self.value_export_path(&clause[0].path, clause[0].span),
            (false, Some(StaticPath::Object(o))) => self.object_value_path(o),
            (false, Some(StaticPath::Package(p))) => Some(self.package_value_path(p)),
            // No object, package or value: the qualifier's resolution reports what it misses
            // (dotty's `typedAheadExpr` of the qualifier, `Not found`, `is not a member`).
            (false, None) => self.value_export_path(&clause[0].path, clause[0].span),
        };
        let Some(vp) = vp else { return None };
        let file = self.home_file(home);
        // A path whose type is in error, an error already reported in its initializer, has a
        // member of every name, of the error type (dotty's `findMember` gives an error symbol,
        // `newErrorSymbol`): each named selector forwards to it silently, a wildcard to nothing.
        if vp.pkg.is_none() && self.widen_path(vp.path) == ERROR {
            self.error_forwarders(home, clause);
            return None;
        }
        // A path whose class is in the middle of its completion on this thread, a cycle through
        // the lookups that completion makes (its parent found through an import of this owner,
        // `exports_blocked`), has its members once the completion ends, where the clause is made:
        // dotty's class completer meets the class complete in the order it takes there.
        if vp.pkg.is_none() {
            let q = self.widen_path(vp.path);
            if let Some(k) = self.class_of(q) {
                let state = self.syms.class(k).state();
                if state.get() == Completion::InProgress && state.mine() {
                    return Some(k);
                }
            }
        }
        let vp = Arc::new(vp);
        if self.index.is_some() {
            self.index_export_clause(home, &vp, clause);
        }
        // The members the clause forwards may be forwarders of the path's class themselves, or
        // top-level forwarders of the package's files.
        match vp.pkg {
            Some(p) if !self.value_forwarders_made => {
                let files: Vec<FileId> = self.syms.pkg(p).export_files.clone();
                for f in files {
                    self.complete_file_forwarders(p, f);
                }
            }
            Some(_) => {}
            None => self.complete_forwarders_of_type(vp.path),
        }
        // The names the clause's other selectors take, which its wildcard leaves out.
        let named: Vec<Name> = clause.iter().filter_map(|imp| if let ImportSel::Name(n, _) = imp.sel { Some(n) } else { None }).collect();
        let mut targets: Vec<Name> = Vec::new();
        for imp in clause {
            match imp.sel {
                ImportSel::Name(n, rename) => {
                    let alias = rename.unwrap_or(n);
                    if self.name_str(alias) == "_" {
                        continue;
                    }
                    let at = self.selector_span(imp);
                    // The type of the name and the term (dotty's `addForwardersNamed` looks up
                    // `name` and `name.toTypeName`), in error only when neither is forwarded.
                    let typed = self.forward_type(home, &vp, n, alias, at);
                    if let Err(why) = self.forward_member(home, &vp, n, alias, at, imp.span, &mut targets, Selected::Named) {
                        if typed {
                            targets.push(alias);
                            continue;
                        }
                        let path = self.path_text(&vp, &clause[0].path);
                        let mut msg = format!("no eligible member {} at {}", self.name_str(n), path);
                        if let Some(why) = why {
                            msg.push_str(&format!("\n{}.{} cannot be exported because it {}", path, self.name_str(n), why));
                        }
                        self.diags.error(file, at, msg);
                    }
                }
                ImportSel::Wildcard => {
                    let at = self.selector_span(imp);
                    for n in self.forwardable_names(&vp, false) {
                        if !named.contains(&n) {
                            let _ = self.forward_member(home, &vp, n, n, at, imp.span, &mut targets, Selected::Wildcard);
                        }
                    }
                    let types = self.forwardable_type_names(&vp);
                    for n in types {
                        // A name another selector of the clause took clashes, which the wildcard
                        // passes over (dotty's `canForward`: `targets.contains(alias)`).
                        if !named.contains(&n) && !targets.contains(&n) {
                            self.forward_type(home, &vp, n, n, at);
                        }
                    }
                }
                ImportSel::Given => {
                    let at = self.selector_span(imp);
                    for n in self.forwardable_names(&vp, true) {
                        if !named.contains(&n) {
                            let _ = self.forward_member(home, &vp, n, n, at, imp.span, &mut targets, Selected::Givens);
                        }
                    }
                }
            }
        }
        None
    }

    /// Whether `name` of class `b` is one of dotty's `caseClassSynthesized`: `hashCode`, `equals`
    /// and `toString` of `Any`, `canEqual` and the `product` members of `Product`.
    fn case_class_synthesized(&self, b: ClassId, name: Name) -> bool {
        let info = self.syms.class(b);
        let class_name = self.name_str(info.name);
        let in_scala = match info.owner {
            Owner::Package(p) => matches!(self.name_str(self.syms.pkg(p).name).as_str(), "scala" | "lang"),
            _ => false,
        };
        let any = in_scala && matches!(class_name.as_str(), "Any" | "AnyRef" | "Object");
        let product = Some(b) == self.b.product || (in_scala && class_name == "Equals");
        // A source case class has no symbol of its synthesized `equals`: the member its type has
        // of the name is `scala.Equals`'s, which the synthesized one implements.
        match self.name_str(name).as_str() {
            "hashCode" | "toString" => any,
            "equals" => any || product,
            "canEqual" | "productArity" | "productPrefix" | "productElement" | "productElementName" => product,
            _ => false,
        }
    }

    /// Whether the class `c` reaches member `m` of the path, as dotty's `isAccessibleFrom` from
    /// inside it: a private one from its class or companion, a qualified one inside its scope,
    /// a protected one inside its qualifier's scope (`accessWithin(boundary)`, `protected[p]`
    /// from `p`) or else from a class deriving from its owner and through a path of that class
    /// (`isProtectedAccessOK`), the two routes `isAccessibleFrom` takes in turn.
    fn export_accessible(&self, c: ClassId, m: SymId, path_class: Option<ClassId>) -> bool {
        if !self.is_accessible(m) {
            return false;
        }
        let info = self.syms.sym(m);
        let Owner::Class(owner) = info.owner else { return true };
        if info.mods & mods::PROTECTED == 0 || self.within_qualified_boundary(m) {
            return true;
        }
        self.derives_or_selects(c, owner) && path_class.is_some_and(|k| k == c || self.syms.class(k).base_types.iter().any(|&(b, _)| b == c))
    }

    /// Whether the site is inside the scope a qualified member names (`protected[p]`): a source
    /// definition's qualifier as the parser filed it, a library member's as its pickle gives it.
    fn within_qualified_boundary(&self, m: SymId) -> bool {
        let info = self.syms.sym(m);
        if self.has_access_scope(info.file, info.span.start) {
            return self.in_access_scope(info.file, info.span.start, info.owner);
        }
        if info.mods & mods::QUALIFIED == 0 {
            return false;
        }
        match self.loaded.as_ref().and_then(|l| l.access_within.get(&m)) {
            Some(&(scope, _)) => self.within_scope_of(info.owner, scope),
            None => false,
        }
    }

    /// A concrete member of a class `c` derives from that `m` overrides (dotty's `canForward`
    /// over `allOverriddenSymbols`): the forwarder's target is a member of `c` already.
    fn overridden_member_of(&mut self, c: ClassId, m: SymId) -> Option<SymId> {
        let info = self.syms.sym(m);
        if info.mods & mods::OVERRIDE == 0 && !info.java_defined {
            return None;
        }
        let (name, Owner::Class(owner)) = (info.name, info.owner) else { return None };
        let this_ty = self.syms.this_type(c);
        let bases: Vec<(ClassId, TypeId)> = self.met_base_types(owner);
        for &(b, bt) in bases.iter().skip(1) {
            if self.base_type(this_ty, b).is_none() {
                continue;
            }
            let mut k = 0;
            while let Some(p) = self.own_alternative(b, name, k) {
                k += 1;
                if self.syms.sym(p).kind == SymKind::Def && !self.is_private(p) && !self.is_abstract_member(p) && self.same_parameters(owner, &bases, m, p, bt) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// The forwarders of a clause whose path is in error: one of the error type per named
    /// selector the class has no member of.
    fn error_forwarders(&mut self, home: Home, clause: &[Import]) {
        let file = self.home_file(home);
        for imp in clause {
            let ImportSel::Name(n, rename) = imp.sel else { continue };
            let alias = rename.unwrap_or(n);
            let taken = match home {
                Home::Class(c) => self.syms.class(c).members.contains_key(&alias),
                Home::File(p, _) => self.syms.pkg(p).entries.get(&alias).is_some_and(|e| e.term.is_some()),
            };
            if self.name_str(alias) == "_" || taken {
                continue;
            }
            let at = self.selector_span(imp);
            let s = self.syms.new_sym(alias, SymKind::Def, mods::FINAL, self.home_owner(home), file, None, at);
            {
                let mut info = self.syms.sym_mut(s);
                info.sig = Some(Arc::new(MethodSig::value(ERROR)));
                info.state().set(Completion::Done);
            }
            self.enter_forwarder(home, s);
        }
    }

    /// Enters a forwarder among the members it joins: a class's, or the package's terms as the
    /// file's top-level definitions are entered (`register_term`: an overload of a name the
    /// package has, or the double definition dotty's `checkNoConflict` reports).
    fn enter_forwarder(&mut self, home: Home, s: SymId) {
        // An extension method's forwarder is an extension method of the home, as the namer
        // enters a written one: on its owner's extensions, not a member.
        if self.syms.sym(s).is_extension {
            let name = self.syms.sym(s).name;
            match home {
                Home::Class(c) => {
                    self.syms.class_mut(c).extensions.push(s);
                    self.syms.note_member_name(c, name);
                }
                Home::File(p, _) => self.syms.pkgs[p.idx()].entries.entry(name).or_default().extensions.push(s),
            }
            return;
        }
        match home {
            Home::Class(c) => self.syms.add_member(c, s),
            Home::File(p, _) => {
                self.register_term(Owner::Package(p), s);
            }
        }
    }

    /// The path as dotty prints a qualifier, a member of a class through its `this`.
    fn path_text(&self, vp: &ValuePath, path: &[Name]) -> String {
        let names = path.iter().map(|&n| self.name_str(n)).collect::<Vec<_>>().join(".");
        if vp.pkg.is_some() {
            return names;
        }
        let mut head = vp.value;
        while let (_, _, Some(prev)) = self.import_values[head.0 as usize] {
            head = prev;
        }
        match self.import_values[head.0 as usize] {
            (_, Some(k), None) if self.syms.class(k).kind != ClassKind::Object => format!("{}.this.{}", self.name_str(self.syms.class(k).name), names),
            _ => names,
        }
    }

    /// Where a selector's forwarders and reports stand: the selector, its rename or bound
    /// included (dotty's `sel.span` in `Namer.exportForwarders`).
    pub(super) fn selector_span(&self, imp: &Import) -> Span {
        imp.selector_span
    }

    /// The names of the qualifier's members a wildcard (`givens`: a `given` selector) exports:
    /// public, not synthetic, no constructor, givens for `given` alone (dotty's
    /// `addWildcardForwarders`), each once, in the order of the qualifier's linearisation.
    fn forwardable_names(&mut self, vp: &ValuePath, givens: bool) -> Vec<Name> {
        if let Some(p) = vp.pkg {
            let mut out = self.package_forwardable_names(p, givens);
            if let Some(ovp) = self.package_object_path(p) {
                for n in self.forwardable_names(&ovp, givens) {
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
            return out;
        }
        let q = self.widen_path(vp.path);
        let Some(k) = self.class_of(q) else { return Vec::new() };
        self.complete_class(k);
        let from_case_class = self.syms.class(k).mods & mods::CASE != 0;
        let bases = self.namer_bases(k);
        let mut out: Vec<Name> = Vec::new();
        for b in bases {
            self.complete_class(b);
            let members: Vec<SymId> = self.syms.class(b).member_order.clone();
            for m in members {
                let info = self.syms.sym(m);
                let name = info.name;
                if b == k && self.mirror_support_member(k, info.owner, name, info.def.is_some()) {
                    continue;
                }
                // Neither private nor synthetic members (dotty's `PrivateOrSynthetic`): a case
                // class's selectors `_1`..`_n` are the class's own (`mods::SELECTOR`), a library's
                // compiler-made members its pickle's (`Loaded::synthetic_members`).
                let synthetic = self.loaded.as_ref().is_some_and(|l| l.synthetic_members.contains_key(&m));
                if out.contains(&name) || info.mods & (mods::PRIVATE | mods::SELECTOR) != 0 || synthetic || info.scoped_private || name == crate::names::INIT {
                    continue;
                }
                let given = info.mods & mods::GIVEN != 0 || info.kind == SymKind::Given;
                if given != givens {
                    continue;
                }
                // Default getters go with their methods (dotty's `DefaultGetterName`, a name's
                // kind, not its spelling): a library's as its pickle names them, the program's
                // being no members; a member the user spelled `x$default$1` is forwarded.
                if self.loaded.as_ref().is_some_and(|l| l.default_getters.contains_key(&m)) {
                    continue;
                }
                // The members of `Any` and `Product` a case class gets synthesized implementations
                // of (dotty's `caseClassSynthesized`), where the path is a case class.
                if from_case_class && self.case_class_synthesized(b, name) {
                    continue;
                }
                out.push(name);
            }
        }
        for e in self.path_extensions(q, None) {
            let info = self.syms.sym(e);
            let given = info.mods & mods::GIVEN != 0 || info.kind == SymKind::Given;
            if !out.contains(&info.name) && info.mods & mods::PRIVATE == 0 && !info.scoped_private && given == givens {
                out.push(info.name);
            }
        }
        out
    }

    /// The bases of `k` an export sees, `Namer.exportForwarders` running before `PostTyper`: all
    /// but the mirror's traits `SyntheticMembers.addMirrorSupport` adds then (`ClassInfo::post_typer_bases`,
    /// an enum's companion's among them).
    fn namer_bases(&self, k: ClassId) -> Vec<ClassId> {
        let info = self.syms.class(k);
        let post_typer = 1..1 + info.post_typer_bases as usize;
        let mirror_parent = self.has_desugared_mirror_parent(k);
        info.base_types.iter().enumerate().filter(|&(i, &(b, _))| !post_typer.contains(&i) && !(mirror_parent && self.is_mirror_class(b))).map(|(_, &(b, _))| b).collect()
    }

    /// Whether a member or alias of `k`, the class an export's path names, is one
    /// `SyntheticMembers.addMirrorSupport` gives it after `Namer`: a member of its mirror's traits,
    /// or one of their names it enters in `k` where `k` defines none (`ordinal`, `fromProduct`,
    /// `MirroredMonoType`, and the refinement's aliases an enum's companion holds).
    fn mirror_support_member(&self, k: ClassId, owner: Owner, name: Name, has_def: bool) -> bool {
        if matches!(owner, Owner::Class(o) if self.is_mirror_class(o)) {
            return true;
        }
        if owner != Owner::Class(k) || has_def || !self.has_desugared_mirror_parent(k) {
            return false;
        }
        self.syms.class(k).base_types.iter().any(|&(b, _)| {
            self.is_mirror_class(b) && (self.syms.class(b).members.contains_key(&name) || self.syms.class(b).type_aliases.contains_key(&name))
        })
    }

    /// The extension methods of the path's class and its bases, named `name` or all of them, in
    /// the order of the linearisation.
    fn path_extensions(&mut self, q: TypeId, name: Option<Name>) -> Vec<SymId> {
        let Some(k) = self.class_of(q) else { return Vec::new() };
        self.complete_class(k);
        let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
        let mut out: Vec<SymId> = Vec::new();
        for b in bases {
            self.complete_class(b);
            for &e in &self.syms.class(b).extensions {
                if name.map_or(true, |n| self.syms.sym(e).name == n) && !out.contains(&e) {
                    out.push(e);
                }
            }
        }
        out
    }

    /// The term members of package `p` named `name`: its files' top-level definitions of the
    /// name, extension methods among them, and an object's module val, which a library's
    /// package enters where they are first asked for (`pkg_term`).
    fn package_members(&mut self, p: PkgId, name: Name) -> Vec<SymId> {
        let _ = self.pkg_term(p, name);
        let Some(e) = self.syms.pkg(p).entries.get(&name) else { return Vec::new() };
        let mut out: Vec<SymId> = match e.term {
            Some(t) => self.syms.alternatives(t).map_or_else(|| vec![t], |a| a.to_vec()),
            None => Vec::new(),
        };
        for &x in &e.extensions {
            if !out.contains(&x) {
                out.push(x);
            }
        }
        out
    }

    /// The term members of package `p` a wildcard (`givens`: a `given` selector) exports: its
    /// files' top-level definitions and objects, public, its givens for `given` alone, by name.
    fn package_forwardable_names(&mut self, p: PkgId, givens: bool) -> Vec<Name> {
        let mut out: Vec<Name> = Vec::new();
        for (&name, e) in &self.syms.pkg(p).entries {
            let alts: Vec<SymId> = match e.term {
                Some(t) => self.syms.alternatives(t).map_or_else(|| vec![t], |a| a.to_vec()),
                None => Vec::new(),
            };
            let public = |s: &SymId| {
                let info = self.syms.sym(*s);
                info.mods & mods::PRIVATE == 0 && !info.scoped_private
            };
            let any = alts.iter().chain(&e.extensions).filter(|s| public(s)).any(|&s| {
                let info = self.syms.sym(s);
                (info.mods & mods::GIVEN != 0 || info.kind == SymKind::Given) == givens
            });
            if any {
                out.push(name);
            }
        }
        out.sort_by(|&a, &b| self.name_str(a).cmp(&self.name_str(b)));
        out
    }

    /// The type members a wildcard forwards through the path: its class's and its bases', the
    /// private ones left out (dotty's `membersBasedOnFlags(excluded = PrivateOrSynthetic)`); a
    /// package's classes and aliases.
    fn forwardable_type_names(&mut self, vp: &ValuePath) -> Vec<Name> {
        if let Some(p) = vp.pkg {
            let mut out: Vec<Name> = Vec::new();
            for (&n, e) in &self.syms.pkg(p).entries {
                let class = e.class.is_some_and(|k| {
                    let info = self.syms.class(k);
                    info.kind != ClassKind::Object && info.mods & mods::PRIVATE == 0
                });
                if class || e.alias.is_some_and(|a| !self.private_alias(a)) {
                    out.push(n);
                }
            }
            // By name, as the package's terms are (`package_forwardable_names`): the order the forwarders
            // are made and pickled in is no interning order, which the whole program's build and a
            // module's own give differently.
            out.sort_by(|&a, &b| self.name_str(a).cmp(&self.name_str(b)));
            if let Some(ovp) = self.package_object_path(p) {
                for n in self.forwardable_type_names(&ovp) {
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
            return out;
        }
        let q = self.widen_path(vp.path);
        let Some(k) = self.class_of(q) else { return Vec::new() };
        self.complete_class(k);
        let bases = self.namer_bases(k);
        let mut out: Vec<Name> = Vec::new();
        for b in bases {
            self.complete_class(b);
            let aliases: Vec<(Name, AliasId)> = self.syms.class(b).type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
            for (n, a) in aliases {
                let (owner, has_def) = (self.syms.aliases[a.idx()].owner, self.syms.aliases[a.idx()].def.is_some());
                if b == k && self.mirror_support_member(k, owner, n, has_def) {
                    continue;
                }
                if !out.contains(&n) && !self.private_alias(a) {
                    out.push(n);
                }
            }
            let nested: Vec<(Name, ClassId)> = self.syms.class(b).nested.iter().map(|(&n, &k)| (n, k)).collect();
            for (n, k) in nested {
                let info = self.syms.class(k);
                if !out.contains(&n) && info.kind != ClassKind::Object && info.mods & mods::PRIVATE == 0 {
                    out.push(n);
                }
            }
        }
        // By name, independent of the order the names were interned in (the whole program's
        // build reads the qualifier's source, a module's own build its products).
        out.sort_by(|&a, &b| self.name_str(a).cmp(&self.name_str(b)));
        out
    }

    /// The type forwarder of the path's type member `name` under `alias`, `type alias = p.name`
    /// (dotty's `addForwarder` of a type, a `TypeAlias` of `pathType.select(sym)`). A class that
    /// has a type of the name, its own or an earlier clause's forwarder, keeps it, the clause in
    /// error (`checkNoConflict`, E161). False where the path has no such member to forward.
    fn forward_type(&mut self, home: Home, vp: &ValuePath, name: Name, alias: Name, span: Span) -> bool {
        // A library's package enters the type where it is first asked for.
        if let Some(p) = vp.pkg {
            let _ = self.pkg_type(p, name);
        }
        if !self.forwardable_type_names(vp).contains(&name) {
            return false;
        }
        let file = self.home_file(home);
        let (alias_taken, nested) = match home {
            Home::Class(c) => {
                let info = self.syms.class(c);
                (info.type_aliases.contains_key(&alias), info.nested.get(&alias).copied())
            }
            Home::File(p, _) => {
                let e = self.syms.pkg(p).entries.get(&alias);
                (e.is_some_and(|e| e.alias.is_some()), e.and_then(|e| e.class))
            }
        };
        let existing = match (alias_taken, nested) {
            (true, _) => Some("type"),
            (false, Some(k)) if self.syms.class(k).kind != ClassKind::Object => Some(if self.syms.class(k).kind == ClassKind::Trait { "trait" } else { "class" }),
            _ => None,
        };
        if let Some(kind) = existing {
            let n = self.name_str(alias);
            let msg = format!("{n} is already defined as {kind} {n}");
            self.diags.error(file, span, msg);
            return true;
        }
        let rhs = self.forwarded_type(vp, name).unwrap_or(ERROR);
        // A JavaScript type the forwarded alias names is the forwarder's too (`Worker::alias_names_js`).
        let js = self.forwarded_alias_js(vp, name);
        let a = AliasId(self.syms.aliases.len() as u32);
        let owner = self.home_owner(home);
        self.syms.aliases.push(AliasInfo { name: alias, owner, file, def: None, tparams: Vec::new(), rhs, bounds: None, deprecated: false, js, exported: true, declared_rhs: crate::tir::NO_TYPE });
        self.syms.alias_cells.set(a.0, Completion::Done);
        match home {
            Home::Class(c) => {
                self.syms.class_mut(c).type_aliases.insert(alias, a);
                crate::symbols::type_member_written(alias);
            }
            Home::File(p, _) => self.syms.pkgs[p.idx()].entries.entry(alias).or_default().alias = Some(a),
        }
        true
    }

    /// The type member `name` of the path, as the type forwarder's alias names it (dotty's
    /// `pathType.select(sym)`): a class of an object or a package its class type, unapplied where
    /// it takes parameters (`etaExpand`, teq's `Ctor`); an alias its right side, a lambda over
    /// its parameters where it takes some, as a class's alias member is.
    fn forwarded_type(&mut self, vp: &ValuePath, name: Name) -> Option<TypeId> {
        let static_class = match vp.pkg {
            Some(p) => {
                let e = self.syms.pkg(p).entries.get(&name);
                if let Some(a) = e.and_then(|e| e.alias) {
                    return Some(self.alias_reference(a));
                }
                match e.and_then(|e| e.class) {
                    Some(k) => Some(k),
                    // The package object's type member, the package's.
                    None => {
                        let ovp = self.package_object_path(p)?;
                        return self.forwarded_type(&ovp, name);
                    }
                }
            }
            None => {
                let q = self.widen_path(vp.path);
                let path_class = self.class_of(q);
                let found = path_class.and_then(|k| {
                    let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
                    bases.into_iter().find_map(|b| self.syms.class(b).nested.get(&name).copied())
                });
                // An opaque type a class or trait declares is, through an object deriving from
                // it, the object's own copy (`derived_opaque`, as `nested_class_through` and
                // `opaque_seen_from` select it): its parameters' bounds and its underlying type as
                // the object fixes them, the type the object's members name (dotty's
                // `pathType.select(sym)`, `HtmlTagOf.Tag`, whose bounds `etaExpand` sees from
                // the path), not the trait's own, bounded by `TagLite.this.Top`.
                let found = match (found, path_class) {
                    (Some(n), Some(o)) if self.is_opaque_path_class(n) && self.syms.class(o).kind == ClassKind::Object => Some(self.derived_opaque(n, o)),
                    (found, _) => found,
                };
                found.filter(|&k| !self.is_inner_class(k))
            }
        };
        match static_class {
            Some(k) if !self.takes_type_params(k) => Some(self.types.class(k, &[])),
            Some(k) => Some(self.types.mk(Type::Ctor(k))),
            None => {
                let t = self.member_type(vp.path, name)?;
                Some(self.forwarded_lambda_bounds(t, vp.path))
            }
        }
    }

    /// An inner class unapplied through the path, `[X] =>> p.C[X]` over the class's own
    /// parameters (`inner_class_through`), with fresh parameters whose bounds are seen from the
    /// path where they name the class's enclosing `this`: dotty's `etaExpand` of
    /// `pathType.select(sym)` takes each parameter's info `asSeenFrom(self.prefix, owner)`, so
    /// that the forwarder's bounds, typed and pickled, name the path's members (`HtmlTagOf.Top`).
    fn forwarded_lambda_bounds(&mut self, t: TypeId, path: TypeId) -> TypeId {
        let Type::Lambda(ps, body) = self.types.get(t) else { return t };
        let Type::Nested(_, class) = self.types.get(body) else { return t };
        let Type::Class(c, _) = self.types.get(class) else { return t };
        let items: Vec<TypeId> = self.types.items(ps).to_vec();
        let own: Vec<TParamId> = items
            .iter()
            .filter_map(|&p| match self.types.get(p) {
                Type::Param(q) => Some(q),
                _ => None,
            })
            .collect();
        let dependent = own.iter().any(|&p| {
            let info = self.syms.tparam(p);
            self.types.has_paths(info.lower) || self.types.has_paths(info.upper)
        });
        if own.len() != items.len() || !dependent {
            return t;
        }
        let fresh: Vec<TParamId> = own
            .iter()
            .map(|&p| {
                let info = self.syms.tparam(p);
                let (n, variance, arity) = (info.name, info.variance, info.arity);
                let q = self.syms.new_tparam(n, variance);
                self.syms.tparams[q.idx()].arity = arity;
                q
            })
            .collect();
        let subst: Subst = own.iter().zip(&fresh).map(|(&p, &q)| (p, self.types.param(q))).collect();
        for (&p, &q) in own.iter().zip(&fresh) {
            let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
            let (lo, hi) = (self.types.subst(lo, &subst), self.types.subst(hi, &subst));
            self.syms.tparams[q.idx()].lower = self.outer_seen_from(lo, path, c);
            self.syms.tparams[q.idx()].upper = self.outer_seen_from(hi, path, c);
        }
        let params: Vec<TypeId> = fresh.iter().map(|&q| self.types.param(q)).collect();
        let l = self.types.list(&params);
        let body = self.types.subst(body, &subst);
        self.types.mk(Type::Lambda(l, body))
    }

    /// A reference to alias `a` as a type forwarder names it (dotty's `pathType.select(sym)`,
    /// `etaExpand` where it takes parameters): `p.T`, or `[X] =>> p.T[X]`. An alias that only
    /// renames a constructor, its right side that constructor applied to its parameters in order
    /// (`type Eq[A] = cats.kernel.Eq[A]`), is that constructor's application, as `etaExpand`'s
    /// `appliedTo` instantiates it (`TypeApplications.appliedTo`, `followAlias`): the forwarder
    /// `[X] =>> cats.kernel.Eq[X]`, which a pickle holds and another module reads alike.
    fn alias_reference(&mut self, a: crate::types::AliasId) -> TypeId {
        self.complete_alias(a);
        let tparams = self.syms.aliases[a.idx()].tparams.clone();
        if tparams.is_empty() {
            return self.types.mk(Type::Alias(a, self.types.list(&[])));
        }
        let ps: Vec<TypeId> = tparams.iter().map(|&q| self.types.param(q)).collect();
        let l = self.types.list(&ps);
        let body = self.alias_body(a);
        let renames = match self.types.get(body) {
            Type::Class(_, args) | Type::Alias(_, args) => self.types.items(args) == ps.as_slice(),
            _ => false,
        };
        let applied = if renames { body } else { self.types.mk(Type::Alias(a, l)) };
        self.types.mk(Type::Lambda(l, applied))
    }

    /// Whether class `k` takes type parameters, a source class's read from its definition
    /// without its completion (dotty's `completerTypeParams`), which may look through the
    /// imports of its file into the owner whose forwarders are being made.
    fn takes_type_params(&mut self, k: ClassId) -> bool {
        let info = self.syms.class(k);
        if let Some(d) = info.def {
            if let DefKind::Class(cls) = &self.ast(info.file).def(d).kind {
                return !cls.tparams.is_empty();
            }
        }
        self.complete_class(k);
        !self.syms.class(k).tparams.is_empty()
    }

    /// Whether the alias `name` of the path's class or a base, the forwarded one, names a
    /// JavaScript type; false for a class member.
    fn forwarded_alias_js(&mut self, vp: &ValuePath, name: Name) -> bool {
        if let Some(p) = vp.pkg {
            let Some(a) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.alias) else { return false };
            self.complete_alias(a);
            return self.syms.aliases[a.idx()].js;
        }
        let q = self.widen_path(vp.path);
        let Some(k) = self.class_of(q) else { return false };
        let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
        let Some(a) = bases.into_iter().find_map(|b| self.syms.class(b).type_aliases.get(&name).copied()) else { return false };
        self.complete_alias(a);
        self.syms.aliases[a.idx()].js
    }

    /// The forwarders of the qualifier's member `name` under `alias`, one per alternative it is
    /// eligible for (dotty's `canForward`): not private, no constructor, not what the class has
    /// already from the member's own owner, no second target of one name in the clause. A name the
    /// class defines itself takes the forwarders as further alternatives, which the check of
    /// double definitions tells apart by their erasure. Whether any was made.
    fn forward_member(&mut self, home: Home, vp: &Arc<ValuePath>, name: Name, alias: Name, span: Span, clause: Span, targets: &mut Vec<Name>, selected: Selected) -> Result<(), Option<String>> {
        let c = self.home_class(home);
        let (alts, path_class) = match vp.pkg {
            Some(p) => {
                let alts = self.package_members(p, name);
                // A member of a package object, `package`'s or a file's `<file>$package` read from
                // a pickle, is the package's as its own are (dotty's
                // `PackageClassDenotation.computeMembersNamed`), forwarded through the object.
                let mut owners: Vec<ClassId> = Vec::new();
                for &m in &alts {
                    if let Owner::Class(o) = self.syms.sym(m).owner {
                        if !owners.contains(&o) {
                            owners.push(o);
                        }
                    }
                }
                let object = match owners.as_slice() {
                    [] if alts.is_empty() => self.package_object_path(p),
                    &[o] if alts.iter().all(|&m| self.syms.sym(m).owner == Owner::Class(o)) => self.object_value_path(o),
                    _ => None,
                };
                if let Some(ovp) = object {
                    return self.forward_member(home, &Arc::new(ovp), name, alias, span, clause, targets, selected);
                }
                (alts, None)
            }
            None => {
                let q = self.widen_path(vp.path);
                let mut alts: Vec<SymId> = match self.find_member(q, name) {
                    Some((found, _)) => self.syms.alternatives(found).map_or_else(|| vec![found], |a| a.to_vec()),
                    None => Vec::new(),
                };
                // Extension methods are members as dotty has them (`ExtensionMethod`), which teq
                // keeps on their classes' extensions.
                for e in self.path_extensions(q, Some(name)) {
                    if !alts.contains(&e) {
                        alts.push(e);
                    }
                }
                (alts, self.class_of(q))
            }
        };
        if alts.is_empty() {
            return Err(None);
        }
        // `targets` holds the named selectors' aliases alone (dotty's `seen`): a wildcard and a
        // given selector pass over them, and forward alternatives of one name beside each other.
        if targets.contains(&alias) {
            return Err(Some("clashes with another export in the same export clause".to_string()));
        }
        let mut reason: Option<String> = None;
        let is_given = |t: &Self, m: SymId| {
            let info = t.syms.sym(m);
            info.kind == SymKind::Given || info.mods & mods::GIVEN != 0
        };
        let alts: Vec<SymId> = alts
            .into_iter()
            .filter(|&m| match selected {
                Selected::Named => true,
                Selected::Wildcard => !is_given(self, m),
                Selected::Givens => is_given(self, m),
            })
            .collect();
        let this_ty = self.syms.this_type(c);
        let mut made: Vec<SymId> = Vec::new();
        for m in alts {
            let (m_mods, m_name, m_owner) = {
                let info = self.syms.sym(m);
                (info.mods, info.name, info.owner)
            };
            if m_name == crate::names::INIT {
                continue;
            }
            // A package's member is a top-level definition of one of its files, which no class
            // has already; through a value or an object, a member of a class.
            let owner = match (m_owner, vp.pkg) {
                (Owner::Class(owner), None) => owner,
                (Owner::Package(_), Some(_)) => {
                    if !self.is_accessible(m) {
                        reason = Some("is not accessible".to_string());
                        continue;
                    }
                    let alt = made.len() as u32;
                    let at = Span { start: span.start + alt.min(span.end.saturating_sub(span.start)), end: span.end };
                    if let Some(s) = self.make_forwarder(home, vp, m, None, alias, at, (clause, alt)) {
                        made.push(s);
                    }
                    continue;
                }
                _ => continue,
            };
            if !self.export_accessible(c, m, path_class) {
                reason = Some("is not accessible".to_string());
                continue;
            }
            // What the class has from the member's owner already (dotty: "is already a member").
            if self.base_type(this_ty, owner).is_some() && (owner == c || !self.is_abstract_member(m)) {
                reason = Some(format!("is already a member of {}", self.home_description(home)));
                continue;
            }
            // A member that overrides a concrete one the class has already is that one.
            if let Some(other) = self.overridden_member_of(c, m) {
                let o = self.syms.sym(other).owner;
                let at = match o { Owner::Class(k) => self.class_description(k), _ => String::new() };
                let what = if self.syms.sym(other).kind == SymKind::Def { "method" } else { "value" };
                reason = Some(format!("overrides {} {} in {}, which is already a member of {}", what, self.name_str(m_name), at, self.home_description(home)));
                continue;
            }
            // `Any`'s members (`equals`, `hashCode`, `toString`, `Object`'s `clone`) are concrete
            // members of every class: a member overriding one of them is one the class has already
            // (dotty's `allOverriddenSymbols` reaches `Any`'s; `i7086`).
            if (m_mods & mods::OVERRIDE != 0 || self.syms.sym(m).java_defined) && self.matches_any_member(m) {
                let at = if m_name == crate::names::CLONE { "class Object" } else { "class Any" };
                reason = Some(format!("overrides method {} in {}, which is already a member of {}", self.name_str(m_name), at, self.home_description(home)));
                continue;
            }
            // Each alternative of an overloaded member at its own offset of the selector, so that
            // the forwarders keep one order in every build (`layout::compare_syms`).
            let alt = made.len() as u32;
            let at = Span { start: span.start + alt.min(span.end.saturating_sub(span.start)), end: span.end };
            if let Some(s) = self.make_forwarder(home, vp, m, Some(owner), alias, at, (clause, alt)) {
                made.push(s);
            }
        }
        if made.is_empty() {
            return Err(reason);
        }
        let made = self.avoid_forwarder_clashes(made);
        // A named selector's alias is what the clause's other forwarders may not take (dotty's
        // `targets`, filled by `addForwardersNamed` alone): the wildcard's and the `given`
        // selector's forwarders of one name are alternatives of each other.
        if selected == Selected::Named {
            targets.push(alias);
        }
        let Home::Class(c) = home else {
            for s in made {
                self.enter_forwarder(home, s);
            }
            return Ok(());
        };
        for s in made {
            if self.syms.sym(s).is_extension {
                self.enter_forwarder(home, s);
                continue;
            }
            match self.syms.class(c).members.get(&alias).copied() {
                Some(old) if self.syms.sym(old).owner == Owner::Class(c) => {
                    if let SymKind::Overloaded(i) = self.syms.sym(old).kind {
                        self.syms.overloads[i as usize].1.push(s);
                        self.syms.sym_mut(s).alternative = true;
                    } else {
                        let set = self.syms.new_overloaded(old, Owner::Class(c), vec![old, s]);
                        self.syms.class_mut(c).members.insert(alias, set);
                    }
                    self.syms.class_mut(c).member_order.push(s);
                }
                _ => self.syms.add_member(c, s),
            }
        }
        Ok(())
    }

    /// dotty's `avoidClashes` (Namer.scala, i14966): of two forwarders of one name that erase alike, the one
    /// whose target overloading resolution would prefer is kept, the other dropped (an export taken as an
    /// import whose use site's reference resolves); where neither is preferred, both stay, which the check of
    /// double definitions reports.
    fn avoid_forwarder_clashes(&mut self, made: Vec<SymId>) -> Vec<SymId> {
        if made.len() < 2 {
            return made;
        }
        let mut dropped = vec![false; made.len()];
        for i in 0..made.len() {
            for j in i + 1..made.len() {
                if dropped[i] || dropped[j] {
                    continue;
                }
                let (a, b) = (made[i], made[j]);
                if self.erased_signature(a, true) != self.erased_signature(b, true) || self.erased_result(a, true) != self.erased_result(b, true) {
                    continue;
                }
                match self.preferred_forwarder(a, b) {
                    Some(true) => dropped[j] = true,
                    Some(false) => dropped[i] = true,
                    None => {}
                }
            }
        }
        made.into_iter().zip(dropped).filter(|&(_, d)| !d).map(|(s, _)| s).collect()
    }

    /// A forwarder of `m`, a member of `owner` the qualifier has, as the path sees it: the
    /// owner's type parameters given by the qualifier's base type, its `this` the path. A
    /// package's member (`owner` none) is the forwarder's signature as it stands.
    fn make_forwarder(&mut self, home: Home, vp: &Arc<ValuePath>, m: SymId, owner: Option<ClassId>, alias: Name, span: Span, (clause, alt): (Span, u32)) -> Option<SymId> {
        let q = if owner.is_some() { self.widen_path(vp.path) } else { crate::types::ANY };
        let mut subst = match owner {
            Some(owner) => {
                let base = self.base_type(q, owner)?;
                self.owner_subst(base)
            }
            None => Vec::new(),
        };
        let sig = self.sig_of(m).clone();
        let file = self.home_file(home);
        // The member's own type parameters, fresh ones whose bounds are seen from the path too
        // (dotty's forwarder takes `mbr.info` as the path sees it): `narrow[B <: A]` of an
        // `Impl[A]` exported through an `Impl[String]` is `narrow[B <: String]`.
        // A bound naming the owner's members through its `this` (`N <: Top` of a trait's
        // `type Top`, which the object fixes) is seen from the path as well.
        let path_bounds = owner.is_some()
            && sig.tparams.iter().any(|&p| {
                let info = self.syms.tparam(p);
                self.types.has_paths(info.lower) || self.types.has_paths(info.upper)
            });
        let tparams: Vec<TParamId> = if sig.tparams.is_empty() || (subst.is_empty() && !path_bounds) {
            sig.tparams.clone()
        } else {
            let fresh: Vec<TParamId> = sig.tparams.iter().map(|&p| {
                let info = self.syms.tparam(p);
                let (n, variance, arity) = (info.name, info.variance, info.arity);
                let q = self.syms.new_tparam(n, variance);
                self.syms.tparams[q.idx()].arity = arity;
                q
            }).collect();
            subst.extend(sig.tparams.iter().zip(&fresh).map(|(&p, &q)| (p, self.types.param(q))));
            for (&p, &q) in sig.tparams.iter().zip(&fresh) {
                let (lo, hi) = (self.syms.tparam(p).lower, self.syms.tparam(p).upper);
                let (lo, hi) = (self.types.subst(lo, &subst), self.types.subst(hi, &subst));
                let owner = owner.expect("a class's member, whose owner's parameters the path gives");
                self.syms.tparams[q.idx()].lower = self.as_seen_from(lo, vp.path, owner);
                self.syms.tparams[q.idx()].upper = self.as_seen_from(hi, vp.path, owner);
            }
            fresh
        };
        let seen = |t: &mut Self, ty: TypeId| {
            let ty = t.types.subst(ty, &subst);
            match owner {
                Some(owner) => t.as_seen_from(ty, vp.path, owner),
                None => ty,
            }
        };
        // A stable public member through a path that refers to nothing private gives its forwarder
        // the member's singleton type through the path (dotty's `ExprType(pathType.select(sym))`,
        // `StableRealizable`): an exported enum case is still the case, to a pattern and to the
        // match's exhaustivity.
        // An object is a stable member too: its forwarder is `final def O: path.O.type` (a
        // module val's `TermRef`), teq's type of an object being its class.
        let module = match self.syms.sym(m).kind {
            SymKind::Object(k) => Some(k),
            _ => None,
        };
        let stable = sig.tparams.is_empty() && sig.clauses.is_empty() && (module.is_some() || self.is_stable_path_member(m)) && self.syms.sym(m).mods & (mods::PRIVATE | mods::PROTECTED) == 0 && !self.path_refers_to_private(vp.value);
        // Through an object, the member's path as teq types a selection of it there (`Term` of a
        // member of an object, `object_member_path`), so that the two singletons are one type.
        // The forwarder's own parameters, which the member's later parameter types and its result
        // name in its place (`def id(e: Entry): e.T` forwards as `(e': Entry): e'.T`), as dotty's
        // forwarder takes the member's method type, its parameter references bound to its own.
        let fresh: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter()).map(|p| self.syms.new_sym(p.name, SymKind::Param, 0, Owner::Local, file, None, span)).collect();
        let terms: Vec<(SymId, TypeId)> =
            sig.clauses.iter().flat_map(|cl| cl.params.iter()).zip(&fresh).map(|(p, &q)| (p.sym, self.types.mk(Type::Term(q)))).collect();
        let rebind = |t: &mut Self, ty: TypeId| {
            let ty = seen(t, ty);
            t.subst_paths(ty, &terms)
        };
        let mut next = fresh.iter().copied();
        let clauses: Vec<ClauseSig> = sig
            .clauses
            .iter()
            .map(|cl| ClauseSig {
                params: cl
                    .params
                    .iter()
                    .map(|p| {
                        let ty = rebind(self, p.ty);
                        let local_ty = match (p.repeated, self.seq_class()) {
                            (true, Some(seq)) => self.types.class(seq, &[ty]),
                            _ => ty,
                        };
                        let q = next.next().expect("a fresh parameter per parameter");
                        // dotty's `RetainedExportTermParamFlags`.
                        let retained = self.syms.sym(p.sym).mods & (mods::GIVEN | mods::IMPLICIT | mods::INLINE);
                        {
                            let mut info = self.syms.sym_mut(q);
                            info.mods |= retained;
                            info.sig = Some(Arc::new(MethodSig::value(local_ty)));
                            info.state().set(Completion::Done);
                            info.by_name = p.by_name;
                        }
                        ParamSig { ty, sym: q, ..*p }
                    })
                    .collect(),
                ..*cl
            })
            .collect();
        // The result after the parameters, whose types its paths through them read.
        let ret = match (stable, vp.object) {
            _ if stable && module.is_some() => self.types.class(module.expect("the object"), &[]),
            // A package's val is its own path (`TermRef` of a top-level val).
            (true, _) if vp.pkg.is_some() => self.types.mk(Type::Term(m)),
            (true, true) => match self.class_of(q) {
                Some(o) => self.object_member_path(o, m),
                None => self.types.mk(Type::Select(vp.path, m)),
            },
            (true, false) => self.types.mk(Type::Select(vp.path, m)),
            // An inline member declared without a result, whose result teq leaves to each
            // expansion, forwards the one scalac infers, as dotty's forwarder takes the member's
            // completed info (`Namer.inferredResultType` of its body, checked for it now).
            (false, _) if sig.ret == ERROR && self.syms.sym(m).mods & mods::INLINE != 0 && self.syms.sym(m).def.is_some() => {
                self.check_inline_definition(m);
                let inferred = self.inferred_inline_result(m).unwrap_or(ERROR);
                rebind(self, inferred)
            }
            (false, _) => rebind(self, sig.ret),
        };
        let target = self.syms.sym(m);
        let (is_extension, ext_tparams, ext_clauses) = (target.is_extension, target.ext_tparams, target.ext_clauses);
        // dotty's `RetainedExportTermFlags`: an inline member's forwarder is inline, its body the
        // member's call on the path, stored as an inline method's (`inline_forwarder_body`).
        let mut retained = target.mods & (mods::GIVEN | mods::IMPLICIT | mods::INFIX | mods::EXTENSION | mods::INLINE | mods::TRANSPARENT);
        if target.kind == SymKind::Given {
            retained |= mods::GIVEN;
        }
        let s = self.syms.new_sym(alias, SymKind::Def, mods::FINAL | retained, self.home_owner(home), file, None, span);
        {
            let mut info = self.syms.sym_mut(s);
            info.sig = Some(Arc::new(MethodSig { tparams, clauses, ret, declared: crate::tir::NO_TYPE }));
            info.state().set(Completion::Done);
            info.is_extension = is_extension;
            info.ext_tparams = ext_tparams;
            info.ext_clauses = ext_clauses;
            info.stable = stable;
        }
        let fwd = Arc::new(Forward { path: vp.value.0, target: m, clause, alt });
        let given = retained & (mods::GIVEN | mods::IMPLICIT) != 0;
        match home {
            Home::Class(c) => {
                self.syms.class_mut(c).forwarders.push((s, fwd));
                if given {
                    self.syms.class_mut(c).givens.push(s);
                }
            }
            Home::File(p, _) => {
                self.syms.pkgs[p.idx()].forwarders.push((s, fwd));
                if given {
                    self.syms.pkgs[p.idx()].givens.push(s);
                }
            }
        }
        Some(s)
    }

    /// The bodies of the class's value forwarders, as its methods: the path the clause's qualifier
    /// is typed as, then the member called with the forwarder's parameters.
    pub(super) fn define_value_forwarders(&mut self, c: ClassId) -> Vec<crate::tir::FunId> {
        if self.syms.class(c).forwarders.is_empty() {
            return Vec::new();
        }
        // The class's forwarders, its extension methods' among them, which are no members.
        let fwds: Vec<(SymId, Arc<Forward>)> = self.syms.class(c).forwarders.clone();
        let mut out = Vec::new();
        for (s, fwd) in fwds {
            if self.syms.sym(s).owner != Owner::Class(c) || self.fun_of_sym.contains_key(&s) {
                continue;
            }
            // An inline one has its body checked and stored at its definition, as an inline
            // method's is (`walk_body`), for its expansions and the pickle.
            if self.is_inline_forwarder(s) {
                self.check_inline_definition(s);
                if let Some(f) = self.inline_forwarder_getters(s) {
                    out.push(f);
                }
                continue;
            }
            if let Some(f) = self.forwarder_fun(s, &fwd) {
                out.push(f);
            }
        }
        out
    }

    /// The bodies of the program's top-level value forwarders, top-level methods of their files
    /// as the files' own definitions are: typed once every body is, by the one worker left.
    pub fn define_top_value_forwarders(&mut self) {
        for i in 0..self.syms.pkgs.len() as u32 {
            let p = PkgId(i);
            if self.syms.pkg(p).forwarders.is_empty() {
                continue;
            }
            let fwds: Vec<(SymId, Arc<Forward>)> = self.syms.pkg(p).forwarders.clone();
            for (s, fwd) in fwds {
                if self.fun_of_sym.contains_key(&s) {
                    continue;
                }
                if self.is_inline_forwarder(s) {
                    self.check_inline_definition(s);
                    if let Some(f) = self.inline_forwarder_getters(s) {
                        self.register_top_fun(f);
                    }
                    continue;
                }
                let file = self.syms.sym(s).file;
                let env = self.env_for(file, Owner::Package(p));
                if let Some(f) = self.with_env(env, |t| t.forwarder_fun(s, &fwd)) {
                    self.register_top_fun(f);
                }
            }
        }
    }

    /// The function of a value forwarder `s`.
    fn forwarder_fun(&mut self, s: SymId, fwd: &Forward) -> Option<crate::tir::FunId> {
        let span = self.syms.sym(s).span;
        let sig = self.sig_of(s).clone();
        let path = self.forward_receiver(fwd, span)?;
        let target = fwd.target;
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let body = if self.is_method_sym(target) {
            // A repeated parameter's sequence goes to the member's as it is, `xs*` (dotty's
            // `adaptForwarderParams`, `seqToRepeated`), typed as the sequence it is.
            let params_flat: Vec<ParamSig> = sig.clauses.iter().flat_map(|cl| cl.params.iter().cloned()).collect();
            let args: Vec<TExprId> = params_flat
                .iter()
                .map(|p| {
                    let e = self.prog.add(TExpr::Local(p.sym));
                    match (p.repeated, self.seq_class()) {
                        (true, Some(seq)) => {
                            let seq_ty = self.types.class(seq, &[p.ty]);
                            self.prog.set_type(e, seq_ty);
                            if self.capturing() {
                                self.capture_wrap(e, crate::tir::capture::Wrap::Splice(p.ty));
                            }
                        }
                        // A by-name parameter is read by its call below, the local its thunk.
                        _ if p.by_name => {}
                        _ => self.prog.set_type(e, p.ty),
                    }
                    // A by-name parameter's argument is passed by name again, its read in the
                    // thunk the parameter's call, as a call passes a by-name parameter on.
                    if p.by_name {
                        let read = self.prog.add(TExpr::CallClosure(e, crate::ast::ListRef::EMPTY));
                        self.prog.set_type(read, p.ty);
                        self.by_name_thunk(read)
                    } else {
                        e
                    }
                })
                .collect();
            let list = self.prog.list(&args);
            let call = self.forwarder_call(path, target);
            let te = self.build_call(&call, list);
            self.prog.set_type(te, sig.ret);
            // The pickle's call names its type arguments, the forwarder's own parameters.
            let targs: Vec<TypeId> = sig.tparams.iter().map(|&t| self.types.param(t)).collect();
            if self.capturing() && !targs.is_empty() {
                self.capture_call_targs(te, &targs);
            }
            te
        } else {
            match path {
                Some((r, rt)) => self.forward_select(r, rt, target, span).0,
                None => {
                    let name = self.syms.sym(target).name;
                    match self.term_callee(super::TermRef::Global(target), name, span) {
                        Some(callee) => self.apply_callee(callee, None, Vec::new(), span, None).0,
                        None => self.prog.add(TExpr::Unit),
                    }
                }
            }
        };
        // A default of the member is the forwarder's too, its getter forwarding to the
        // member's on the path, `n$default$i(ps before) = q.m$default$i(ps before)` (dotty's
        // `exportForwarders`, `foreachDefaultGetterOf`): the getter's application a call
        // leaving the argument out makes (`TExpr::Default`), each with a path of its own.
        let defaults = self.forwarder_getters(fwd, &sig, &params, span);
        let f = self.prog.add_fun(TFun { sym: s, params, defaults, body: Some(body), body_unconsuming: false, captures: 0, lifted: false });
        self.fun_of_sym.insert(s, f);
        Some(f)
    }

    /// A forwarder's default getters: each the member's on the path, applied to the forwarder's
    /// parameters before its own, `n$default$i(ps before) = q.m$default$i(ps before)` (dotty's
    /// `foreachDefaultGetterOf`, `addForwarder` of each getter).
    pub(super) fn forwarder_getters(&mut self, fwd: &Forward, sig: &MethodSig, params: &[SymId], span: Span) -> Vec<Option<TExprId>> {
        let target = fwd.target;
        let mut defaults = vec![None; params.len()];
        if self.is_method_sym(target) {
            let flat: Vec<(usize, TypeId, bool)> = sig.clauses.iter().enumerate().flat_map(|(ci, cl)| cl.params.iter().map(move |p| (ci, p.ty, p.has_default))).collect();
            for (i, &(ci, ty, has_default)) in flat.iter().enumerate() {
                if !has_default {
                    continue;
                }
                // A package's member's getter is a top-level definition too, called with no
                // receiver.
                let Some(path) = self.forward_receiver(fwd, span) else { continue };
                let before: usize = sig.clauses[..ci].iter().map(|cl| cl.params.len()).sum();
                let mut items: Vec<TExprId> = path.iter().map(|&(r, _)| r).collect();
                for k in 0..before {
                    let e = self.prog.add(TExpr::Local(params[k]));
                    self.prog.set_type(e, flat[k].1);
                    items.push(e);
                }
                let l = self.prog.list(&items);
                let receiver = if path.is_some() { crate::tir::DEFAULT_RECEIVER } else { 0 };
                let d = self.prog.add(TExpr::Default(crate::tir::DefaultOf::Method(target), i as u32 | receiver, l));
                self.prog.set_type(d, ty);
                defaults[i] = Some(d);
            }
        }
        defaults
    }

    /// An inline forwarder's ordinary default getters, which a caller over the products applies:
    /// a function without a body, as an inline method's is, an inline parameter's getter left
    /// to its expansions (`Desugar.defaultGetters`: a getter is inline where its parameter is).
    fn inline_forwarder_getters(&mut self, s: SymId) -> Option<crate::tir::FunId> {
        let stored = self.inline_definitions.get(&s)?.defaults.clone();
        let sig = self.sig_arc(s);
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let defaults: Vec<Option<TExprId>> = params.iter().enumerate().map(|(i, &p)| stored.get(i).copied().flatten().filter(|_| self.syms.sym(p).mods & mods::INLINE == 0)).collect();
        if defaults.iter().all(Option::is_none) {
            return None;
        }
        let f = self.prog.add_fun(TFun { sym: s, params, defaults, body: None, body_unconsuming: false, captures: 0, lifted: false });
        self.fun_of_sym.insert(s, f);
        Some(f)
    }

    /// The default getter of an inline forwarder `s`: the forwarded member's, applied on the
    /// forwarder's path, and that path (none for a package's member), as dotty forwards the
    /// member's getters with it (`Namer.foreachDefaultGetterOf`, `path.m$default$i`). None for
    /// what is no inline forwarder, whose getters are its function's own.
    pub(super) fn forwarded_default(&mut self, s: SymId) -> Option<(crate::tir::DefaultOf, Option<TExprId>)> {
        if !self.is_inline_forwarder(s) {
            return None;
        }
        let fwd = self.syms.forward_of(s)?;
        let path = self.forward_receiver(&fwd, Span::default())?;
        Some((crate::tir::DefaultOf::Method(fwd.target), path.map(|(r, _)| r)))
    }

    /// The forwarder's qualifier as a tree: the path its clause's qualifier is typed as, read
    /// where the forwarder stands; none for a package's member, named statically.
    pub(super) fn forward_receiver(&mut self, fwd: &Forward, span: Span) -> Option<Option<(TExprId, TypeId)>> {
        if fwd.is_static() {
            return Some(None);
        }
        self.import_value_ref(ValueImport(fwd.path), span).map(Some)
    }

    /// The forwarder's call of its member: on the path, the owner's type parameters as the path
    /// gives them, or a package's member's static call.
    fn forwarder_call(&mut self, path: Option<(TExprId, TypeId)>, target: SymId) -> super::apply::MethodCall {
        let owner_subst = match path.and_then(|(_, rt)| self.find_member(rt, self.syms.sym(target).name)) {
            Some((_, owner_ty)) => self.owner_subst(owner_ty),
            None => Vec::new(),
        };
        super::apply::MethodCall { recv: path.map(|(r, _)| r), sym: target, owner_subst, ext_recv: None, prefix: None, refined: None }
    }

    /// An inline member's forwarder, which has no runtime body: a call expands its body, the
    /// member's call on the path (dotty's `PrepareInlineable.registerInlineInfo` of the forwarder).
    #[inline]
    pub(super) fn is_inline_forwarder(&self, s: SymId) -> bool {
        self.syms.sym(s).mods & (mods::INLINE | mods::TRANSPARENT) != 0 && self.syms.is_forwarder(s)
    }

    /// The body of an inline forwarder for its definition check: the member's call on the path
    /// read from the class's `this` (which an expansion replaces by the call's receiver), the
    /// forwarder's parameters its arguments; the member's own inline call kept for the expansion,
    /// as the check keeps every inline call of a body (`defer_inline`).
    pub(super) fn inline_forwarder_body(&mut self, s: SymId, fwd: &Forward, sig: &MethodSig) -> (Option<TExprId>, TypeId, Vec<Option<TExprId>>) {
        let span = self.syms.sym(s).span;
        let nparams = sig.clauses.iter().map(|cl| cl.params.len()).sum();
        let Some(path) = self.forward_receiver(fwd, span) else { return (None, sig.ret, vec![None; nparams]) };
        let target = fwd.target;
        let args: Vec<TExprId> = sig
            .clauses
            .iter()
            .flat_map(|cl| cl.params.iter())
            .map(|p| {
                let e = self.prog.add(TExpr::Local(p.sym));
                self.prog.set_type(e, p.ty);
                e
            })
            .collect();
        let list = self.prog.list(&args);
        let te = if self.is_method_sym(target) {
            let call = self.forwarder_call(path, target);
            let te = self.build_call(&call, list);
            self.prog.set_type(te, sig.ret);
            // The member's type parameters are the forwarder's own (fresh ones of the same order).
            let target_sig = self.sig_arc(target);
            let subst: Subst = target_sig.tparams.iter().copied().zip(sig.tparams.iter().map(|&t| self.types.param(t))).collect();
            if !subst.is_empty() {
                self.note_type_args(te, &subst);
            }
            if self.is_inline_callee(target) && self.keeps_inline_calls() {
                self.defer_inline(&call, &target_sig, &subst, te, sig.ret, span, None);
            }
            te
        } else {
            // An inline val's forwarder (`inline val k = 3`) is its selection, the constant it is.
            match path {
                Some((r, rt)) => self.forward_select(r, rt, target, span).0,
                None => {
                    let name = self.syms.sym(target).name;
                    match self.term_callee(super::TermRef::Global(target), name, span) {
                        Some(callee) => self.apply_callee(callee, None, Vec::new(), span, None).0,
                        None => self.prog.add(TExpr::Unit),
                    }
                }
            }
        };
        // The body stands at the export's selector, every node of it (dotty's forwarder body
        // `withSpan(span.focus)`): the pickle writes that place, which an expansion over the
        // products reads as the whole build's expansion has it.
        let file = self.syms.sym(s).file;
        let nodes: Vec<TExprId> = self.prog.descendants(te).collect();
        for e in nodes {
            if self.prog.span_of(e).is_none() {
                self.prog.set_span(e, file, span);
            }
        }
        let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
        let defaults = self.forwarder_getters(fwd, sig, &params, span);
        (Some(te), sig.ret, defaults)
    }

    /// `recv.m` for a member that takes no argument list: a field, a val's getter or a
    /// parameterless method, typed as a selection of it is.
    fn forward_select(&mut self, recv: TExprId, recv_ty: TypeId, m: SymId, span: Span) -> (TExprId, TypeId) {
        let callee = self.member_callee(recv, recv_ty, m);
        self.apply_callee(callee, None, Vec::new(), span, None)
    }
}
