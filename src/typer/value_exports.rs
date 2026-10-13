//! Export clauses whose qualifier is a value: `export info.{ply, color}`, `export chess.*`, `export
//! env.net.baseUrl`. An object's or a package's export is an alias (`exports`), which a reference
//! resolves to the original; a value's member is reached through the value, so such a clause makes
//! forwarders, as dotty's `Namer.exportForwarders` does: final members of the class, a method's
//! `final def m[T](a: A): R = path.m[T](a)` and a value's `final def v: V = path.v`, with the
//! member's signature as the path sees it. They are made once every class of the program is
//! complete (the signature phase's end, before the bodies fork), and their bodies when the class
//! is checked, so that every backend and the interpreter find them as ordinary methods, an
//! abstract member they implement included.

use super::Worker;
use crate::ast::{mods, DefKind, Import, ImportSel};
use crate::intern::Name;
use crate::source::Span;
use crate::symbols::*;
use crate::tir::{TExpr, TExprId, TFun};
use crate::types::*;
use std::sync::Arc;

/// The qualifier of a value export: its steps and its path type (`C.this.info`, `O.v`).
pub(super) struct ValuePath {
    steps: Vec<ForwardStep>,
    path: TypeId,
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

    /// The qualifier of an export clause of `c` when it goes through a value: a member of the
    /// class (`info`, `env.net`) or a val of an object or a package (`Config.default`). `None` for a
    /// path of objects and packages alone, whose clause the export table takes.
    pub(super) fn value_export_path(&mut self, c: ClassId, path: &[Name]) -> Option<ValuePath> {
        let (&first, rest) = path.split_first()?;
        // The head is a member of the class, its own or inherited: found on its `this` type,
        // without the scope's lookup, which would build export tables before their time.
        let kind = self.syms.class(c).kind;
        let this_ty = self.syms.this_type(c);
        let (head, _) = self.find_member(this_ty, first)?;
        let mut steps = Vec::new();
        // The object or package the path stands on while it names no value.
        let mut module: Option<ClassId> = None;
        let pkg: Option<PkgId> = None;
        let mut path_ty = NOTHING;
        match self.syms.sym(head).kind {
            // A member object of an object is static; of a class, a value of the instance.
            SymKind::Object(o) if kind == ClassKind::Object => module = Some(o),
            SymKind::Overloaded(_) | SymKind::Object(_) => return None,
            _ => {
                steps.push(ForwardStep::This(head));
                let this = self.this_prefix(c);
                path_ty = self.types.mk(Type::Select(this, head));
            }
        }
        let mut pkg = pkg;
        for &seg in rest {
            if steps.is_empty() {
                // Still on objects and packages: an object goes on as such, a val starts the value.
                let next = match (module, pkg) {
                    (Some(o), _) => self.import_step_class(o, seg),
                    (None, Some(p)) => self.import_step_pkg(p, seg),
                    _ => None,
                };
                match next {
                    Some(super::ImportTarget::ClassAll(o)) => {
                        (module, pkg) = (Some(o), None);
                        continue;
                    }
                    Some(super::ImportTarget::PkgAll(p)) => {
                        (module, pkg) = (None, Some(p));
                        continue;
                    }
                    _ => {}
                }
                let o = module?;
                let prefix = self.types.class(o, &[]);
                let (s, _) = self.find_member(prefix, seg)?;
                if matches!(self.syms.sym(s).kind, SymKind::Object(_) | SymKind::Overloaded(_)) {
                    return None;
                }
                steps.push(ForwardStep::Module(o));
                steps.push(ForwardStep::Member(s));
                path_ty = self.types.mk(Type::Select(prefix, s));
                continue;
            }
            let under = self.widen_path(path_ty);
            let (s, _) = self.find_member(under, seg)?;
            if matches!(self.syms.sym(s).kind, SymKind::Overloaded(_)) {
                return None;
            }
            steps.push(ForwardStep::Member(s));
            path_ty = self.types.mk(Type::Select(path_ty, s));
        }
        if steps.is_empty() {
            return None;
        }
        Some(ValuePath { steps, path: path_ty })
    }

    /// Whether an export clause of `c` goes through a value, the clauses `value_export_path`
    /// resolves: decided on the path's symbols, without a signature, so that the table a type
    /// lookup builds while a val's type is inferred infers no val (`export foo.pin.*` beside
    /// `val foo = new Foo: ...`, as dotty's class completer sees `Foo` before the forwarders).
    pub(super) fn export_path_through_value(&mut self, c: ClassId, path: &[Name]) -> bool {
        let Some((&first, rest)) = path.split_first() else { return false };
        let kind = self.syms.class(c).kind;
        let this_ty = self.syms.this_type(c);
        let Some((head, _)) = self.find_member(this_ty, first) else { return false };
        let (mut module, mut pkg): (Option<ClassId>, Option<PkgId>) = match self.syms.sym(head).kind {
            SymKind::Object(o) if kind == ClassKind::Object => (Some(o), None),
            SymKind::Overloaded(_) | SymKind::Object(_) => return false,
            _ => return true,
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
                    let Some(o) = module else { return false };
                    let prefix = self.types.class(o, &[]);
                    let Some((s, _)) = self.find_member(prefix, seg) else { return false };
                    return !matches!(self.syms.sym(s).kind, SymKind::Object(_) | SymKind::Overloaded(_));
                }
            }
        }
        false
    }

    /// The forwarders of every program class's value exports, at the end of the signature phase.
    pub fn add_value_forwarders(&mut self) {
        for i in 0..self.syms.classes.len() as u32 {
            let c = ClassId(i);
            // A class with no export clause, the most, is passed over first, by what the namer
            // read of its body when it entered it (`ClassInfo::has_exports`).
            if !self.syms.class(c).has_exports {
                continue;
            }
            let clauses = self.export_clauses(c);
            if clauses.is_empty() {
                continue;
            }
            let file = self.syms.class(c).file;
            if self.in_jar(file) || self.std.slot_of(file).is_some() {
                continue;
            }
            // From inside the class, which decides what of the path it reaches: its frames and its
            // outer classes', without imports, which the forwarders resolve nothing through.
            let mut frames = Vec::new();
            let mut o = Owner::Class(c);
            while let Owner::Class(k) = o {
                frames.push(super::Frame::Class(k));
                o = self.syms.class(k).owner;
            }
            frames.reverse();
            let env = super::Env { file, frames, imports: Vec::new() };
            self.with_env(env, |t| t.class_value_forwarders(c, &clauses));
            self.mark_forwarders_meeting_inherited(c);
        }
    }

    /// A forwarder of a name an ancestor defines is an alternative beside the inherited ones, as
    /// a written member is (`mark_members_meeting_inherited`): `f(Int)` exported next to an
    /// inherited `f(String)` overloads it (dotty's `canForward` refuses only what overrides).
    fn mark_forwarders_meeting_inherited(&mut self, c: ClassId) {
        let names: Vec<Name> = self.syms.class(c).member_order.iter().filter(|&&m| self.syms.forward_of(m).is_some()).map(|&m| self.syms.sym(m).name).collect();
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
        let mut i = 0;
        while i < clauses.len() {
            // A clause's selectors share its path and the start of its span.
            let start = clauses[i].span.start;
            let mut j = i;
            while j < clauses.len() && clauses[j].span.start == start && clauses[j].path == clauses[i].path {
                j += 1;
            }
            let clause = &clauses[i..j];
            i = j;
            let Some(vp) = self.value_export_path(c, &clause[0].path) else {
                // The export table leaves a path through a value to this pass, its error too.
                if self.export_path_through_value(c, &clause[0].path) {
                    let path = &clause[0].path;
                    let head = self.lookup_term(path[0]);
                    if let Err(msg) = self.resolve_export_path(head, path) {
                        let file = self.syms.class(c).file;
                        self.diags.error(file, clause[0].span, msg);
                    }
                }
                continue;
            };
            // A path whose type is in error, an error already reported in its initializer, has a
            // member of every name, of the error type (dotty's `findMember` gives an error symbol,
            // `newErrorSymbol`): each named selector forwards to it silently, a wildcard to nothing.
            if self.widen_path(vp.path) == ERROR {
                self.error_forwarders(c, clause);
                continue;
            }
            let vp = Arc::new(vp);
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
                        let at = self.selector_span(c, imp, Some(n));
                        // The type of the name and the term (dotty's `addForwardersNamed` looks up
                        // `name` and `name.toTypeName`), in error only when neither is forwarded.
                        let typed = self.forward_type(c, &vp, n, alias, at);
                        if let Err(why) = self.forward_member(c, &vp, n, alias, at, &mut targets, None) {
                            if typed {
                                targets.push(alias);
                                continue;
                            }
                            let path = self.path_text(c, &vp, &clause[0].path);
                            let mut msg = format!("no eligible member {} at {}", self.name_str(n), path);
                            if let Some(why) = why {
                                msg.push_str(&format!("\n{}.{} cannot be exported because it {}", path, self.name_str(n), why));
                            }
                            let file = self.syms.class(c).file;
                            self.diags.error(file, at, msg);
                        }
                    }
                    ImportSel::Wildcard => {
                        let at = self.selector_span(c, imp, None);
                        for n in self.forwardable_names(&vp, false) {
                            if !named.contains(&n) {
                                let _ = self.forward_member(c, &vp, n, n, at, &mut targets, Some(false));
                            }
                        }
                        for n in self.forwardable_type_names(&vp) {
                            if !named.contains(&n) {
                                self.forward_type(c, &vp, n, n, at);
                            }
                        }
                    }
                    ImportSel::Given => {
                        let at = self.selector_span(c, imp, None);
                        for n in self.forwardable_names(&vp, true) {
                            if !named.contains(&n) {
                                let _ = self.forward_member(c, &vp, n, n, at, &mut targets, Some(true));
                            }
                        }
                    }
                }
            }
        }
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
        match self.name_str(name).as_str() {
            "hashCode" | "equals" | "toString" => any,
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
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(owner).base_types.clone();
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
    fn error_forwarders(&mut self, c: ClassId, clause: &[Import]) {
        for imp in clause {
            let ImportSel::Name(n, rename) = imp.sel else { continue };
            let alias = rename.unwrap_or(n);
            if self.name_str(alias) == "_" || self.syms.class(c).members.contains_key(&alias) {
                continue;
            }
            let at = self.selector_span(c, imp, Some(n));
            let file = self.syms.class(c).file;
            let s = self.syms.new_sym(alias, SymKind::Def, mods::FINAL, Owner::Class(c), file, None, at);
            {
                let mut info = self.syms.sym_mut(s);
                info.sig = Some(Arc::new(MethodSig::value(ERROR)));
                info.state().set(Completion::Done);
            }
            self.syms.add_member(c, s);
        }
    }

    /// The path as dotty prints a qualifier, a member of the class through its `this`.
    fn path_text(&self, c: ClassId, vp: &ValuePath, path: &[Name]) -> String {
        let names = path.iter().map(|&n| self.name_str(n)).collect::<Vec<_>>().join(".");
        match vp.steps.first() {
            Some(ForwardStep::This(_)) => format!("{}.this.{}", self.name_str(self.syms.class(c).name), names),
            _ => names,
        }
    }

    /// Where a selector stands in its clause: the name it selects (the last occurrence of its
    /// text in the clause), or a wildcard's or `given`'s token.
    fn selector_span(&self, c: ClassId, imp: &Import, name: Option<Name>) -> Span {
        let file = self.syms.class(c).file;
        let text = &self.source(file).text;
        let Some(clause) = text.get(imp.span.start as usize..imp.span.end as usize) else { return imp.span };
        let needle = match (name, &imp.sel) {
            (Some(n), _) => self.name_str(n),
            (None, ImportSel::Given) => "given".to_string(),
            _ => "*".to_string(),
        };
        let found = match &imp.sel {
            ImportSel::Name(..) => clause.find(&needle),
            _ => clause.rfind(&needle),
        };
        match found {
            Some(at) => Span { start: imp.span.start + at as u32, end: imp.span.start + (at + needle.len()) as u32 },
            None => imp.span,
        }
    }

    /// The names of the qualifier's members a wildcard (`givens`: a `given` selector) exports:
    /// public, not synthetic, no constructor, givens for `given` alone (dotty's
    /// `addWildcardForwarders`), each once, in the order of the qualifier's linearisation.
    fn forwardable_names(&mut self, vp: &ValuePath, givens: bool) -> Vec<Name> {
        let q = self.widen_path(vp.path);
        let Some(k) = self.class_of(q) else { return Vec::new() };
        self.complete_class(k);
        let from_case_class = self.syms.class(k).mods & mods::CASE != 0;
        let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
        let mut out: Vec<Name> = Vec::new();
        for b in bases {
            self.complete_class(b);
            let members: Vec<SymId> = self.syms.class(b).member_order.clone();
            for m in members {
                let info = self.syms.sym(m);
                let name = info.name;
                // Neither private nor synthetic members (dotty's `PrivateOrSynthetic`): a case
                // class's selectors `_1`..`_n` are the class's own (`mods::SELECTOR`), a library's
                // compiler-made members its pickle's (`Loaded::synthetic_members`).
                let synthetic = self.loaded.as_ref().is_some_and(|l| l.synthetic_members.contains_key(&m));
                if out.contains(&name) || info.mods & (mods::PRIVATE | mods::SELECTOR) != 0 || synthetic || info.scoped_private || name == crate::names::INIT {
                    continue;
                }
                let given = info.mods & mods::GIVEN != 0 || info.kind == SymKind::Given;
                if given != givens || matches!(info.kind, SymKind::Object(_)) {
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
        out
    }

    /// The type members a wildcard forwards through the path: its class's and its bases', the
    /// private ones left out (dotty's `membersBasedOnFlags(excluded = PrivateOrSynthetic)`).
    fn forwardable_type_names(&mut self, vp: &ValuePath) -> Vec<Name> {
        let q = self.widen_path(vp.path);
        let Some(k) = self.class_of(q) else { return Vec::new() };
        self.complete_class(k);
        let bases: Vec<ClassId> = self.syms.class(k).base_types.iter().map(|&(b, _)| b).collect();
        let mut out: Vec<Name> = Vec::new();
        for b in bases {
            self.complete_class(b);
            let aliases: Vec<(Name, AliasId)> = self.syms.class(b).type_aliases.iter().map(|(&n, &a)| (n, a)).collect();
            for (n, a) in aliases {
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
        out.sort_by_key(|n| n.0);
        out
    }

    /// The type forwarder of the path's type member `name` under `alias`, `type alias = p.name`
    /// (dotty's `addForwarder` of a type, a `TypeAlias` of `pathType.select(sym)`). A class that
    /// has a type of the name, its own or an earlier clause's forwarder, keeps it, the clause in
    /// error (`checkNoConflict`, E161). False where the path has no such member to forward.
    fn forward_type(&mut self, c: ClassId, vp: &ValuePath, name: Name, alias: Name, span: Span) -> bool {
        if !self.forwardable_type_names(vp).contains(&name) {
            return false;
        }
        let file = self.syms.class(c).file;
        let existing = {
            let info = self.syms.class(c);
            match (info.type_aliases.get(&alias), info.nested.get(&alias)) {
                (Some(_), _) => Some("type"),
                (None, Some(&k)) if self.syms.class(k).kind != ClassKind::Object => Some(if self.syms.class(k).kind == ClassKind::Trait { "trait" } else { "class" }),
                _ => None,
            }
        };
        if let Some(kind) = existing {
            let n = self.name_str(alias);
            let msg = format!("{n} is already defined as {kind} {n}");
            self.diags.error(file, span, msg);
            return true;
        }
        let rhs = self.member_type(vp.path, name).unwrap_or(ERROR);
        // A JavaScript type the forwarded alias names is the forwarder's too (`Worker::alias_names_js`).
        let js = self.forwarded_alias_js(vp, name);
        let a = AliasId(self.syms.aliases.len() as u32);
        self.syms.aliases.push(AliasInfo { name: alias, owner: Owner::Class(c), file, def: None, tparams: Vec::new(), rhs, bounds: None, deprecated: false, js, declared_rhs: crate::tir::NO_TYPE });
        self.syms.alias_cells.set(a.0, Completion::Done);
        self.syms.class_mut(c).type_aliases.insert(alias, a);
        true
    }

    /// Whether the alias `name` of the path's class or a base, the forwarded one, names a
    /// JavaScript type; false for a class member.
    fn forwarded_alias_js(&mut self, vp: &ValuePath, name: Name) -> bool {
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
    /// `givens`: a wildcard's filter of the alternatives, the given ones for a given selector and
    /// the others for `*` (dotty's `addWildcardForwarders`: a given member forwarded alone, the
    /// others through `memberBasedOnFlags(excluded = Given)`); a named selector forwards them all.
    fn forward_member(&mut self, c: ClassId, vp: &Arc<ValuePath>, name: Name, alias: Name, span: Span, targets: &mut Vec<Name>, givens: Option<bool>) -> Result<(), Option<String>> {
        let q = self.widen_path(vp.path);
        let Some((found, _)) = self.find_member(q, name) else { return Err(None) };
        if givens.is_none() && targets.contains(&alias) {
            return Err(Some("clashes with another export in the same export clause".to_string()));
        }
        let path_class = self.class_of(q);
        let mut reason: Option<String> = None;
        let alts: Vec<SymId> = match self.syms.alternatives(found) {
            Some(alts) => alts.to_vec(),
            None => vec![found],
        };
        let this_ty = self.syms.this_type(c);
        let mut made: Vec<SymId> = Vec::new();
        for m in alts {
            let (m_mods, m_name, m_kind, m_owner) = {
                let info = self.syms.sym(m);
                (info.mods, info.name, info.kind, info.owner)
            };
            if m_name == crate::names::INIT || matches!(m_kind, SymKind::Object(_)) {
                continue;
            }
            if givens.is_some_and(|g| g != (m_mods & mods::GIVEN != 0 || m_kind == SymKind::Given)) {
                continue;
            }
            let Owner::Class(owner) = m_owner else { continue };
            if !self.export_accessible(c, m, path_class) {
                reason = Some("is not accessible".to_string());
                continue;
            }
            let _ = m_mods;
            // What the class has from the member's owner already (dotty: "is already a member").
            if self.base_type(this_ty, owner).is_some() && (owner == c || !self.is_abstract_member(m)) {
                reason = Some(format!("is already a member of {}", self.name_str(self.syms.class(c).name)));
                continue;
            }
            // A member that overrides a concrete one the class has already is that one.
            if let Some(other) = self.overridden_member_of(c, m) {
                let o = self.syms.sym(other).owner;
                let at = match o { Owner::Class(k) => self.name_str(self.syms.class(k).name).to_string(), _ => String::new() };
                reason = Some(format!("overrides {} in {}, which is already a member of {}", self.name_str(m_name), at, self.name_str(self.syms.class(c).name)));
                continue;
            }
            // Each alternative of an overloaded member at its own offset of the selector, so that
            // the forwarders keep one order in every build (`layout::compare_syms`).
            let at = Span { start: span.start + (made.len() as u32).min(span.end.saturating_sub(span.start)), end: span.end };
            if let Some(s) = self.make_forwarder(c, vp, m, owner, alias, at) {
                made.push(s);
            }
        }
        if made.is_empty() {
            return Err(reason);
        }
        targets.push(alias);
        for s in made {
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

    /// A forwarder of `m`, a member of `owner` the qualifier has, as the path sees it: the
    /// owner's type parameters given by the qualifier's base type, its `this` the path.
    fn make_forwarder(&mut self, c: ClassId, vp: &Arc<ValuePath>, m: SymId, owner: ClassId, alias: Name, span: Span) -> Option<SymId> {
        let q = self.widen_path(vp.path);
        let base = self.base_type(q, owner)?;
        let subst = self.owner_subst(base);
        let sig = self.sig_of(m).clone();
        let file = self.syms.class(c).file;
        let seen = |t: &mut Self, ty: TypeId| {
            let ty = t.types.subst(ty, &subst);
            t.as_seen_from(ty, vp.path, owner)
        };
        let ret = seen(self, sig.ret);
        let clauses: Vec<ClauseSig> = sig
            .clauses
            .iter()
            .map(|cl| ClauseSig {
                params: cl
                    .params
                    .iter()
                    .map(|p| {
                        let ty = seen(self, p.ty);
                        let local_ty = match (p.repeated, self.seq_class()) {
                            (true, Some(seq)) => self.types.class(seq, &[ty]),
                            _ => ty,
                        };
                        let q = self.syms.new_sym(p.name, SymKind::Param, 0, Owner::Local, file, None, span);
                        {
                            let mut info = self.syms.sym_mut(q);
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
        let target = self.syms.sym(m);
        let (is_extension, ext_tparams, ext_clauses) = (target.is_extension, target.ext_tparams, target.ext_clauses);
        let mut retained = target.mods & (mods::GIVEN | mods::IMPLICIT | mods::INFIX | mods::EXTENSION);
        if target.kind == SymKind::Given {
            retained |= mods::GIVEN;
        }
        let s = self.syms.new_sym(alias, SymKind::Def, mods::FINAL | retained, Owner::Class(c), file, None, span);
        {
            let mut info = self.syms.sym_mut(s);
            info.sig = Some(Arc::new(MethodSig { tparams: sig.tparams.clone(), clauses, ret, declared: crate::tir::NO_TYPE }));
            info.state().set(Completion::Done);
            info.is_extension = is_extension;
            info.ext_tparams = ext_tparams;
            info.ext_clauses = ext_clauses;
        }
        self.syms.class_mut(c).forwarders.push((s, Arc::new(Forward { steps: vp.steps.clone(), target: m })));
        if retained & (mods::GIVEN | mods::IMPLICIT) != 0 {
            self.syms.class_mut(c).givens.push(s);
        }
        Some(s)
    }

    /// The bodies of the class's value forwarders, as its methods: the path's steps from `this`
    /// or the object, then the member called with the forwarder's parameters.
    pub(super) fn define_value_forwarders(&mut self, c: ClassId) -> Vec<crate::tir::FunId> {
        if self.syms.class(c).forwarders.is_empty() {
            return Vec::new();
        }
        let members: Vec<SymId> = self.syms.class(c).member_order.clone();
        let mut out = Vec::new();
        for s in members {
            let Some(fwd) = self.syms.forward_of(s) else { continue };
            if self.syms.sym(s).owner != Owner::Class(c) || self.fun_of_sym.contains_key(&s) {
                continue;
            }
            let span = self.syms.sym(s).span;
            let sig = self.sig_of(s).clone();
            let Some((r, rt)) = self.forward_path(c, &fwd, span) else { continue };
            let target = fwd.target;
            let params: Vec<SymId> = sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.sym)).collect();
            let body = if self.is_method_sym(target) {
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
                let owner_subst = match self.find_member(rt, self.syms.sym(target).name) {
                    Some((_, owner_ty)) => self.owner_subst(owner_ty),
                    None => Vec::new(),
                };
                let call = super::apply::MethodCall { recv: Some(r), sym: target, owner_subst, ext_recv: None, prefix: None, refined: None };
                let te = self.build_call(&call, list);
                self.prog.set_type(te, sig.ret);
                // The pickle's call names its type arguments, the forwarder's own parameters.
                let targs: Vec<TypeId> = sig.tparams.iter().map(|&t| self.types.param(t)).collect();
                if self.capturing() && !targs.is_empty() {
                    self.capture_call_targs(te, &targs);
                }
                te
            } else {
                let (te, _) = self.forward_select(r, rt, target, span);
                te
            };
            // A default of the member is the forwarder's too, its getter forwarding to the
            // member's on the path, `n$default$i(ps before) = q.m$default$i(ps before)` (dotty's
            // `exportForwarders`, `foreachDefaultGetterOf`): the getter's application a call
            // leaving the argument out makes (`TExpr::Default`), each with a path of its own.
            let mut defaults = vec![None; params.len()];
            if self.is_method_sym(target) {
                let flat: Vec<(usize, TypeId, bool)> = sig.clauses.iter().enumerate().flat_map(|(ci, cl)| cl.params.iter().map(move |p| (ci, p.ty, p.has_default))).collect();
                for (i, &(ci, ty, has_default)) in flat.iter().enumerate() {
                    if !has_default {
                        continue;
                    }
                    let Some((path, _)) = self.forward_path(c, &fwd, span) else { continue };
                    let before: usize = sig.clauses[..ci].iter().map(|cl| cl.params.len()).sum();
                    let mut items = vec![path];
                    for k in 0..before {
                        let e = self.prog.add(TExpr::Local(params[k]));
                        self.prog.set_type(e, flat[k].1);
                        items.push(e);
                    }
                    let l = self.prog.list(&items);
                    let d = self.prog.add(TExpr::Default(crate::tir::DefaultOf::Method(target), i as u32 | crate::tir::DEFAULT_RECEIVER, l));
                    self.prog.set_type(d, ty);
                    defaults[i] = Some(d);
                }
            }
            let f = self.prog.add_fun(TFun { sym: s, params, defaults, body: Some(body), body_unconsuming: false, captures: 0, lifted: false });
            self.fun_of_sym.insert(s, f);
            out.push(f);
        }
        out
    }

    /// The forwarder's qualifier as a tree, from `this` or the object along the path's steps.
    fn forward_path(&mut self, c: ClassId, fwd: &Forward, span: Span) -> Option<(TExprId, TypeId)> {
        let mut recv: Option<(TExprId, TypeId)> = None;
        for step in &fwd.steps {
            recv = Some(match (*step, recv) {
                (ForwardStep::This(m), _) => {
                    let this = self.prog.add(TExpr::This);
                    let this_ty = self.syms.this_type(c);
                    self.prog.set_type(this, this_ty);
                    self.forward_select(this, this_ty, m, span)
                }
                (ForwardStep::Module(o), _) => {
                    let te = self.prog.add(TExpr::Module(o));
                    let ty = self.types.class(o, &[]);
                    self.prog.set_type(te, ty);
                    (te, ty)
                }
                (ForwardStep::Member(m), Some((r, rt))) => self.forward_select(r, rt, m, span),
                (ForwardStep::Member(_), None) => return None,
            });
        }
        recv
    }

    /// `recv.m` for a member that takes no argument list: a field, a val's getter or a
    /// parameterless method, typed as a selection of it is.
    fn forward_select(&mut self, recv: TExprId, recv_ty: TypeId, m: SymId, span: Span) -> (TExprId, TypeId) {
        let callee = self.member_callee(recv, recv_ty, m);
        self.apply_callee(callee, None, Vec::new(), span, None)
    }
}
