//! The lean std's selections, checked against scala-library's shapes. A body that selects a std
//! definition writes the member of scala-library
//! or of the JDK that stands for it; the selection's key, `owner<TAB>name<TAB>shape` as
//! tests/tasty/stdshapes/Check.scala reads it, has to be one of `std_shapes.txt`, the keys that
//! resolve in scala-library 3.8.4 as scalac's unpickler resolves them, or the body is withheld.
//! `std_shapes` lists every key the std can give, which the checker filters into that file.

use super::term::{conversion_sig, sig_params, universal_member, universal_sig, StdTarget, ARRAY_ELEMS, UNIVERSAL};
use super::*;
use std::sync::OnceLock;

fn allowlist() -> &'static FxMap<&'static str, ()> {
    static SET: OnceLock<FxMap<&'static str, ()>> = OnceLock::new();
    SET.get_or_init(|| include_str!("../std_shapes.txt").lines().filter(|l| !l.is_empty()).map(|l| (l, ())).collect())
}

/// The scala-library classes a member of a wider result (a `receiver` entry's) is looked up in,
/// nearest first: the result's class and those of its ancestors the allowlist lists.
const WIDER_ANCESTORS: &[(&str, &[&str])] = &[
    ("scala.collection.Iterable", &["scala.collection.Iterable", "scala.collection.IterableOps"]),
    ("scala.collection.mutable.Buffer", &["scala.collection.mutable.Buffer", "scala.collection.SeqOps", "scala.collection.Iterable", "scala.collection.IterableOps"]),
];

/// The class scala-library's member `name`, selected by name on a value of the wider class
/// `class`, erases its result to (an `Iterable`'s `toList` a `List`; its `tail` an `Object`, the
/// collection's own `C`), from the nearest ancestor the allowlist has the member in.
pub(super) fn library_result_by_name(class: &str, name: &str) -> Option<&'static str> {
    let owners = WIDER_ANCESTORS.iter().find(|(c, _)| *c == class)?.1;
    owners.iter().find_map(|owner| {
        let prefix = format!("{}\t{}\t", owner, name);
        allowlist().keys().filter_map(|l| l.strip_prefix(prefix.as_str())).find_map(|shape| {
            let (params, ret) = shape.rsplit_once(':')?;
            (params == "-" || (params.starts_with('[') && !params.contains(','))).then_some(ret)
        })
    })
}

/// A hand entry of the inverse table (`std_inverse.txt`): a selection of the lean std that
/// scala-library has under another shape, and the one it has.
pub(super) struct Inverse {
    pub lean: (&'static str, &'static str, &'static str),
    pub library: (&'static str, &'static str, &'static str),
    /// The conversion scalac selects the member on: its object, its name and its shape.
    pub via: Option<(&'static str, &'static str, &'static str)>,
    /// The element type of a Java varargs parameter the lean member leaves out, which
    /// scala-library's call passes empty after its last clause's arguments (`varargs <E>`).
    pub appended: Option<&'static str>,
    /// Whether a `Char` argument is widened to the `Int` scala-library's parameter takes
    /// (`widened`), as scalac's adaptation of the argument does.
    pub widened: bool,
    /// The class scala-library's member returns, whose comparisons with an `Int` a comparison of
    /// the lean member's result selects (`compared <C>`).
    pub compared: Option<&'static str>,
    /// Whether scala-library's result is wider than the lean std's (`receiver`), which a binding
    /// or an argument of the lean type would not take.
    pub receiver: bool,
}

impl Inverse {
    /// Whether the selection is written alike under the two shapes: by name or as an object,
    /// with no conversion between.
    /// Whether a selection of the member is written only as the receiver of another selection,
    /// where its result is scala-library's: a `receiver` or a `compared` form.
    pub(super) fn receiver_only(&self) -> bool {
        self.receiver || self.compared.is_some()
    }

    pub(super) fn written_alike(&self) -> bool {
        let by_name = |shape: &str| shape == "object" || shape.starts_with("-:");
        self.via.is_none() && self.lean.1 == self.library.1 && by_name(self.lean.2) && by_name(self.library.2)
    }
}

pub(super) fn inverse_table() -> &'static [Inverse] {
    static TABLE: OnceLock<Vec<Inverse>> = OnceLock::new();
    TABLE.get_or_init(|| {
        include_str!("../std_inverse.txt")
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| {
                let f: Vec<&'static str> = l.split('\t').collect();
                let appended = f[6].strip_prefix("varargs ");
                let compared = f[6].strip_prefix("compared ");
                let widened = f[6] == "widened";
                let receiver = f[6] == "receiver";
                let via = (f[6] != "-" && appended.is_none() && compared.is_none() && !widened && !receiver).then(|| {
                    let v: Vec<&'static str> = f[6].split(' ').collect();
                    (v[0], v[1], v[2])
                });
                Inverse { lean: (f[0], f[1], f[2]), library: (f[3], f[4], f[5]), via, appended, widened, compared, receiver }
            })
            .collect()
    })
}

/// The library members (owner, name and shape) whose `Char` argument the table's `widened` form
/// writes as `Char.char2int(c)`.
pub(crate) fn widened_library_members() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    inverse_table().iter().filter(|e| e.widened).map(|e| e.library)
}

/// Whether a member of this name has an entry whose library result is wider than the lean
/// std's, which a selection then writes only as a receiver (`receiver_only`).
pub(super) fn may_be_receiver_only(name: &str) -> bool {
    inverse_table().iter().any(|e| e.receiver_only() && e.lean.1 == name)
}

pub(super) fn inverse_entry(owner: &str, name: &str, shape: &str) -> Option<&'static Inverse> {
    inverse_table().iter().find(|e| e.lean == (owner, name, shape))
}

/// A signature as a key spells it, read back: `None` for a selection by name or an object.
pub(super) fn sig_of_text(shape: &str) -> Option<(Vec<SigParam>, String)> {
    let (params, result) = shape.rsplit_once(':')?;
    if params == "-" {
        return None;
    }
    let params = params
        .split(',')
        .filter(|p| !p.is_empty())
        .map(|p| match p.strip_prefix('[').and_then(|p| p.strip_suffix(']')).and_then(|n| n.parse().ok()) {
            Some(n) => SigParam::Types(n),
            None => SigParam::Type(p.to_string()),
        })
        .collect();
    Some((params, result.to_string()))
}

/// A signature as the keys spell it: the parameters (`[n]` a type parameter clause), `:`, the
/// result.
pub(super) fn sig_text(params: &[SigParam], result: &str) -> String {
    let params: Vec<String> = params
        .iter()
        .map(|p| match p {
            SigParam::Types(n) => format!("[{}]", n),
            SigParam::Type(t) => t.clone(),
        })
        .collect();
    format!("{}:{}", params.join(","), result)
}

/// The shape of a selection by name alone (a val, a parameterless def), with its result's
/// erasure, which the selections on it depend on.
pub(super) fn by_name_shape(result: &str) -> String {
    format!("-:{}", result)
}

/// The class of a library member as a key names it: `scala.Predef$.ArrowAssoc` for a class of
/// `Predef`.
pub(super) fn library_owner(pkg: &str, class: &str) -> String {
    match pkg {
        "scala.Predef" => format!("scala.Predef$.{}", class),
        _ => format!("{}.{}", pkg, class),
    }
}

impl<'w, 'a> P<'w, 'a> {
    pub(super) fn is_std_file(&self, f: FileId) -> bool {
        self.w.files.as_slice().get(f.0 as usize).map_or(false, |src| src.is_std) && !self.w.in_jar(f)
    }

    pub(super) fn is_std_sym(&self, s: SymId) -> bool {
        self.is_std_file(self.w.syms.sym(s).file)
    }

    pub(super) fn is_std_class(&self, c: ClassId) -> bool {
        let info = self.w.syms.class(c);
        self.is_std_file(info.file) || info.def.is_none() && info.kind == ClassKind::Builtin
    }

    /// Whether the library member that stands for a std definition resolves as written; a body
    /// that would select it otherwise is withheld.
    pub(super) fn std_shape(&mut self, owner: &str, name: &str, shape: &str) -> bool {
        let key = format!("{}\t{}\t{}", owner, name, shape);
        if let Some(seen) = self.shapes_seen.as_mut() {
            seen.push(key.clone());
            return true;
        }
        if allowlist().contains_key(key.as_str()) || inverse_entry(owner, name, shape).map_or(false, |e| e.written_alike()) {
            return true;
        }
        let class = owner.rsplit('.').next().unwrap_or(owner).trim_end_matches('$');
        self.fail(format!("the std's {}.{}, which scala-library has under another shape", class, name));
        false
    }

    /// The key of scala-library's `unapply` of a std case class: its companion and the shape
    /// returning `Option`.
    pub(super) fn std_unapply_key(&mut self, c: ClassId) -> (String, String) {
        let cls = self.w.library_class_name(c, "");
        let n = self.w.syms.class(c).own_tparams().len();
        let mut sig = Vec::new();
        if n > 0 {
            sig.push(SigParam::Types(n));
        }
        sig.push(SigParam::Type(cls.clone()));
        (format!("{}$", cls), sig_text(&sig, "scala.Option"))
    }

    /// The key of scala-library's `unapplySeq` of a sequence class's companion: `Array`'s, or
    /// the `SeqFactory`'s it inherits as seen from the companion, whose `CC[A]` is the class,
    /// each returning the wrapper its signature erases.
    pub(super) fn std_unapply_seq_key(&mut self, c: ClassId) -> (String, String) {
        if c == self.w.b.array {
            return ("scala.Array$".to_string(), "[1],java.lang.Object:java.lang.Object".to_string());
        }
        let cls = self.w.library_class_name(c, "");
        (format!("{}$", cls), format!("[1],{}:scala.collection.SeqOps", cls))
    }

    /// The key of scala-library's `apply` of a case class's companion (`C(args)` in quoted code,
    /// `case_apply`): the constructor's one clause, returning the class.
    pub(super) fn case_apply_key(&mut self, c: ClassId) -> (String, Vec<SigParam>, String) {
        let cls = self.w.library_class_name(c, "");
        let n = self.w.syms.class(c).own_tparams().len();
        let mut sig = Vec::new();
        if n > 0 {
            sig.push(SigParam::Types(n));
        }
        let ctor = self.w.syms.class(c).ctor.clone();
        for p in ctor.iter().flat_map(|cl| cl.params.iter()) {
            let erased = self.result_erasure(p.ty);
            sig.push(SigParam::Type(erased));
        }
        (format!("{}$", cls), sig, cls)
    }

    /// The std's class of a qualified name, `scala.Option`.
    pub(super) fn std_class_named(&self, path: &str) -> Option<ClassId> {
        let (pkg, class) = path.rsplit_once('.')?;
        let mut at = ROOT_PKG;
        for seg in pkg.split('.') {
            let n = self.w.interner.lookup(seg)?;
            at = self.w.syms.pkg(at).entries.get(&n)?.pkg?;
        }
        let n = self.w.interner.lookup(class)?;
        self.w.syms.pkg(at).entries.get(&n)?.class
    }

    /// Whether scala-library resolves the key, without withholding the body where it does not.
    pub(super) fn std_allowed(&mut self, owner: &str, name: &str, shape: &str) -> bool {
        let key = format!("{}\t{}\t{}", owner, name, shape);
        allowlist().contains_key(key.as_str())
    }

    /// The name a member is selected by: with its target name after a `/` where it has one.
    pub(super) fn shape_name(&self, s: SymId) -> String {
        let name = self.name(self.w.syms.sym(s).name);
        match self.target_name(s) {
            Some(t) if t != name => format!("{}/{}", name, t),
            _ => name,
        }
    }

    /// The signature a selection of the member `s` carries: none where scalac selects it by its
    /// name alone (a val, a parameterless def), the kind of type it cannot erase as the error.
    pub(super) fn member_signature(&mut self, s: SymId) -> Result<Option<(Vec<SigParam>, String)>, String> {
        // A library member's signature names no class of the program's bodies: one for every
        // pickle.
        let file = self.w.syms.sym(s).file;
        let library = self.w.files.as_slice().get(file.0 as usize).map_or(false, |f| f.is_std) || self.w.in_jar(file);
        if library {
            if let Some(known) = self.index.library_signatures.borrow().get(&s) {
                return known.clone();
            }
        }
        let sig = self.member_signature_now(s);
        if library {
            self.index.library_signatures.borrow_mut().insert(s, sig.clone());
        }
        sig
    }

    fn member_signature_now(&mut self, s: SymId) -> Result<Option<(Vec<SigParam>, String)>, String> {
        let info = self.sym_info(s);
        let java = self.is_java_method(s);
        let has_sig = matches!(info.kind, SymKind::Def | SymKind::Given) && {
            let sig = self.w.sig_of(s);
            !(sig.clauses.is_empty() && sig.tparams.is_empty())
        } || info.name == crate::names::INIT
            || java;
        if !has_sig || info.owner == Owner::Local {
            return Ok(None);
        }
        let local = self.local_prefix();
        match self.w.pickled_signature(s, &local) {
            Some((params, result)) => Ok(Some((sig_params(params), result))),
            None => Err(self.w.unerasable_in(s, &local)),
        }
    }

    /// The signature of a selection of `s` on a receiver of the class `receiver`: the declared
    /// one where scala-library resolves it, else, where the key names that class and a std member
    /// it inherits (`shape_owner`), the member as the class sees it, its declaring class's type
    /// parameters bound as the class's own parameters bind them (`IndexedSeq`'s `map` returns an
    /// `IndexedSeq`), which scalac's `SELECTin` asks of an inherited member found from the class.
    pub(super) fn member_signature_on(&mut self, s: SymId, receiver: Option<ClassId>) -> Result<Option<(Vec<SigParam>, String)>, String> {
        let sig = self.member_signature(s)?;
        let seen = self.seen_signature(s, receiver, &sig);
        if seen.is_none() || self.shapes_seen.is_some() {
            return Ok(sig);
        }
        let declared = self.with_erased_tags(s, sig.clone());
        match self.std_member_key(s, &declared, receiver) {
            Some((owner, name, shape)) if !self.std_allowed(&owner, &name, &shape) => Ok(seen),
            _ => Ok(sig),
        }
    }

    /// The signature of the inherited std member `s` as the std class `receiver` sees it, where
    /// that is another than the declared one.
    pub(super) fn seen_signature(&mut self, s: SymId, receiver: Option<ClassId>, sig: &Option<(Vec<SigParam>, String)>) -> Option<(Vec<SigParam>, String)> {
        let (Owner::Class(d), Some(_)) = (self.w.syms.sym(s).owner, sig) else { return None };
        let c = receiver.filter(|_| self.is_std_sym(s))?;
        if c == d || self.shape_owner(s, Some(c)) != Some(c) {
            return None;
        }
        self.w.complete_class(c);
        let own: Vec<TypeId> = self.w.syms.class(c).own_tparams().iter().map(|&p| self.w.types.param(p)).collect();
        let this = self.w.types.class(c, &own);
        let base = self.w.base_type(this, d)?;
        let subst = self.w.owner_subst(base);
        if subst.iter().all(|&(p, t)| self.w.types.get(t) == Type::Param(p)) {
            return None;
        }
        let local = self.local_prefix();
        let seen = self.w.pickled_signature_seen(s, &local, &subst).map(|(params, result)| (sig_params(params), result));
        let text = |x: &(Vec<SigParam>, String)| sig_text(&x.0, &x.1);
        seen.filter(|x| sig.as_ref().map(text) != Some(text(x)))
    }

    /// The class a std member is selected on as a key names it: the receiver's, which
    /// scala-library may declare or override it in otherwise than the std, or its owner.
    pub(super) fn shape_owner(&mut self, s: SymId, receiver: Option<ClassId>) -> Option<ClassId> {
        let Owner::Class(c) = self.w.syms.sym(s).owner else { return None };
        Some(receiver.filter(|&r| r != c && self.is_std_class(r)).unwrap_or(c))
    }

    /// The key of a selection of the std member `s` on a receiver of the class `receiver`,
    /// checked.
    pub(super) fn std_member_shape(&mut self, s: SymId, sig: &Option<(Vec<SigParam>, String)>, receiver: Option<ClassId>) -> bool {
        match self.std_member_key(s, sig, receiver) {
            Some((owner, name, shape)) => self.std_shape(&owner, &name, &shape),
            None => true,
        }
    }

    /// The key of a selection of the std member `s`, `None` for a member of no std class.
    pub(super) fn std_member_key(&mut self, s: SymId, sig: &Option<(Vec<SigParam>, String)>, receiver: Option<ClassId>) -> Option<(String, String, String)> {
        if !self.is_std_sym(s) {
            return None;
        }
        let c = self.shape_owner(s, receiver)?;
        let owner = self.w.library_class_name(c, "");
        let name = self.shape_name(s);
        // A given with a body of no parameters is an object, which scala-library may have as
        // an object of its own type.
        let si = self.sym_info(s);
        if si.kind == SymKind::Given && si.impl_class.is_some() && sig.is_none() {
            return Some((owner, name, "object".to_string()));
        }
        let shape = match sig {
            Some((p, r)) => sig_text(p, r),
            None => {
                let ret = self.w.sig_of(s).ret;
                by_name_shape(&self.result_erasure(ret))
            }
        };
        Some((owner, name, shape))
    }

    /// How many `ClassTag`s of the lean std's `@jvmEvidence` member `s` its signature leaves out
    /// here, which scala-library's takes in its last clause.
    pub(super) fn erased_tag_count(&self, s: SymId) -> usize {
        if self.w.erases_evidence() && self.is_std_sym(s) {
            self.w.syms.sym(s).erased_tags.count_ones() as usize
        } else {
            0
        }
    }

    /// The signature scala-library's member of the std member `s` has: the lean one's, with a
    /// last clause of the `ClassTag`s it leaves out.
    pub(super) fn with_erased_tags(&self, s: SymId, sig: Option<(Vec<SigParam>, String)>) -> Option<(Vec<SigParam>, String)> {
        let tags = self.erased_tag_count(s);
        sig.map(|(mut params, result)| {
            params.extend((0..tags).map(|_| SigParam::Type("scala.reflect.ClassTag".to_string())));
            (params, result)
        })
    }

    /// The inverse table's entry for a selection of the std member `s` that scala-library has
    /// under another shape, which the selection is then written in.
    pub(super) fn std_inverse(&mut self, s: SymId, sig: &Option<(Vec<SigParam>, String)>, receiver: Option<ClassId>) -> Option<&'static Inverse> {
        if self.shapes_seen.is_some() {
            return None;
        }
        let (owner, name, shape) = self.std_member_key(s, sig, receiver)?;
        let key = format!("{}\t{}\t{}", owner, name, shape);
        if allowlist().contains_key(key.as_str()) {
            return None;
        }
        inverse_entry(&owner, &name, &shape)
    }

    /// A result type's erasure as a signature names it, `?` where it has none.
    pub(super) fn result_erasure(&mut self, t: TypeId) -> String {
        self.w.library_sig_name(t, "", true).unwrap_or_else(|| "?".to_string())
    }

    /// The key of a std object as a path, checked.
    pub(super) fn std_object_shape(&mut self, o: ClassId) -> bool {
        if !self.is_std_class(o) {
            return true;
        }
        let full = scala_name(self.w, o);
        let full = full.trim_end_matches('$');
        let (owner, name) = full.rsplit_once('.').unwrap_or(("", full));
        let (owner, name) = (owner.to_string(), name.to_string());
        self.std_shape(&owner, &name, "object")
    }

    /// Every key the std can give a body (`TEQ_STD_SHAPES`): its classes' members, constructors,
    /// default getters and objects, and the library members its top-level definitions and
    /// extensions stand for.
    pub(super) fn all_std_shapes(&mut self) -> Vec<String> {
        self.shapes_seen = Some(Vec::new());
        let mut classes = Vec::new();
        let mut pkgs = vec![ROOT_PKG];
        let mut tops = Vec::new();
        while let Some(p) = pkgs.pop() {
            let entries: Vec<PkgEntry> = self.w.syms.pkg(p).entries.values().cloned().collect();
            for e in entries {
                if let Some(k) = e.pkg {
                    pkgs.push(k);
                }
                if let Some(c) = e.class.filter(|&c| self.is_std_class(c)) {
                    classes.push(c);
                }
                for s in e.term.into_iter().chain(e.extensions.iter().copied()).filter(|&s| self.is_std_sym(s)) {
                    match self.w.syms.sym(s).kind {
                        SymKind::Object(c) => classes.push(c),
                        _ => tops.push(s),
                    }
                }
            }
        }
        // The tuple classes a program makes as it needs them (`synthesize_tuple`).
        for n in 1..=22 {
            let c = self.w.tuple_class(n);
            if !classes.contains(&c) {
                classes.push(c);
            }
        }
        let mut at = 0;
        while at < classes.len() {
            let c = classes[at];
            at += 1;
            self.w.complete_class(c);
            let info = self.class_info(c);
            let mut more: Vec<ClassId> = info.nested.values().copied().collect();
            more.extend(info.members.values().filter_map(|&s| match self.w.syms.sym(s).kind {
                SymKind::Object(k) => Some(k),
                _ => None,
            }));
            more.extend(info.companion);
            for k in more {
                if !classes.contains(&k) {
                    classes.push(k);
                }
            }
        }
        for &c in &classes {
            self.class_shapes(c, &mut tops);
            // A case class's `unapply`, which scala-library's case classes declare returning
            // `Option` (`class_pattern`).
            let info = self.w.syms.class(c);
            if info.mods & crate::ast::mods::CASE != 0 && info.singleton.is_none() && info.kind != ClassKind::Object {
                let (key, sig) = self.std_unapply_key(c);
                self.std_shape(&key, "unapply", &sig);
                if self.w.syms.class(c).ctor.len() == 1 {
                    let (key, sig, result) = self.case_apply_key(c);
                    self.std_shape(&key, "apply", &sig_text(&sig, &result));
                }
            }
            // A sequence pattern's `unapplySeq` (`seq_pattern`).
            if self.class_info(c).companion.is_some() && self.w.is_seq_pattern_class(c) {
                let (key, shape) = self.std_unapply_seq_key(c);
                self.std_shape(&key, "unapplySeq", &shape);
            }
        }
        let (key, shape) = self.std_unapply_seq_key(self.w.b.array);
        self.std_shape(&key, "unapplySeq", &shape);
        for s in tops {
            let alts = self.w.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_else(|| vec![s]);
            for a in alts {
                self.std_target_shapes(a);
            }
        }
        self.universal_shapes();
        let mut seen = self.shapes_seen.take().unwrap_or_default();
        seen.sort();
        seen.dedup();
        seen
    }

    fn class_shapes(&mut self, c: ClassId, tops: &mut Vec<SymId>) {
        let info = self.class_info(c);
        if info.kind == ClassKind::Object {
            self.std_object_shape(c);
        }
        let holder = self.name(info.name).ends_with("$package");
        let mut members: Vec<SymId> = Vec::new();
        for &s in info.members.values() {
            members.extend(self.w.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_else(|| vec![s]));
        }
        members.extend(info.extensions.iter().copied());
        for s in members {
            if self.w.syms.sym(s).owner != Owner::Class(c) || !self.is_std_sym(s) {
                continue;
            }
            if holder {
                tops.push(s);
                continue;
            }
            if let Ok(sig) = self.member_signature(s) {
                let sig = self.with_erased_tags(s, sig);
                self.std_member_shape(s, &sig, None);
            }
            let sig = self.w.sig_of(s).clone();
            let mut index = 0;
            for (ci, clause) in sig.clauses.iter().enumerate() {
                for p in &clause.params {
                    if p.has_default {
                        let owner = self.w.library_class_name(c, "");
                        let name = self.name(self.w.syms.sym(s).name);
                        self.getter_shape(&owner, &name, index, sig.tparams.len(), &sig.clauses, ci, p.ty);
                    }
                    index += 1;
                }
            }
        }
        if matches!(info.kind, ClassKind::Class | ClassKind::Builtin) || info.kind == ClassKind::Trait {
            self.ctor_shapes(c);
        }
        // The members it inherits, selected on it.
        let bases: Vec<ClassId> = info.base_types.iter().map(|&(b, _)| b).filter(|&b| b != c && self.is_std_class(b)).collect();
        for b in bases {
            let bi = self.class_info(b);
            if self.name(bi.name).ends_with("$package") {
                continue;
            }
            let mut inherited: Vec<SymId> = Vec::new();
            for (&n, &s) in bi.members.iter() {
                if info.members.get(&n).map_or(false, |&own| self.w.syms.sym(own).owner == Owner::Class(c)) {
                    continue;
                }
                inherited.extend(self.w.syms.alternatives(s).map(|a| a.to_vec()).unwrap_or_else(|| vec![s]));
            }
            for s in inherited {
                let si = self.w.syms.sym(s);
                if si.owner != Owner::Class(b) || !self.is_std_sym(s) || si.mods & mods::PRIVATE != 0 {
                    continue;
                }
                if let Ok(sig) = self.member_signature(s) {
                    if let Some(seen) = self.seen_signature(s, Some(c), &sig) {
                        let seen = self.with_erased_tags(s, Some(seen));
                        self.std_member_shape(s, &seen, Some(c));
                    }
                    let sig = self.with_erased_tags(s, sig);
                    self.std_member_shape(s, &sig, Some(c));
                }
            }
        }
    }

    /// The default getter `name$default$N` of a parameter, on the class `owner` names.
    pub(super) fn getter_shape(&mut self, owner: &str, name: &str, index: usize, tparams: usize, clauses: &[ClauseSig], clause_of: usize, ty: TypeId) -> bool {
        let getter = format!("{}$default${}", name, index + 1);
        let shape = if tparams > 0 || clause_of > 0 {
            let Some((params, result)) = self.w.pickled_default_signature(tparams, clauses, index, "") else {
                self.fail("a default getter's signature".to_string());
                return false;
            };
            sig_text(&sig_params(params), &result)
        } else {
            by_name_shape(&self.result_erasure(ty))
        };
        self.std_shape(owner, &getter, &shape)
    }

    fn ctor_shapes(&mut self, c: ClassId) {
        let owner = self.w.library_class_name(c, "");
        let java = self.is_java_class(c);
        if let Some((params, result)) = self.w.pickled_ctor_signature(c, "") {
            let params = sig_params(params);
            let types = params.iter().filter(|p| matches!(p, SigParam::Types(_))).count();
            let first: Vec<ParamSig> = self.w.syms.class(c).ctor.first().map(|cl| cl.params.clone()).unwrap_or_default();
            // A Java class's constructor the std gives defaults stands for the JDK's overloads.
            let shortest = if java { first.iter().position(|p| p.has_default).unwrap_or(first.len()) } else { first.len() };
            for n in shortest..=first.len() {
                let kept: Vec<SigParam> = if java { params[..types + n].to_vec() } else { params.clone() };
                self.std_shape(&owner, "<init>", &sig_text(&kept, &result));
            }
        }
        let info = self.class_info(c);
        let companion = format!("{}$", owner.trim_end_matches('$'));
        let mut index = 0;
        for (ci, clause) in info.ctor.iter().enumerate() {
            for p in &clause.params {
                if p.has_default && !java {
                    self.getter_shape(&companion, "<init>", index, info.own_tparams().len(), &info.ctor, ci, p.ty);
                }
                index += 1;
            }
        }
        for s in info.ctors.clone() {
            if let Some((params, result)) = self.w.pickled_signature(s, "") {
                self.std_shape(&owner, "<init>", &sig_text(&sig_params(params), &result));
            }
        }
    }
}

impl<'w, 'a> P<'w, 'a> {
    /// The library members a std top-level definition or extension stands for, under each of
    /// the overloads a union-bounded one stands for.
    fn std_target_shapes(&mut self, s: SymId) {
        let Some(target) = self.std_target(s) else { return };
        let info = self.sym_info(s);
        let name = self.name(info.name);
        let sig = self.w.sig_of(s).clone();
        let (skip, owner) = match target {
            // Written as scalac's expansion, or inside an inline body as the call.
            StdTarget::Summon => {
                self.std_shape("scala.Predef$", "summon", "[1],java.lang.Object:java.lang.Object");
                return;
            }
            StdTarget::Conforms => {
                self.predef_shape("$conforms", &[SigParam::Types(1)], "scala.Function1");
                return;
            }
            StdTarget::Object { pkg, object } => {
                self.std_shape(pkg, object, "object");
                let owner = format!("{}.{}$", pkg, object);
                if name == "println" {
                    self.std_shape(&owner, "println", ":scala.Unit");
                }
                if sig.clauses.is_empty() && sig.tparams.is_empty() {
                    let r = self.result_erasure(sig.ret);
                    self.std_shape(&owner, &name, &by_name_shape(&r));
                    return;
                }
                ((0, 0), owner)
            }
            StdTarget::Receiver { pkg, class, .. } => (((info.ext_tparams as usize), (info.ext_clauses as usize)), library_owner(pkg, class)),
            // The array's own class, `ArrayOps` and the `ArraySeq`s, which the member may be one of.
            StdTarget::Array => {
                self.std_shape("scala", "Predef", "object");
                let mut owners = vec!["scala.Array".to_string()];
                for elem in ARRAY_ELEMS {
                    for conv in [elem.ops(), elem.wrap()] {
                        self.predef_shape(&conv.name, &conversion_sig(conv.targs, &conv.param), &conv.result);
                        let owner = library_owner(conv.class.0, &conv.class.1);
                        if !owners.contains(&owner) {
                            owners.push(owner);
                        }
                    }
                }
                let skip = ((info.ext_tparams as usize), (info.ext_clauses as usize));
                let clauses = self.std_own_clauses(s, target);
                let Some((params, keeps, result)) = self.std_signature_at(s, skip, &[], &[]) else { return };
                let by_name = params.is_empty() && clauses.is_empty() && !(keeps && sig.tparams.len() > skip.0);
                let shape = if by_name { by_name_shape(&result) } else { sig_text(&params, &result) };
                // A `@jvmEvidence` member's `ClassTag`s, which its signature here leaves out.
                let tags = self.w.syms.sym(s).erased_tags.count_ones() as usize;
                let mut with_tags = params.clone();
                with_tags.extend((0..tags).map(|_| SigParam::Type("scala.reflect.ClassTag".to_string())));
                for owner in owners {
                    self.std_shape(&owner, &name, &shape);
                    if tags > 0 {
                        self.std_shape(&owner, &name, &sig_text(&with_tags, &result));
                    }
                }
                return;
            }
            StdTarget::Wrapped { conv, targs, param, result, class: (pkg, class) } => {
                self.std_shape("scala", "Predef", "object");
                self.predef_shape(conv, &conversion_sig(targs, param), result);
                (((info.ext_tparams as usize), (info.ext_clauses as usize)), library_owner(pkg, class))
            }
        };
        let own_tparams: Vec<TParamId> = sig.tparams[skip.0.min(sig.tparams.len())..].to_vec();
        let unions: Vec<Vec<TypeId>> = own_tparams
            .iter()
            .map(|&tp| {
                let u = self.w.syms.tparam(tp).upper;
                match self.w.types.get(u) {
                    Type::Union(..) => self.union_members(u),
                    _ => Vec::new(),
                }
            })
            .collect();
        // A parameter of a union type (`split(separator: String | Char)`) is selected at the type
        // of its argument, one key per member.
        let params: Vec<TypeId> = sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).collect();
        let value_unions: Vec<Vec<TypeId>> = params.iter().map(|&t| if matches!(self.w.types.get(t), Type::Union(..)) { self.union_members(t) } else { Vec::new() }).collect();
        let variants = unions.iter().chain(&value_unions).map(|u| u.len()).max().unwrap_or(0).max(1);
        let clauses: Vec<usize> = match target {
            StdTarget::Object { .. } => sig.clauses.iter().map(|c| c.params.len()).collect(),
            _ => self.std_own_clauses(s, target),
        };
        for k in 0..variants {
            let inst: Vec<TypeId> = unions.iter().map(|u| u.get(k).or(u.last()).copied().unwrap_or(ANY)).collect();
            let arg_types: Vec<Option<TypeId>> = value_unions.iter().map(|u| u.get(k).or(u.last()).copied()).collect();
            let Some((params, keeps, result)) = self.std_signature_at(s, skip, &inst, &arg_types) else { continue };
            let by_name = params.is_empty() && clauses.is_empty() && !(keeps && !own_tparams.is_empty());
            let shape = if by_name { by_name_shape(&result) } else { sig_text(&params, &result) };
            self.std_shape(&owner, &name, &shape);
        }
    }

    fn union_members(&self, t: TypeId) -> Vec<TypeId> {
        match self.w.types.get(t) {
            Type::Union(a, b) => {
                let mut out = self.union_members(a);
                out.extend(self.union_members(b));
                out
            }
            _ => vec![t],
        }
    }

    fn universal_shapes(&mut self) {
        for name in UNIVERSAL {
            if let Some((pkg, owner, tps, params, result, clause)) = universal_member(name) {
                let shape = if !clause && tps == 0 { by_name_shape(result) } else { sig_text(&universal_sig(tps, params), result) };
                self.std_shape(&format!("{}.{}", pkg, owner), name, &shape);
            }
        }
    }
}
