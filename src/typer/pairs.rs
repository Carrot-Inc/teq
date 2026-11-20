//! The pairs of a class: every member of a base class that overrides or implements a member of a
//! later base class, both seen from the class's `this`, enumerated once where the class is
//! (dotty's `OverridingPairs.Cursor` over the class, OverridingPairs.scala 28-200). The back ends
//! read a class's pairs whose overriding member it inherits (a trait's member implementing
//! another trait's declaration in the class that mixes both): the JavaScript name a call through
//! the declaration dispatches by, the accessor of a stored member implementing a method, the
//! default getter a call takes, the JVM's bridges and the interpreter's dispatch.

use super::check::Agreement;
use super::Worker;
use crate::intern::Name;
use crate::names;
use crate::symbols::*;
use crate::tir::ClassPair;
use crate::types::*;

impl Worker<'_> {
    /// The pairs of the class `c`, in the cursor's order: for each name, the members `c` and its
    /// bases declare entered deferred ones first and then concrete ones, each from the root of the
    /// linearisation to `c`; an entry entered later overrides each earlier one of another owner
    /// that it matches, which it leaves no longer a candidate to override. Constructors, private
    /// members, type members and extension methods take no part. Only the names two of `c` and its
    /// bases have are entered (`shared_name_bits`).
    pub(super) fn class_pairs(&mut self, c: ClassId, own_names: bool) -> Vec<ClassPair> {
        // The record keeps the pairs whose overriding member `c` inherits: a member of a base over another base's,
        // whose name two bases share, or over an abstract member of `c` itself, which only a class with one declares
        // (an abstract class; a deferred given's). Their names' bits: the bases' shared ones, `c`'s own taken in
        // where `own_names` (such a class, and the dump's twin check, which enters every name); `c`'s own members of
        // those names are entered with them.
        let shared = self.shared_name_bits(c, own_names);
        if shared == 0 {
            return Vec::new();
        }
        // The candidates with their name and their base's index, entered from the root to `c` and each
        // class's members last to first, as `fillDecls` enters them; a base none of whose names is shared
        // has none. The masks' bits are shared by names apart too: most classes so met have no name twice,
        // and nothing further is made for them.
        let mut found: Vec<(Name, usize, SymId)> = Vec::new();
        let count = self.syms.class(c).base_types.len();
        for i in (0..count).rev() {
            let k = self.syms.class(c).base_types[i].0;
            if self.syms.class(k).names_mask & shared == 0 {
                continue;
            }
            for j in (0..self.syms.class(k).member_order.len()).rev() {
                let m = self.syms.class(k).member_order[j];
                let name = self.syms.sym(m).name;
                if crate::symbols::name_bit(name) & shared != 0 && name != names::INIT {
                    found.push((name, i, m));
                }
            }
        }
        // A name declared once is in no pair; the others' entries in the cursor's order: the deferred ones
        // first, then the concrete ones, each in the order entered (a stable sort by name keeps that order).
        found.sort_by_key(|&(name, _, _)| name);
        if !found.windows(2).any(|w| w[0].0 == w[1].0) {
            return Vec::new();
        }
        let bases = self.met_base_types(c);
        let mut entries: Vec<(Name, bool, usize, SymId)> = Vec::new();
        let mut at = 0;
        while at < found.len() {
            let name = found[at].0;
            let end = at + found[at..].iter().take_while(|e| e.0 == name).count();
            if end - at > 1 {
                let first = entries.len();
                for &(_, i, m) in &found[at..end] {
                    if !self.is_private(m) {
                        entries.push((name, !self.is_abstract_member(m), i, m));
                    }
                }
                entries[first..].sort_by_key(|&(_, concrete, _, _)| concrete);
            }
            at = end;
        }
        let superclass = self.syms.class(c).superclass;
        let mut pairs = Vec::new();
        let mut visited: Vec<SymId> = Vec::new();
        let mut start = 0;
        while start < entries.len() {
            let name = entries[start].0;
            let end = start + entries[start..].iter().take_while(|e| e.0 == name).count();
            let list = &entries[start..end];
            visited.clear();
            for o in (1..list.len()).rev() {
                let (_, _, oi, overriding) = list[o];
                if visited.contains(&overriding) {
                    continue;
                }
                for p in (0..o).rev() {
                    let (_, _, pi, overridden) = list[p];
                    if bases[oi].0 == bases[pi].0 || !self.pair_matches(c, (oi > 0).then_some(bases[oi].1), overriding, bases[pi].1, overridden) {
                        continue;
                    }
                    visited.push(overridden);
                    let bridged = superclass.is_some_and(|s| self.has_pair_as_own(s, overriding, overridden, true));
                    pairs.push(ClassPair { overriding, overridden, bridged });
                }
            }
            start = end;
        }
        pairs
    }

    /// The name bits that two of `c`'s bases have (their names masks, which hold every name of their
    /// tables), `c` among them where `with_own`: a name without its bit there is declared by one of them at most
    /// and is in no pair of theirs. None, and `c` has no such pair.
    fn shared_name_bits(&self, c: ClassId, with_own: bool) -> u64 {
        let (mut seen, mut shared) = (0u64, 0u64);
        let from = if with_own { 0 } else { 1 };
        for &(b, _) in self.syms.class(c).base_types.iter().skip(from) {
            let mask = self.syms.class(b).names_mask;
            shared |= seen & mask;
            seen |= mask;
        }
        shared
    }

    /// The cursor's `matches`: `overriding` (its owner of type `o_owner` in `c`, `None` for `c`'s
    /// own) and `overridden` (its owner of type `p_owner`) take the same parameters seen from `c`.
    fn pair_matches(&mut self, c: ClassId, o_owner: Option<TypeId>, overriding: SymId, p_owner: TypeId, overridden: SymId) -> bool {
        let osig = self.sig_arc(overriding);
        self.compare_sigs_of(c, Some(overriding), osig, o_owner, overridden, p_owner) != Agreement::Params
    }

    /// Whether a direct parent of `c` has the pair as its own (dotty's `isHandledByParent`), which
    /// the checks leave to that parent: asked of a pair of `class_pairs(c)` where it is read (the
    /// checks, the dump), not made with every pair.
    pub(super) fn pair_handled(&mut self, c: ClassId, pair: &ClassPair) -> bool {
        let bases = self.met_base_types(c);
        let (Owner::Class(ob), Owner::Class(pb)) = (self.syms.sym(pair.overriding).owner, self.syms.sym(pair.overridden).owner) else { return false };
        let (Some(oi), Some(pi)) = (bases.iter().position(|&(b, _)| b == ob), bases.iter().position(|&(b, _)| b == pb)) else { return false };
        let parents: Vec<(ClassId, TypeId)> = self.syms.class(c).parents.clone().into_iter().filter_map(|p| self.class_of(p).map(|k| (k, p))).collect();
        self.handled_by_parent(c, &bases, &parents, pair.overriding, oi, pair.overridden, pi)
    }

    /// dotty's `isHandledByParent` (OverridingPairs 84-100) with `RefChecks.canBeHandledByParent`
    /// (239-244): a direct parent derives from both owners, at `c`'s instance of an owner with type
    /// parameters (`isSubParent`), and has the pair as its own: matched seen from it, in its
    /// linearisation's order or it Java's, the overridden member no `abstract override`.
    #[allow(clippy::too_many_arguments)]
    fn handled_by_parent(&mut self, c: ClassId, bases: &[(ClassId, TypeId)], parents: &[(ClassId, TypeId)], overriding: SymId, oi: usize, overridden: SymId, pi: usize) -> bool {
        if oi == 0 || self.is_abstract_override(overridden) {
            return false;
        }
        let (ob, pb) = (bases[oi].0, bases[pi].0);
        for &(k, kt) in parents {
            if !self.sub_parent(c, k, kt, ob) || !self.sub_parent(c, k, kt, pb) {
                continue;
            }
            let java = self.is_java_class(k);
            if self.has_pair_as_own(k, overriding, overridden, java) {
                return true;
            }
        }
        false
    }

    /// dotty's `isSubParent`: the parent `k` (of type `kt` in `c`) derives from `bc`, and `bc` has
    /// no type parameters or `k` gives it `c`'s instance.
    fn sub_parent(&mut self, c: ClassId, k: ClassId, kt: TypeId, bc: ClassId) -> bool {
        if !self.syms.class(k).base_types.iter().any(|&(b, _)| b == bc) {
            return false;
        }
        if self.syms.class(bc).tparams.is_empty() {
            return true;
        }
        match (self.parent_base_instance(kt, bc), self.base_type_of(c, bc)) {
            (Some(a), Some(b)) => a == b || self.same_type(a, b),
            _ => false,
        }
    }

    /// Whether the class `k` has the pair as its own: it derives from both owners, they match seen
    /// from it, and the overriding one's owner comes first in its linearisation (dotty's
    /// `inLinearizationOrder`) unless `any_order` (a Java parent; the bridges' superclass, whose
    /// cursor asks the match alone).
    fn has_pair_as_own(&mut self, k: ClassId, overriding: SymId, overridden: SymId, any_order: bool) -> bool {
        let (Owner::Class(ob), Owner::Class(pb)) = (self.syms.sym(overriding).owner, self.syms.sym(overridden).owner) else { return false };
        let kb = &self.syms.class(k).base_types;
        let (Some(oi), Some(pi)) = (kb.iter().position(|&(b, _)| b == ob), kb.iter().position(|&(b, _)| b == pb)) else { return false };
        if !any_order && oi > pi {
            return false;
        }
        let (ot, pt) = (self.base_instance_at(k, oi), self.base_instance_at(k, pi));
        self.pair_matches(k, (oi > 0).then_some(ot), overriding, pt, overridden)
    }

    /// Records the pairs of the classes of the program that `of` keeps (`record_pairs_at`), and the
    /// program's inherited defaults: those of each class recorded where `every_class` (the final
    /// passes), gathered from every class's pairs again otherwise. A check that only reports
    /// (`emits_bodies` off) records none: no back end reads them.
    pub(super) fn record_class_pairs(&mut self, of: impl Fn(&Symbols, ClassId) -> bool, every_class: bool) {
        // A check that only reports has no back end to read them.
        if !self.emits_bodies {
            return;
        }
        if every_class {
            self.prog.inherited_defaults = Default::default();
        }
        for i in 0..self.prog.classes.len() {
            let c = self.prog.classes[i].id;
            if of(&self.syms, c) && self.record_pairs_at(i) && every_class {
                self.add_inherited_defaults_at(i);
            }
        }
        // A session's re-typed classes: the others' defaults stand, and every class's are gathered again.
        if !every_class {
            self.gather_inherited_defaults();
        }
    }

    /// The pairs of the class at `i` of the program's classes: those whose overriding member it
    /// inherits, with a body or a value, are kept on its record for the back ends
    /// (`TClassLists::inherited_pairs`), and a stored member that one of them makes implement a
    /// method is read through its accessor. Whether it kept any.
    pub(super) fn record_pairs_at(&mut self, i: usize) -> bool {
        if !self.emits_bodies {
            return false;
        }
        let c = self.prog.classes[i].id;
        // The pairs' dump writes every pair of every class, a trait's too, for the twin check; what the record
        // keeps and acts on below is the same with the dump as without it.
        if crate::tir::pairs_dump().is_some() {
            let every = self.class_pairs(c, true);
            self.dump_class_pairs(c, &every);
        }
        // A trait's pairs are read through the classes that mix it in, each of which has them (every reader is
        // a class's: its bridges, an instance's dispatch, the accessor and the default the class's pair settles),
        // as dotty's bridges are a class's alone.
        if self.syms.class(c).kind == ClassKind::Trait {
            if crate::tir::pairs_dump().is_some() {
                crate::tir::pairs_dump_write(&format!("{}\tnot-recorded\ttrait\t@{}\n", self.class_path(c), c.0));
            }
            return false;
        }
        let own_names = self.syms.class(c).mods & crate::ast::mods::ABSTRACT != 0 || !self.prog.classes[i].deferred_givens.is_empty();
        let pairs = self.class_pairs(c, own_names);
        let mut inherited: Vec<ClassPair> = pairs.into_iter().filter(|p| self.syms.sym(p.overriding).owner != Owner::Class(c) && !self.is_abstract_member(p.overriding)).collect();
        if crate::tir::pairs_control().is_some() {
            self.apply_pairs_control(c, &mut inherited);
        }
        let any = !inherited.is_empty();
        let tc = &mut self.prog.classes[i];
        match tc.lists.as_deref_mut() {
            Some(l) => l.inherited_pairs = inherited,
            None if any => tc.lists = Some(Box::new(crate::tir::TClassLists { inherited_pairs: inherited, ..Default::default() })),
            None => {}
        }
        // Every reader reads the stored record: the accessor here, the back ends after the passes, and the dump.
        let stored = self.prog.classes[i].inherited_pairs().to_vec();
        for p in &stored {
            self.reach_through_accessor(p.overriding, p.overridden);
        }
        self.dump_stored_pairs(c, &stored);
        any
    }

    /// Writes what `c` keeps (`TClassLists::inherited_pairs`), read from the store as the readers read it: a `stored`
    /// line a pair, with its bridged mark and the two ids, or `stored-none` where it keeps none.
    fn dump_stored_pairs(&mut self, c: ClassId, stored: &[ClassPair]) {
        if crate::tir::pairs_dump().is_none() {
            return;
        }
        let class = self.class_path(c);
        let mut out = String::new();
        if stored.is_empty() {
            out.push_str(&format!("{class}\tstored-none\t@{}\n", c.0));
        }
        for p in stored {
            let (o, d) = (self.member_text(p.overriding), self.member_text(p.overridden));
            out.push_str(&format!("{class}\tstored\t{o}\t{d}\tbridged={}\t{}\t{}\t@{}\n", p.bridged, p.overriding.0, p.overridden.0, c.0));
        }
        crate::tir::pairs_dump_write(&out);
    }

    /// The proofs' control (`crate::tir::pairs_control`) on the pairs `c` is about to keep: the named pair dropped,
    /// its target swapped or its bridged mark cleared, each change written to the dump as a `control` line.
    fn apply_pairs_control(&mut self, c: ClassId, kept: &mut Vec<ClassPair>) {
        let Some((kind, class, member, other)) = crate::tir::pairs_control() else { return };
        if class != "*" && self.name_str(self.syms.class(c).name) != class.as_str() {
            return;
        }
        let mut fired: Vec<(ClassPair, Option<ClassPair>)> = Vec::new();
        let mut k = 0;
        while k < kept.len() {
            let p = kept[k];
            if self.name_str(self.syms.sym(p.overridden).name) != member.as_str() {
                k += 1;
                continue;
            }
            match kind.as_str() {
                "drop" => {
                    kept.remove(k);
                    fired.push((p, None));
                    continue;
                }
                "unbridge" if p.bridged => {
                    kept[k].bridged = false;
                    fired.push((p, Some(kept[k])));
                }
                "swap" => {
                    if let Owner::Class(owner) = self.syms.sym(p.overriding).owner {
                        let name = self.syms.sym(p.overriding).name;
                        let order = self.syms.class(owner).member_order.clone();
                        let target = match other {
                            Some(o) => order.iter().copied().find(|&s| self.name_str(self.syms.sym(s).name) == o.as_str()),
                            None => order.iter().copied().find(|&s| s != p.overriding && self.syms.sym(s).name == name),
                        };
                        if let Some(t) = target {
                            kept[k].overriding = t;
                            fired.push((p, Some(kept[k])));
                        }
                    }
                }
                _ => {}
            }
            k += 1;
        }
        for (before, after) in fired {
            let changed = match after {
                None => String::from("-"),
                Some(a) => format!("{} bridged={}", self.member_text(a.overriding), a.bridged),
            };
            let line = format!("{}\tcontrol\t{kind}\t{}\t{}\t{changed}\t@{}\n", self.class_path(c), self.member_text(before.overriding), self.member_text(before.overridden), c.0);
            crate::tir::pairs_dump_write(&line);
        }
    }

    /// The program's inherited defaults (`Program::inherited_defaults`), made from every class's
    /// inherited pairs: a method implementing one that declares a default.
    fn gather_inherited_defaults(&mut self) {
        self.prog.inherited_defaults = Default::default();
        for i in 0..self.prog.classes.len() {
            self.add_inherited_defaults_at(i);
        }
    }

    /// Adds the inherited defaults the pairs of the class at `i` make.
    pub(super) fn add_inherited_defaults_at(&mut self, i: usize) {
        for j in 0..self.prog.classes[i].inherited_pairs().len() {
            let p = self.prog.classes[i].inherited_pairs()[j];
            if self.syms.sym(p.overriding).kind != SymKind::Def {
                continue;
            }
            let declares = self.syms.sym(p.overridden).sig.as_ref().is_some_and(|sig| sig.clauses.iter().any(|cl| cl.params.iter().any(|q| q.has_default)));
            if declares {
                let list = self.prog.inherited_defaults.entry(p.overriding).or_default();
                if !list.contains(&p.overridden) {
                    list.push(p.overridden);
                }
            }
        }
    }

    /// Writes the pairs of `c` to the pairs' dump (`crate::tir::pairs_dump`), a line each: the class, whether the
    /// overriding member is the class's own, the two members, the two bits, their ids, whether the overriding member
    /// is one the override check walks (`def`: a definition, a selector, a member object or a parameter field) and
    /// whether the class is the std's, then the class's id.
    pub(super) fn dump_class_pairs(&mut self, c: ClassId, pairs: &[ClassPair]) {
        if crate::tir::pairs_dump().is_none() {
            return;
        }
        let class = self.class_path(c);
        let std = if self.source(self.syms.class(c).file).is_std { "std" } else { "program" };
        let mut out = String::new();
        if pairs.is_empty() {
            out.push_str(&format!("{class}\tnone\t{std}\t@{}\n", c.0));
        }
        for p in pairs {
            let own = self.syms.sym(p.overriding).owner == Owner::Class(c);
            let (o, d) = (self.member_text(p.overriding), self.member_text(p.overridden));
            let handled = self.pair_handled(c, p);
            let walked = if self.walked_member(c, p.overriding) { "def" } else { "nodef" };
            // whether the record's rule keeps it (`record_pairs_at`): a class's pair whose overriding member it
            // inherits and which is not abstract; the twin holds the stored record to these
            let kept = if !own && self.syms.class(c).kind != ClassKind::Trait && !self.is_abstract_member(p.overriding) { "kept" } else { "-" };
            out.push_str(&format!(
                "{class}\t{}\t{o}\t{d}\thandled={handled}\tbridged={}\t{}\t{}\t{walked}\t{std}\t@{}\t{kept}\n",
                if own { "own" } else { "inherited" },
                p.bridged,
                p.overriding.0,
                p.overridden.0,
                c.0
            ));
        }
        crate::tir::pairs_dump_write(&out);
    }

    /// Writes a pair the override check found (`check_overrides_of`) to the pairs' dump: a `walk` line.
    pub(super) fn dump_walk_pair(&mut self, c: ClassId, m: SymId, p: SymId) {
        if crate::tir::pairs_dump().is_none() {
            return;
        }
        let line = format!("{}\twalk\t{}\t{}\n", self.class_path(c), self.member_text(m), self.member_text(p));
        crate::tir::pairs_dump_write(&line);
    }

    /// Writes, for every own pair of the class at `i` of the program's classes once the names are settled, the
    /// names the two members go by, whether a bridge of the class answers the overridden one's name by the
    /// overriding one, and of a stored member implementing a method whether it goes by its accessor: `name` lines
    /// of the pairs' dump.
    pub(super) fn dump_own_names_at(&mut self, i: usize) {
        if !self.emits_bodies {
            return;
        }
        let c = self.prog.classes[i].id;
        // A trait's names are answered in the classes that mix it in, by their bridges (a class's, never a
        // trait's): its own pairs are their inherited ones; an abstract class's are answered in its concrete
        // subclasses (`bridges_of` there).
        let info = self.syms.class(c);
        if self.source(info.file).is_std || info.kind == ClassKind::Trait || info.mods & crate::ast::mods::ABSTRACT != 0 {
            return;
        }
        let pairs = self.class_pairs(c, true);
        let bridges = self.prog.classes[i].bridges.clone();
        let class = self.class_path(c);
        let mut out = String::new();
        for p in pairs {
            let (o, d) = (p.overriding, p.overridden);
            // A member the std declares goes by the runtime's names (a function trait's `apply`), not the walk's.
            if self.syms.sym(o).owner != Owner::Class(c) || !self.walked_member(c, o) || self.syms.sym(o).is_extension || self.in_std(d) {
                continue;
            }
            let (no, nd) = (self.syms.dispatch_name(o), self.syms.dispatch_name(d));
            let bridged = bridges.iter().any(|&(x, y)| y == o && self.syms.dispatch_name(x) == nd);
            let info = self.syms.sym(o);
            let stored = matches!(info.kind, SymKind::Val | SymKind::Var) || (info.kind == SymKind::Given && info.mods & crate::ast::mods::LAZY != 0);
            let accessor = if self.jvm || !stored || self.syms.sym(d).kind != SymKind::Def {
                "-"
            } else if self.syms.sym(o).needs_accessor {
                "accessor"
            } else {
                "no-accessor"
            };
            let (ot, dt) = (self.member_text(o), self.member_text(d));
            out.push_str(&format!(
                "{class}\tname\t{ot}\t{dt}\t{}\t{}\t{}\t{accessor}\n",
                self.name_str(no),
                self.name_str(nd),
                if bridged { "bridged" } else { "-" }
            ));
        }
        crate::tir::pairs_dump_write(&out);
    }

    /// Whether the override check walks the member `m` of `c` (`check_overrides_of`): one with a
    /// definition, a selector, a member object, or a constructor parameter that is a field.
    fn walked_member(&self, c: ClassId, m: SymId) -> bool {
        let info = self.syms.sym(m);
        let m_mods = info.mods;
        info.def.is_some() || m_mods & crate::ast::mods::SELECTOR != 0 || (m_mods & crate::ast::mods::FIELD != 0 && self.is_ctor_param(c, m)) || self.is_member_object(m)
    }

    /// A member as the dump writes it: its owner's path, its name and its signature.
    fn member_text(&mut self, m: SymId) -> String {
        let owner = match self.syms.sym(m).owner {
            Owner::Class(k) => self.class_path(k),
            _ => String::new(),
        };
        let name = self.name_str(self.syms.sym(m).name).to_string();
        let sig = self.sig_string(m);
        format!("{owner}.{name}{sig}")
    }
}
