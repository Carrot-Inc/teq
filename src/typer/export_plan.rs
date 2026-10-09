//! The forwarders of `export` clauses as scalac's `Exporter` writes them: per clause, the names
//! it forwards, the member each forwards to and the qualifier it goes through. teq's own
//! references resolve an export to the original (`exports`); these forwarders are what code teq
//! did not compile finds: the TASTy writer pickles them (`tasty::write`), and in link mode the
//! JVM emitter writes them as methods, from the
//! same plan, made before the reach so that it keeps what they call.

use super::exports::{ExportQualifier, ExportScope};
use super::resolve::{TermRef, TypeRef};
use super::Worker;
use crate::ast::{mods, Import, ImportSel};
use crate::intern::{FxMap, Name};
use crate::source::FileId;
use crate::symbols::*;
use crate::types::*;
use std::sync::Arc;

/// Where export clauses stand: a class's body, or a file's top level in a package.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExportSite {
    Class(ClassId),
    Package(FileId, PkgId),
}

/// One export clause and what it forwards, the names an earlier clause of the site forwarded
/// left out.
pub struct ClausePlan {
    pub path: Vec<Name>,
    pub sels: Vec<ImportSel>,
    /// The object or package the clause takes members from; None where it is neither.
    pub q: Option<ExportQualifier>,
    pub types: Vec<(Name, TypeRef)>,
    /// The objects forwarded, an object's and a case class's companion alike: each with the
    /// name it is exported under, its own, and its class (the case class for a companion).
    pub objects: Vec<(Name, Name, Option<ClassId>)>,
    /// The terms forwarded: each with the name it is exported under, the member, and the name
    /// the qualifier has it under (its own, or that of the qualifier's own export of it).
    pub forwards: Vec<(Name, SymId, Name)>,
}

/// A forwarder the JVM writes: one per alternative of an overloaded name.
#[derive(Clone)]
pub struct JvmForwarder {
    pub name: Name,
    /// The member, or for an exported object its module's class.
    pub target: JvmTarget,
    pub q: ExportQualifier,
    /// The name of the member on the qualifier: its own, or that of the qualifier's forwarder
    /// of it, which the forwarder calls (a relay).
    pub selected: Name,
    pub relayed: bool,
    /// The member's signature as the qualifier sees it (`seen_through`).
    pub sig: Option<Arc<MethodSig>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JvmTarget {
    Member(SymId),
    Object(ClassId),
}

/// The forwarders of every export site of the program's own files, as the JVM writes them.
#[derive(Default)]
pub struct JvmExports {
    pub classes: FxMap<ClassId, Vec<JvmForwarder>>,
    pub files: FxMap<FileId, Vec<JvmForwarder>>,
}

impl Worker<'_> {
    /// The clauses of an export site, `entries` one per selector, with what each forwards: the
    /// selection of scalac's `Exporter`, whose own members (`own`) a wildcard leaves out.
    pub fn export_clause_plans(&mut self, site: ExportSite, scope: &ExportScope, entries: &[Import], own: &[Name]) -> Vec<ClausePlan> {
        // The clauses as written, each with its selectors: a clause's selectors share the
        // start of its span.
        let mut clauses: Vec<(Vec<Name>, u32, Vec<ImportSel>)> = Vec::new();
        for e in entries {
            match clauses.last_mut() {
                Some((path, at, sels)) if *path == e.path && *at == e.span.start => sels.push(e.sel.clone()),
                _ => clauses.push((e.path.clone(), e.span.start, vec![e.sel.clone()])),
            }
        }
        let mut out = Vec::new();
        let mut written_types: Vec<Name> = Vec::new();
        let mut written_terms: Vec<Name> = Vec::new();
        for (path, _, sels) in clauses {
            let q = match site {
                ExportSite::Class(c) => self.export_qualifier(c, &path),
                ExportSite::Package(file, pkg) => self.package_export_qualifier(file, pkg, &path),
            };
            let Some(q) = q else {
                out.push(ClausePlan { path, sels, q: None, types: Vec::new(), objects: Vec::new(), forwards: Vec::new() });
                continue;
            };
            let mut forwards: Vec<(Name, SymId, Name)> = Vec::new();
            let mut objects: Vec<(Name, Name, Option<ClassId>)> = Vec::new();
            let mut types: Vec<(Name, TypeRef)> = Vec::new();
            for sel in &sels {
                match sel {
                    ImportSel::Name(n, rename) => {
                        let fwd = rename.unwrap_or(*n);
                        match export_target(self, scope, fwd) {
                            Some((_, sym)) if matches!(self.syms.sym(sym).kind, SymKind::Object(_)) => objects.push((fwd, *n, object_class(self, sym))),
                            Some((_, sym)) => forwards.push((fwd, sym, *n)),
                            None if exports_companion(self, scope, fwd) => objects.push((fwd, *n, companion_class(scope, fwd))),
                            None => {}
                        }
                        for &x in scope.extensions.get(&fwd).into_iter().flatten() {
                            forwards.push((fwd, x, *n));
                        }
                        if let Some(&t) = scope.types.get(&fwd) {
                            types.push((fwd, t));
                        }
                    }
                    ImportSel::Wildcard => {
                        // The clause's own qualifier's members, less the names its selectors
                        // name or hide.
                        let named: Vec<Name> = sels.iter().filter_map(|x| if let ImportSel::Name(n, _) = x { Some(*n) } else { None }).collect();
                        let mut names: Vec<Name> = scope.terms.keys().copied().filter(|n| !named.contains(n) && !own.contains(n)).collect();
                        // By name: scalac's order is its scope's, which differs between its builds, and a
                        // member of another module's products has no place in a source to order by.
                        names.sort_by(|a, b| self.interner.get(*a).cmp(self.interner.get(*b)));
                        for n in names {
                            // A default getter is forwarded with its method.
                            if self.interner.get(n).contains("$default$") {
                                continue;
                            }
                            match export_target(self, scope, n) {
                                Some((_, sym)) => {
                                    let si = self.syms.sym(sym);
                                    let (given, owner) = (si.mods & (mods::GIVEN | mods::IMPLICIT) != 0 || si.kind == SymKind::Given, si.owner);
                                    if given || !(self.qualifier_has(q, owner) || self.qualifier_exports(q, n)) {
                                        continue;
                                    }
                                    if matches!(self.syms.sym(sym).kind, SymKind::Object(_)) {
                                        objects.push((n, n, object_class(self, sym)));
                                    } else {
                                        forwards.push((n, sym, n));
                                    }
                                }
                                None if exports_companion(self, scope, n) && (qualifier_has_class(self, q, scope, n) || self.qualifier_exports(q, n)) => objects.push((n, n, companion_class(scope, n))),
                                None => {}
                            }
                        }
                        let mut enames: Vec<Name> = scope.extensions.keys().copied().filter(|n| !named.contains(n)).collect();
                        enames.sort_by(|a, b| self.interner.get(*a).cmp(self.interner.get(*b)));
                        // The qualifier's own members and what its export table holds, a relayed export's.
                        let relayed = self.qualifier_export_table(q);
                        for n in enames {
                            let exported = relayed.as_ref().map_or(false, |e| e.extensions.contains_key(&n));
                            let own: Vec<SymId> = scope.extensions[&n].iter().copied().filter(|&x| exported || self.qualifier_has(q, self.syms.sym(x).owner)).collect();
                            forwards.extend(own.into_iter().map(|x| (n, x, n)));
                        }
                        let mut tnames: Vec<Name> = scope.types.keys().copied().filter(|n| !named.contains(n)).collect();
                        tnames.sort_by(|a, b| self.interner.get(*a).cmp(self.interner.get(*b)));
                        for n in tnames {
                            let t = scope.types[&n];
                            let owner = match t {
                                TypeRef::Class(k) => Some(self.syms.class(k).owner),
                                TypeRef::Alias(a) => Some(self.syms.alias(a).owner),
                                _ => None,
                            };
                            if owner.map_or(false, |o| self.qualifier_has(q, o)) || relayed.as_ref().map_or(false, |e| e.types.contains_key(&n)) {
                                types.push((n, t));
                            }
                        }
                    }
                    // The qualifier's givens, each a given forwarder, by name.
                    ImportSel::Given => {
                        let candidates: Vec<SymId> = scope.givens.clone();
                        let mut givens: Vec<(Name, SymId)> = Vec::new();
                        for g in candidates {
                            let (n, owner) = (self.syms.sym(g).name, self.syms.sym(g).owner);
                            if self.qualifier_has(q, owner) {
                                givens.push((n, g));
                            } else {
                                // A given the qualifier exports itself, under the name it has there.
                                givens.extend(self.qualifier_export_names(q, g).into_iter().map(|n| (n, g)));
                            }
                        }
                        givens.sort_by(|a, b| self.interner.get(a.0).cmp(self.interner.get(b.0)));
                        givens.dedup();
                        forwards.extend(givens.into_iter().map(|(n, g)| (n, g, n)));
                    }
                }
            }
            types.retain(|&(n, _)| {
                let fresh = !written_types.contains(&n);
                if fresh {
                    written_types.push(n);
                }
                fresh
            });
            // A name an earlier clause exported is not exported again; within a clause a name
            // may stand for several extensions, each a forwarder.
            let earlier = written_terms.clone();
            objects.retain(|&(n, _, _)| {
                let fresh = !earlier.contains(&n) && !written_terms.contains(&n);
                if fresh {
                    written_terms.push(n);
                }
                fresh
            });
            let mut seen: Vec<SymId> = Vec::new();
            forwards.retain(|&(n, sym, _)| {
                if earlier.contains(&n) || seen.contains(&sym) {
                    return false;
                }
                seen.push(sym);
                written_terms.push(n);
                true
            });
            out.push(ClausePlan { path, sels, q: Some(q), types, objects, forwards });
        }
        out
    }

    /// The clauses of class `c`'s exports, with what they forward.
    pub fn class_export_plans(&mut self, c: ClassId) -> Vec<ClausePlan> {
        let Some(scope) = self.exports_of(c) else { return Vec::new() };
        let info = self.syms.class(c);
        let Some(d) = info.def else { return Vec::new() };
        let ast = self.ast(info.file);
        let crate::ast::DefKind::Class(cls) = &ast.def(d).kind else { return Vec::new() };
        let entries: Vec<Import> = ast.exports[cls.exports.start as usize..(cls.exports.start + cls.exports.len) as usize].to_vec();
        let own: Vec<Name> = info.members.iter().filter(|(_, &m)| self.syms.sym(m).owner == Owner::Class(c)).map(|(&n, _)| n).collect();
        self.export_clause_plans(ExportSite::Class(c), &scope, &entries, &own)
    }

    /// The clauses of a file's top-level exports in package `pkg`, members of its `<file>$package`
    /// object as scalac makes them.
    pub fn package_export_plans(&mut self, file: FileId, pkg: PkgId) -> Vec<ClausePlan> {
        let entries: Vec<Import> = self.ast(file).top_exports.clone();
        if entries.is_empty() {
            return Vec::new();
        }
        let Some(scope) = self.pkg_exports_of(pkg) else { return Vec::new() };
        let own: Vec<Name> = self.syms.pkg(pkg).entries.iter().filter(|(_, e)| e.term.map_or(false, |s| self.syms.sym(s).owner == Owner::Package(pkg))).map(|(&n, _)| n).collect();
        self.export_clause_plans(ExportSite::Package(file, pkg), &scope, &entries, &own)
    }

    /// The names under which the object `q` exports the member `s`.
    pub fn qualifier_export_names(&mut self, q: ExportQualifier, s: SymId) -> Vec<Name> {
        let ExportQualifier::Object(o) = q else { return Vec::new() };
        let Some(scope) = self.exports_of(o) else { return Vec::new() };
        scope.terms.keys().copied().filter(|&n| export_target(self, &scope, n).map_or(false, |(_, t)| t == s)).collect()
    }

    /// The export table of the object `q`, whose forwarders are among its members to a relay.
    pub fn qualifier_export_table(&mut self, q: ExportQualifier) -> Option<Arc<ExportScope>> {
        let ExportQualifier::Object(o) = q else { return None };
        self.exports_of(o)
    }

    /// Whether the name is one the object an export takes members from exports itself: a
    /// forwarder of a forwarder, which scalac writes to the qualifier's own.
    pub fn qualifier_exports(&mut self, q: ExportQualifier, n: Name) -> bool {
        let ExportQualifier::Object(o) = q else { return false };
        self.exports_of(o).map_or(false, |e| e.terms.contains_key(&n))
    }

    /// Whether a member of `owner` is the qualifier's own, or inherited by it (an object's), or
    /// a package's (its own or in one of its files' package objects).
    pub fn qualifier_has(&self, q: ExportQualifier, owner: Owner) -> bool {
        match (q, owner) {
            (ExportQualifier::Object(o), Owner::Class(k)) => self.syms.class(o).base_types.iter().any(|&(b, _)| b == k),
            (ExportQualifier::Package(p), Owner::Package(k)) => p == k,
            (ExportQualifier::Package(p), Owner::Class(k)) => {
                let info = self.syms.class(k);
                info.owner == Owner::Package(p) && info.kind == ClassKind::Object && {
                    let name = self.interner.get(info.name);
                    name == "package" || name.ends_with("$package")
                }
            }
            _ => false,
        }
    }

    /// A member's signature as the object it is exported from sees it: a member of a generic
    /// parent (`object Strings extends Impl[String]`) with the parent's type parameters bound.
    pub fn export_seen_through(&mut self, sym: SymId, q: ExportQualifier) -> MethodSig {
        self.seen_through_now(sym, q)
    }

    fn seen_through_now(&mut self, sym: SymId, q: ExportQualifier) -> MethodSig {
        match q {
            ExportQualifier::Object(o) => self.member_sig_seen_from(sym, o),
            ExportQualifier::Package(_) => self.sig_of(sym).clone(),
        }
    }

    /// The signature of the member `sym` as class `c` sees it, its own type parameters for its
    /// parameters: a member of a generic base class (`object Strings extends Impl[String]`,
    /// `trait Stack extends Parent[String]`) with the base's type parameters bound as `c`'s base
    /// type binds them, as scalac's `asSeenFrom` gives an export's forwarder and a trait's super
    /// accessor (`SuperAccessors`' `superInfo`).
    pub fn member_sig_seen_from(&mut self, sym: SymId, c: ClassId) -> MethodSig {
        let sig = self.sig_of(sym).clone();
        let Owner::Class(k) = self.syms.sym(sym).owner else { return sig };
        if k == c || self.syms.class(k).tparams.is_empty() {
            return sig;
        }
        let own: Vec<TypeId> = self.syms.class(c).own_tparams().iter().map(|&p| self.types.param(p)).collect();
        let this = self.types.class(c, &own);
        let Some(base) = self.base_type(this, k) else { return sig };
        let mut subst = self.owner_subst(base);
        // The method's own type parameters get copies whose bounds are seen through the
        // qualifier too (`narrow[B <: A]` of `Impl[String]` is `narrow[B <: String]`); the
        // declaration's stay as they are.
        let mut tparams = sig.tparams.clone();
        let bounded = sig.tparams.iter().any(|&t| {
            let info = self.syms.tparam(t);
            self.types.subst(info.upper, &subst) != info.upper || self.types.subst(info.lower, &subst) != info.lower
        });
        if bounded {
            let fresh: Vec<TParamId> = sig
                .tparams
                .iter()
                .map(|&t| {
                    let info = self.syms.tparam(t).clone();
                    let n = self.syms.new_tparam(info.name, info.variance);
                    self.syms.tparams[n.idx()] = info;
                    n
                })
                .collect();
            for (&old, &new) in sig.tparams.iter().zip(&fresh) {
                subst.push((old, self.types.param(new)));
            }
            for &n in &fresh {
                let (upper, lower) = (self.syms.tparam(n).upper, self.syms.tparam(n).lower);
                let (upper, lower) = (self.types.subst(upper, &subst), self.types.subst(lower, &subst));
                let info = &mut self.syms.tparams[n.idx()];
                info.upper = upper;
                info.lower = lower;
            }
            tparams = fresh;
        }
        let types = &self.types;
        MethodSig {
            tparams,
            clauses: sig
                .clauses
                .iter()
                .map(|cl| ClauseSig { params: cl.params.iter().map(|p| ParamSig { ty: types.subst(p.ty, &subst), ..p.clone() }).collect(), ..cl.clone() })
                .collect(),
            ret: types.subst(sig.ret, &subst),
        }
    }

    /// In link mode, the forwarders the JVM writes for every export site of the program's own
    /// files, one per member and per alternative of an overloaded one; an inline member's
    /// forwarder is inline, of no method, and a type's is no method.
    pub fn plan_jvm_exports(&mut self, std_files: &[bool]) -> JvmExports {
        let mut out = JvmExports::default();
        let program_file = |f: FileId| !std_files.get(f.0 as usize).copied().unwrap_or(true);
        let sites: Vec<ClassId> = (0..self.syms.classes.len())
            .map(|i| ClassId(i as u32))
            .filter(|&c| {
                let info = self.syms.class(c);
                info.has_exports && info.def.is_some() && program_file(info.file) && self.loaded.as_ref().map_or(true, |l| !l.classes.contains_key(&c))
            })
            .collect();
        for c in sites {
            let plans = self.class_export_plans(c);
            let forwarders = self.jvm_forwarders(&plans);
            if !forwarders.is_empty() {
                out.classes.insert(c, forwarders);
            }
        }
        for f in 0..self.files.len() {
            let file = FileId(f as u32);
            if !program_file(file) || self.ast(file).top_exports.is_empty() {
                continue;
            }
            let pkg = self.file_pkgs.own()[f];
            let plans = self.package_export_plans(file, pkg);
            let forwarders = self.jvm_forwarders(&plans);
            if !forwarders.is_empty() {
                out.files.insert(file, forwarders);
            }
        }
        out
    }

    fn jvm_forwarders(&mut self, plans: &[ClausePlan]) -> Vec<JvmForwarder> {
        let mut out = Vec::new();
        for plan in plans {
            let Some(q) = plan.q else { continue };
            for &(n, original, object) in &plan.objects {
                let Some(object) = object else { continue };
                out.push(JvmForwarder { name: n, target: JvmTarget::Object(object), q, selected: original, relayed: false, sig: None });
            }
            for &(n, sym, selected) in &plan.forwards {
                let alts: Vec<SymId> = self.syms.alternatives(sym).map_or_else(|| vec![sym], |a| a.to_vec());
                for a in alts {
                    let info = self.syms.sym(a);
                    if info.mods & mods::INLINE != 0 || info.intrinsic.is_some() {
                        continue;
                    }
                    let relayed = !self.qualifier_has(q, info.owner);
                    let sig = Arc::new(self.export_seen_through(a, q));
                    out.push(JvmForwarder { name: n, target: JvmTarget::Member(a), q, selected, relayed, sig: Some(sig) });
                }
            }
        }
        out
    }
}

/// What an exported name forwards to: the member and the object it is reached through.
pub fn export_target(w: &Worker, scope: &ExportScope, n: Name) -> Option<(Option<ClassId>, SymId)> {
    let r = scope.terms.get(&n)?;
    let sym = r.sym()?;
    let obj = match r {
        TermRef::ModuleMember(o, _) => Some(*o),
        _ => match w.syms.sym(sym).owner {
            Owner::Class(o) => Some(o),
            _ => None,
        },
    };
    Some((obj, sym))
}

/// Whether the name a table exports is a case class's, whose companion scalac makes and
/// exports beside it (the companion of the program's own case class has no symbol of its own).
pub fn exports_companion(w: &Worker, scope: &ExportScope, n: Name) -> bool {
    match scope.terms.get(&n) {
        Some(TermRef::Class(k)) => w.syms.class(*k).mods & crate::ast::mods::CASE != 0,
        _ => false,
    }
}

/// The class of the object `sym`.
fn object_class(w: &Worker, sym: SymId) -> Option<ClassId> {
    match w.syms.sym(sym).kind {
        SymKind::Object(k) => Some(k),
        _ => None,
    }
}

/// The case class whose companion a table exports under `n`.
fn companion_class(scope: &ExportScope, n: Name) -> Option<ClassId> {
    match scope.terms.get(&n) {
        Some(TermRef::Class(k)) => Some(*k),
        _ => None,
    }
}

fn qualifier_has_class(w: &Worker, q: ExportQualifier, scope: &ExportScope, n: Name) -> bool {
    match scope.terms.get(&n) {
        Some(TermRef::Class(k)) => w.qualifier_has(q, w.syms.class(*k).owner),
        _ => false,
    }
}
