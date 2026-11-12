//! Whether an entry of `teq fix`'s ForbiddenSymbol names a symbol of the program typed (docs/FIX.md, "The rules teq
//! ships"): an entry is a symbol by its owners' names (`scalafix`'s `SymbolOps.normalize`: `a.b.C.m.`), so it names
//! one where a package, a class or an object stands at each name and the last is a package of the one before it, or
//! a class, an object, a type or a member it declares. A member a class inherits is no symbol of that class
//! (`scala.collection.IterableOnce.min` names none: `min` is `IterableOnceOps`'). An entry no job of a run resolves
//! is told once (`fix::engine`).
//!
//! On a job typed under teq's lean std a name of scala-library is looked up as the std binds it (`STD_BINDINGS`,
//! through `pkg_type`), and a member the std has as an extension by the std's library naming, the allowlist of
//! scala-library's shapes the TASTy writer selects by (`tasty::write::std_shape_owned`). That allowlist keys a name
//! under every class it resolves on, so a lean job may take an inherited member for a symbol: it then tells no entry
//! dead that a JVM job, which reads scala-library itself, would tell.

use super::super::resolve::{TermRef, TypeRef};
use super::super::Worker;
use crate::symbols::*;
use crate::types::*;

#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Pkg(PkgId),
    Class(ClassId),
    /// A method, whose parameters and type parameters are symbols of its own (`m().(x)`, `m().[T]`).
    Method(SymId),
}

impl<'a> Worker<'a> {
    /// Whether the normalized symbol `entry` names a symbol; where not, what is missing.
    pub fn sem_entry(&mut self, entry: &str) -> Result<(), String> {
        let names = segments(entry);
        let Some((last, owners)) = names.split_last() else { return Err("it names nothing".to_string()) };
        let owner = owners.join(".");
        let mut scopes = vec![Scope::Pkg(ROOT_PKG)];
        let mut missing = None;
        for (i, name) in owners.iter().enumerate() {
            let next = self.sem_entry_step(&scopes, name, true);
            if next.is_empty() {
                missing = Some(format!("there is no {}", owners[..=i].join(".")));
                break;
            }
            scopes = next;
        }
        if missing.is_none() && !self.sem_entry_step(&scopes, last, false).is_empty() {
            return Ok(());
        }
        // A member teq's std has as an extension of the class scala-library declares it in.
        if self.sem_lean() && !owner.is_empty() && crate::tasty::write::std_shape_owned(owners, last) {
            return Ok(());
        }
        Err(missing.unwrap_or_else(|| if owner.is_empty() { format!("there is no {}", last) } else { format!("{} has no member {}", owner, last) }))
    }

    /// Whether the job's std is teq's lean one, which binds scala-library's names to its own classes.
    fn sem_lean(&self) -> bool {
        self.loaded.as_ref().map_or(true, |l| !l.scala_library)
    }

    /// The package `path` names, where the job has it.
    fn sem_pkg(&self, path: &str) -> Option<PkgId> {
        let mut p = ROOT_PKG;
        for name in path.split('.') {
            let n = self.interner.intern(name);
            p = self.syms.entry_named(self.interner, p, n).and_then(|e| e.pkg)?;
        }
        Some(p)
    }

    /// What `name` names in one of `scopes`: a package, a class or an object to look further in (`owners`), or
    /// whatever the scope declares under the name. Each scope is looked in once, with what stands for it under the
    /// same name of an entry, found once before the lookups: a class's companion (a class and its companion are one
    /// name), and under the lean std the std's package of a class scala-library has in this one (`STD_BINDINGS`:
    /// `scala.collection.IterableOps` is the std's `scala.IterableOps`).
    fn sem_entry_step(&mut self, scopes: &[Scope], name: &str, owners: bool) -> Vec<Scope> {
        // A name the job has not spelled yet may be a class of its class path, which the lookups below load.
        let n = self.interner.intern(name);
        let mut all: Vec<(Scope, bool)> = scopes.iter().map(|&s| (s, true)).collect();
        for &scope in scopes {
            match scope {
                Scope::Class(c) => {
                    self.complete_class(c);
                    if let Some(k) = self.syms.class(c).companion.filter(|&k| !all.iter().any(|(s, _)| *s == Scope::Class(k))) {
                        all.push((Scope::Class(k), true));
                    }
                }
                Scope::Pkg(p) if p != ROOT_PKG && self.sem_lean() => {
                    let here = self.pkg_description(p);
                    let bound = crate::typer::loader::compile::STD_BINDINGS.iter().find(|(from, bound, _)| *from == here && *bound == name).map(|&(_, _, to)| to);
                    if let Some(to) = bound.and_then(|to| self.sem_pkg(to)).filter(|&to| !all.iter().any(|(s, _)| *s == Scope::Pkg(to))) {
                        // The std's package stands for the class alone, not for its own subpackages.
                        all.push((Scope::Pkg(to), false));
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for (scope, packages) in all {
            match scope {
                Scope::Pkg(p) => {
                    if packages {
                        if let Some(sub) = self.syms.entry_named(self.interner, p, n).and_then(|e| e.pkg) {
                            out.push(Scope::Pkg(sub));
                        }
                    }
                    // A file's module of top-level definitions, `<File>$package`, and a `package object`, which teq
                    // reads as a block of its package: their members are the package's.
                    if owners && (name.ends_with("$package") || name == "package") {
                        out.push(scope);
                    }
                    match self.quietly(|w| w.pkg_type(p, n)) {
                        Some(TypeRef::Class(c)) => out.push(Scope::Class(c)),
                        Some(_) if !owners => out.push(scope),
                        _ => {}
                    }
                    // A top-level extension method, which no lookup by a plain name gives.
                    let extensions: Vec<SymId> = self.syms.entry_named(self.interner, p, n).map(|e| e.extensions.clone()).unwrap_or_default();
                    for m in extensions {
                        out.push(if owners { Scope::Method(m) } else { scope });
                    }
                    match self.quietly(|w| w.pkg_term(p, n)) {
                        Some(TermRef::Class(c)) => out.push(Scope::Class(c)),
                        Some(TermRef::Global(s)) => match self.syms.sym(s).kind {
                            SymKind::Object(c) => out.push(Scope::Class(c)),
                            _ if !owners => out.push(scope),
                            SymKind::Def | SymKind::Given => out.push(Scope::Method(s)),
                            _ => {}
                        },
                        Some(_) if !owners => out.push(scope),
                        _ => {}
                    }
                }
                Scope::Class(c) => {
                    self.complete_class(c);
                    let (member, nested, alias) = {
                        let info = self.syms.class(c);
                        (info.members.get(&n).copied(), info.nested.get(&n).copied(), info.type_aliases.contains_key(&n))
                    };
                    if let Some(k) = nested {
                        out.push(Scope::Class(k));
                    }
                    if let Some(m) = member {
                        // A member the class declares, an alternative of an overloaded name among them.
                        let declared: Vec<SymId> = match self.syms.alternatives(m) {
                            Some(alts) => alts.iter().copied().filter(|&a| self.syms.sym(a).owner == Owner::Class(c)).collect(),
                            None if self.syms.sym(m).owner == Owner::Class(c) => vec![m],
                            None => Vec::new(),
                        };
                        for m in declared {
                            match self.syms.sym(m).kind {
                                SymKind::Object(k) => out.push(Scope::Class(k)),
                                _ if !owners => out.push(scope),
                                SymKind::Def | SymKind::Given => out.push(Scope::Method(m)),
                                _ => {}
                            }
                        }
                    }
                    if alias && !owners {
                        out.push(scope);
                    }
                    // An extension method the class declares.
                    let extensions: Vec<SymId> = self.syms.class(c).extensions.iter().copied().filter(|&m| self.syms.sym(m).name == n).collect();
                    for m in extensions {
                        out.push(if owners { Scope::Method(m) } else { scope });
                    }
                    // A type parameter of the class.
                    if !owners && self.syms.class(c).tparams.iter().any(|&tp| self.syms.tparam(tp).name == n) {
                        out.push(scope);
                    }
                }
                Scope::Method(m) => {
                    let sig = self.sig_arc(m);
                    let named = sig.clauses.iter().flat_map(|cl| cl.params.iter()).any(|q| q.name == n) || sig.tparams.iter().any(|&tp| self.syms.tparam(tp).name == n);
                    if named && !owners {
                        out.push(scope);
                    }
                }
            }
        }
        out.dedup_by(|a, b| a == b);
        out
    }
}

/// The names of a normalized symbol, `a.b.C.m.`: split at its dots, a name between backquotes whole.
fn segments(entry: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in entry.chars() {
        match c {
            '`' => quoted = !quoted,
            '.' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::segments;

    #[test]
    fn an_entry_is_its_owners_names() {
        assert_eq!(segments("scala.collection.IterableOps.init."), ["scala", "collection", "IterableOps", "init"]);
        assert_eq!(segments("japgolly.scalajs.react.vdom.`html_<^`.`^`."), ["japgolly", "scalajs", "react", "vdom", "html_<^", "^"]);
        assert_eq!(segments("a.`b.c`.d."), ["a", "b.c", "d"]);
        assert!(segments("").is_empty());
    }

    #[test]
    fn the_shapes_are_asked_by_an_owner_as_a_symbol_writes_it() {
        let owned = |names: &[&str], member: &str| crate::tasty::write::std_shape_owned(&names.iter().map(|n| n.to_string()).collect::<Vec<_>>(), member);
        assert!(owned(&["scala", "collection", "StringOps"], "last"));
        assert!(owned(&["scala", "Predef", "ArrowAssoc"], "->"));
        assert!(owned(&["scala", "math", "package"], "max"));
        assert!(!owned(&["scala", "math"], "max"));
        assert!(!owned(&["scala", "collection", "IterableOnce"], "min"));
    }
}
