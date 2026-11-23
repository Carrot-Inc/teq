//! A package's members named N among its package objects read from the class path, as dotty selects
//! them (`PackageClassDenotation.computeMembersNamed` and `dropStale`, SymDenotations.scala 2558-2624):
//! the union of every package object's members named N and the package's own; where an alternative
//! is defined in the current run, the current run's alone; else, where they come from one file, all;
//! else those of the youngest file, an error where several files are youngest, a warning (once per
//! package) where another file stands in another container than the chosen one.
//!
//! The selection is made once every object of the package is entered (`enter_pkg_objects`), and
//! its outcome is the package's entries: what a lookup of a term, a type, an import, a given or an
//! extension reads. What `dropStale` reports is given where the name is looked up
//! (`note_pkg_selection`).

use super::super::{PendingPlace, Worker};
use crate::ast::mods;
use crate::classpath::AssociatedFile;
use crate::intern::{FxMap, Name};
use crate::source::{Group, Span, NO_FILE};
use crate::symbols::*;
use crate::types::*;
use std::sync::Arc;

/// Which alternatives of a name the selection keeps.
#[derive(Clone, PartialEq, Eq)]
pub(in crate::typer) enum Admitted {
    /// One file defines the name among the package objects: every alternative.
    All,
    /// An alternative is defined in the current run: those alone (`compiledNow`).
    CurrentRun,
    /// The alternatives of this file alone, the youngest.
    File(Arc<AssociatedFile>),
}

/// What `dropStale` reports of a name: an error (`TypeError`), or a warning.
pub(in crate::typer) struct Report {
    pub error: bool,
    pub msg: String,
}

impl<'a> Worker<'a> {
    /// dotty's `computeMembersNamed` and `dropStale` for the name `name` of package `p` (a type's
    /// name where `types`) over the package's objects as `PkgInfo::package_objects` holds them:
    /// `None` where no package object has a member of the name. The objects are taken in dotty's
    /// order of them (`PackageObject::rank`), whatever order they were entered in.
    pub(in crate::typer) fn select_pkg_member(&self, p: PkgId, name: Name, types: bool) -> Option<(Admitted, Option<Report>)> {
        let mut defining: Vec<&PackageObject> = self.syms.pkg(p).package_objects.iter().filter(|o| self.object_defines(o.class, name, types)).collect();
        if defining.is_empty() {
            return None;
        }
        if self.current_run_defines(p, name, types) {
            return Some((Admitted::CurrentRun, None));
        }
        defining.sort_by_key(|o| o.rank);
        let mut files: Vec<&Arc<AssociatedFile>> = Vec::new();
        for o in &defining {
            if !files.iter().any(|f| f.display == o.file.display) {
                files.push(&o.file);
            }
        }
        if files.len() == 1 {
            return Some((Admitted::All, None));
        }
        let last = files.iter().map(|f| f.modified).max().unwrap_or_default();
        let youngest: Vec<&Arc<AssociatedFile>> = files.iter().copied().filter(|f| f.modified == last).collect();
        let chosen = youngest[0];
        let shown = self.interner.get(name);
        let ambiguous = |f: &AssociatedFile| format!("Toplevel definition {} is defined in\n  {}\nand also in\n  {}", shown, chosen.display, f.display);
        if youngest.len() > 1 {
            let msg = format!("{}\nOne of these files should be removed from the classpath.", ambiguous(youngest[1]));
            return Some((Admitted::File(chosen.clone()), Some(Report { error: true, msg })));
        }
        // An older file of another container: the youngest is not necessarily the one meant, the
        // older one standing earlier on the class path.
        let report = files.iter().find(|f| f.container != chosen.container).map(|f| Report { error: false, msg: format!("{}\nKeeping only the definition in {}", ambiguous(f), chosen.display) });
        Some((Admitted::File(chosen.clone()), report))
    }

    /// Whether the package object `obj` has a member named `name` (a type where `types`) that
    /// `nonPrivateMembersNamed` gives: a term, an extension or a given; a class or a type alias.
    fn object_defines(&self, obj: ClassId, name: Name, types: bool) -> bool {
        let info = self.syms.class(obj);
        let public = |s: SymId| self.syms.sym(s).mods & mods::PRIVATE == 0;
        if types {
            return info.nested.contains_key(&name) || info.type_aliases.contains_key(&name);
        }
        info.members.get(&name).is_some_and(|&s| public(s))
            || info.extensions.iter().any(|&s| self.syms.sym(s).name == name && public(s))
            || info.givens.iter().any(|&s| self.syms.sym(s).name == name && public(s))
    }

    /// Whether the class `obj` declares a member named `name` (a type where `types`), private
    /// ones included.
    pub(in crate::typer) fn declares(&self, obj: ClassId, name: Name, types: bool) -> bool {
        let info = self.syms.class(obj);
        if types {
            return info.nested.contains_key(&name) || info.type_aliases.contains_key(&name);
        }
        info.members.contains_key(&name) || info.extensions.iter().any(|&s| self.syms.sym(s).name == name) || info.givens.iter().any(|&s| self.syms.sym(s).name == name)
    }

    /// Whether package `p` has a member named `name` defined in the current run (`compiledNow`): a
    /// program's source defines it, or it is a package, which has no associated file.
    fn current_run_defines(&self, p: PkgId, name: Name, types: bool) -> bool {
        // The program's `package object p` (`Desugar.packageModuleDef`'s module `package`).
        let source_object = self.syms.pkg(p).package_object.filter(|&o| self.program_source(self.syms.class(o).file));
        if source_object.is_some_and(|o| self.declares(o, name, types)) {
            return true;
        }
        let scoped = self.syms.scoped(self.interner, p, name);
        let own = self.syms.package_object_entry(p, name);
        let class_now = |c: ClassId| self.program_source(self.syms.class(c).file);
        if types {
            return scoped.and_then(|e| e.class).is_some_and(class_now) || own.and_then(|e| e.alias).is_some_and(|a| self.program_source(self.syms.aliases[a.idx()].file));
        }
        let sym_now = |s: SymId| self.program_source(self.syms.sym(s).file);
        own.is_some_and(|e| e.term.is_some_and(sym_now) || e.extensions.iter().any(|&s| sym_now(s)))
            || self.syms.pkg(p).givens.iter().any(|&s| self.syms.sym(s).name == name && sym_now(s))
            || scoped.is_some_and(|e| e.pkg.is_some() || e.class.is_some_and(|c| self.syms.class(c).kind == ClassKind::Object && class_now(c)))
    }

    /// Spills the members of the package objects of `pkg` into the package, each name from the
    /// objects its selection admits (`select_pkg_member`): a name one file defines as before, the
    /// package's own entries first; a name the youngest file is chosen for, from that file's
    /// object; a name the current run defines, from none. A name whose selection reports is
    /// recorded for its lookups (`PkgInfo::conflicts`).
    pub(super) fn spill_package_objects(&mut self, pkg: PkgId) {
        let objects: Vec<PackageObject> = self.syms.pkg(pkg).package_objects.clone();
        if objects.is_empty() {
            return;
        }
        let mut decided: FxMap<(Name, bool), Option<Admitted>> = FxMap::default();
        let mut admits = |w: &mut Self, name: Name, types: bool, o: &PackageObject| -> Option<bool> {
            let admitted = decided
                .entry((name, types))
                .or_insert_with(|| {
                    let (admitted, report) = w.select_pkg_member(pkg, name, types)?;
                    if let Some(r) = report {
                        w.syms.pkgs[pkg.idx()].conflicts.push(PkgConflict { name, types, error: r.error, msg: r.msg.into() });
                    }
                    Some(admitted)
                })
                .clone();
            match admitted {
                // No object has a member of the name but a private one, which is no alternative
                // (`nonPrivateMembersNamed`): entered as the object has it.
                None => Some(false),
                Some(Admitted::All) => w.object_defines(o.class, name, types).then_some(false),
                Some(Admitted::CurrentRun) => None,
                Some(Admitted::File(f)) => (f.display == o.file.display).then_some(true),
            }
        };
        for o in &objects {
            self.spill_object(pkg, o.class, &mut |w: &mut Self, name, types| admits(w, name, types, o));
        }
    }

    /// Enters the members of the package object `obj` in the entries of its package `pkg`: its
    /// nested classes, type aliases, terms, extensions and givens, each name as `admits` decides
    /// (`Some(true)`: the object's alternative, which replaces what the package's entry holds;
    /// `Some(false)`: the entry's first; `None`: not the package's).
    pub(in crate::typer) fn spill_object(&mut self, pkg: PkgId, obj: ClassId, admits: &mut dyn FnMut(&mut Self, Name, bool) -> Option<bool>) {
        let from_program = self.program_source(self.syms.class(obj).file);
        let info = self.syms.class(obj);
        let members: Vec<SymId> = info.member_order.clone();
        let nested: Vec<(Name, ClassId)> = info.nested.iter().map(|(&n, &c)| (n, c)).collect();
        let aliases: Vec<(Name, AliasId)> = info.type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
        let extensions = info.extensions.clone();
        let givens = info.givens.clone();
        for (n, c) in nested {
            let Some(chosen) = admits(self, n, true) else { continue };
            let e = self.syms.enter_scoped(self.interner, pkg, n);
            if chosen || e.class.is_none() {
                e.class = Some(c);
            }
        }
        for (n, a) in aliases {
            let Some(chosen) = admits(self, n, true) else { continue };
            let class = self.syms.scoped(self.interner, pkg, n).and_then(|e| e.class);
            let e = self.syms.enter_package_object(pkg, n);
            if chosen || (class.is_none() && e.alias.is_none()) {
                e.alias = Some(a);
            }
        }
        for s in members {
            let name = self.syms.sym(s).name;
            let Some(chosen) = admits(self, name, false) else { continue };
            let entry = self.syms.class(obj).members[&name];
            // `val Right = scala.util.Right` of `scala.package` names the companion of a class
            // the std defines itself, whose constructor the name means there.
            let std_class = !from_program && self.syms.scoped(self.interner, pkg, name).and_then(|e| e.class).map_or(false, |c| !self.in_jar(self.syms.class(c).file));
            let e = self.syms.enter_package_object(pkg, name);
            if (chosen || e.term.is_none()) && !std_class {
                e.term = Some(entry);
            }
        }
        for s in extensions {
            let name = self.syms.sym(s).name;
            if admits(self, name, false).is_some() {
                self.syms.enter_package_object(pkg, name).extensions.push(s);
            }
        }
        for s in givens {
            let name = self.syms.sym(s).name;
            if admits(self, name, false).is_some() {
                self.syms.pkgs[pkg.idx()].givens.push(s);
            }
        }
    }

    /// The lookup of the name `name` of package `p` (a type's where `types`): what the selection
    /// among the package's objects reports of it is given (`dropStale`). The error where the
    /// youngest files tie, at each lookup; the warning where an older file stands in another
    /// container, once for the package (`ambiguityWarningIssued`), presented at the run's end
    /// (`report_package_conflicts`) whatever becomes of the lookup's attempt. The error stands at
    /// the innermost expression being typed when the lookup met the name (`Worker::pending_places`):
    /// dotty's at its tree, whose point is a selection's name where teq's span starts at the
    /// selection.
    #[inline]
    pub(in crate::typer) fn note_pkg_selection(&mut self, p: PkgId, name: Name, types: bool) {
        if self.syms.pkg(p).conflicts.is_empty() {
            return;
        }
        self.report_pkg_selection(p, name, types);
    }

    #[cold]
    #[inline(never)]
    fn report_pkg_selection(&mut self, p: PkgId, name: Name, types: bool) {
        let Some(c) = self.syms.pkg(p).conflicts.iter().find(|c| c.name == name && c.types == types).cloned() else { return };
        if c.error {
            // Outside an expression (an import's or a definition's completion), without a place.
            if self.expr_depth == 0 {
                self.diags.error_by(Group::Typer, NO_FILE, Span::default(), c.msg.to_string());
                return;
            }
            // In one, made now, so that what discards the diagnostics from here on discards it, and placed by the
            // innermost expression being typed on its return (`place_pending`), as dotty's `TypeError` unwinds to
            // the innermost `typedUnadapted`, which reports it at its tree.
            let body = (self.dependent_checks == 0 && self.is_body_file(self.env.file)).then_some(self.body_node);
            self.error_by(Group::Typer, Span::default(), c.msg.to_string());
            let ordinal = self.diags.items.last_mut().map_or(0, |d| {
                d.awaiting = crate::source::AwaitingPlace(true);
                d.ordinal.0
            });
            self.pending_places.push(PendingPlace { ordinal, depth: self.expr_depth, file: self.env.file, msg: c.msg.clone(), body });
            return;
        }
        // The first lookup in one worker's walk: before the bodies, by the rank of the item under
        // way, after them none earlier than what was kept; whichever worker looks first.
        let rank = if !self.bodies_started {
            (0, 0)
        } else if self.diag_range_open {
            self.diag_rank
        } else {
            (u32::MAX, u32::MAX)
        };
        if self.syms.pkg(p).conflict_warning.as_ref().is_some_and(|&(kept, _)| kept <= rank) {
            return;
        }
        self.with_loader(|w| {
            if !w.syms.pkg(p).conflict_warning.as_ref().is_some_and(|&(kept, _)| kept <= rank) {
                w.syms.pkgs[p.idx()].conflict_warning = Some((rank, c.msg.clone()));
            }
        });
    }

    /// The errors of the lookups met while the expression `e` was the innermost one being typed
    /// (`Worker::pending_places`), placed at it on its return: dotty's `errorTree(xtree, ex, xtree.srcPos.focus)` of
    /// the `TypeError` the innermost `typedUnadapted` catches. A library body's record words its message and its place
    /// by the span (`Worker::error`), as of the file and the selection being typed when the lookup met it.
    #[cold]
    #[inline(never)]
    pub(in crate::typer) fn place_pending(&mut self, e: crate::ast::ExprId) {
        let span = self.cur_ast().expr_span(e);
        let mut k = 0;
        while k < self.pending_places.len() {
            if self.pending_places[k].depth != self.expr_depth {
                k += 1;
                continue;
            }
            let u = self.pending_places.remove(k);
            let worded = u.body.map(|node| {
                let saved = (std::mem::replace(&mut self.env.file, u.file), std::mem::replace(&mut self.body_node, node));
                let msg = self.library_body_error(span, u.msg.to_string());
                let place = self.body_error_place(span);
                (self.env.file, self.body_node) = saved;
                (msg, place)
            });
            self.diags.place(u.ordinal, span, worded);
        }
    }

    /// The warnings of the packages' selections (`note_pkg_selection`), each package's one, at the
    /// run's end: dotty's `report.warning` without a position.
    pub(in crate::typer) fn report_package_conflicts(&mut self) {
        let warnings: Vec<Arc<str>> = self.syms.pkgs.iter().filter_map(|p| p.conflict_warning.as_ref().map(|(_, msg)| msg.clone())).collect();
        for msg in warnings {
            self.diags.warn_by(Group::Typer, NO_FILE, Span::default(), msg.to_string());
        }
    }
}
