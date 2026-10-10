//! Overloaded methods: which alternatives a name stands for in a class that inherits some of
//! them, and which one a reference means. Resolution follows SLS 6.26.3 as dotc's
//! `resolveOverloaded` reads it: alternatives that fit the shape of the call, then those the typed
//! arguments are compatible with, then the most specific one. An argument is typed once; a
//! function literal without parameter types is judged by its shape and typed against the
//! alternative that was chosen.

use super::apply::{ArgList, ArgSrc, Callee, MethodCall};
use super::check::Agreement;
use super::profile::{About, Kind, Outcome};
use super::Worker;
use crate::ast::{Expr, ListRef, Stmt, TyExpr};
use crate::intern::{FxMap, Name};
use crate::names;
use crate::source::{FileId, Span};
use crate::symbols::*;
use crate::tir::*;
use crate::types::*;
use std::sync::Arc;

/// One alternative as the receiver sees it.
pub(super) struct Alt {
    pub sym: SymId,
    pub sig: Arc<MethodSig>,
    pub owner_subst: Subst,
    /// Set for the constructor of the case class whose companion the receiver is, which
    /// `C(args)` and `C.apply` mean next to the `apply` methods of the companion, as the `apply`
    /// scalac synthesizes; `sym` is an `apply` of the companion then.
    pub ctor: Option<ClassId>,
}

fn overload_outcome(choice: &Result<usize, Failure>) -> Outcome {
    match choice {
        Ok(_) => Outcome::Found,
        Err(Failure::Ambiguous(_)) => Outcome::Ambiguous,
        Err(_) => Outcome::NotFound,
    }
}

impl Alt {
    /// The parameter clauses that take argument lists, in order.
    fn explicit_clauses(&self) -> impl Iterator<Item = &ClauseSig> {
        self.sig.clauses.iter().filter(|c| !c.is_using)
    }

    fn clause(&self, k: usize) -> Option<&ClauseSig> {
        self.explicit_clauses().nth(k)
    }

    fn is_method(&self) -> bool {
        self.explicit_clauses().next().is_some()
    }

    fn has_defaults(&self) -> bool {
        self.sig.clauses.iter().any(|c| c.params.iter().any(|p| p.has_default))
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Plain,
    /// A function literal that leaves a parameter type open, with its arity; a `{ case ... }`
    /// literal fits a function of any arity.
    Lambda(usize, bool),
    Splice,
}

#[derive(Clone, Copy)]
struct ArgShape {
    name: Option<Name>,
    shape: Shape,
}

/// What the arguments of one list are, once those that can be typed on their own are typed.
struct ListInfo {
    shapes: Vec<ArgShape>,
    types: Vec<Option<TypeId>>,
    /// The function literals that were typed with the parameter types the candidates agree on,
    /// each with its type and the type of its result.
    lambdas: Vec<Option<(TExprId, TypeId, TypeId)>>,
    /// The names an argument's value comes out of, each with its node and the candidates that
    /// take a function of it, recorded once the resolution says which candidates remain
    /// (`resolve_overloaded`): index sessions alone.
    function_names: Vec<(crate::ast::ExprId, TExprId, Vec<usize>)>,
}

/// The parameter each argument of a list goes to.
enum Mapping {
    /// Argument `j` goes to parameter `j`: positional arguments, one per parameter.
    InOrder,
    To(Vec<usize>),
}

impl Mapping {
    #[inline]
    fn param(&self, arg: usize) -> usize {
        match self {
            Mapping::InOrder => arg,
            Mapping::To(params) => params[arg],
        }
    }
}

enum Failure {
    NoneApplicable,
    Ambiguous(Vec<usize>),
}

/// A reference to an overloaded name that no alternative answers.
pub(super) struct Unresolved {
    pub msg: String,
    pub lists: Vec<ArgList>,
    pub none_applicable: bool,
    /// An alternative whose header the parser could not complete was left out, which may have
    /// been the one meant: the failure is reported as dependent.
    pub dependent: bool,
}

impl<'a> Worker<'a> {
    // ---- the alternatives of a name in a class ----

    /// The `k`-th alternative that `b` itself defines for `name`.
    pub(super) fn own_alternative(&self, b: ClassId, name: Name, k: usize) -> Option<SymId> {
        let &entry = self.syms.class(b).members.get(&name)?;
        match self.syms.alternatives(entry) {
            Some(alts) => alts.iter().copied().filter(|&a| self.syms.sym(a).owner == Owner::Class(b)).nth(k),
            None => (k == 0).then_some(entry),
        }
    }

    /// Settles the entry `entry` of `c`, whose name an ancestor has methods for: the inherited
    /// ones that no alternative of `c` overrides join it. `None` when the entry stood for
    /// nothing but one inherited method, which the lookup then finds where it is defined.
    #[cold]
    #[inline(never)]
    pub(super) fn merge_inherited(&mut self, c: ClassId, entry: SymId) -> Option<SymId> {
        // The entry, its alternatives and what it meets in the ancestors are records of the
        // shared region unless the whole hierarchy is the body's own.
        if !self.forked {
            return self.merge_inherited_unlocked(c, entry);
        }
        // A program class's entry is compared outside the loader's lock, where a signature
        // another worker holds is waited for, and the merged set is made under it;
        // two workers settling the entry at once compare alike, and
        // the second finds it settled.
        if self.program_file(self.syms.class(c).file) && crate::shared::lock_depth() == 0 {
            self.merging.push(entry);
            let all = self.merged_alternatives(c, entry);
            self.merging.pop();
            return self.with_loader(|w| {
                if !w.syms.sym(entry).merge_pending {
                    let name = w.syms.sym(entry).name;
                    return w.syms.class(c).members.get(&name).copied();
                }
                let settled = w.settle_merged(c, entry, all);
                w.syms.sym_mut(entry).merge_pending = false;
                settled
            });
        }
        self.with_loader(|w| {
            // Another worker settled the entry between the caller's look at it and the
            // lock: what the class holds for the name now is the settled entry.
            if !w.syms.sym(entry).merge_pending {
                let name = w.syms.sym(entry).name;
                return w.syms.class(c).members.get(&name).copied();
            }
            w.merge_inherited_unlocked(c, entry)
        })
    }

    /// The entry stays pending until the merge is whole: an entry read as settled before the
    /// class holds the merged set would be read without the inherited alternatives.
    pub(super) fn merge_inherited_unlocked(&mut self, c: ClassId, entry: SymId) -> Option<SymId> {
        self.merging.push(entry);
        let all = self.merged_alternatives(c, entry);
        self.merging.pop();
        let settled = self.settle_merged(c, entry, all);
        self.syms.sym_mut(entry).merge_pending = false;
        settled
    }

    /// The alternatives of `entry` in `c`, its own first, then the inherited methods that none
    /// of them overrides.
    pub(super) fn merged_alternatives(&mut self, c: ClassId, entry: SymId) -> (Vec<SymId>, Vec<SymId>) {
        let name = self.syms.sym(entry).name;
        let own: Vec<SymId> = match self.syms.alternatives(entry) {
            Some(alts) => alts.to_vec(),
            None => vec![entry],
        };
        let mut all = own.clone();
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        // A joined entry (`own` empty) takes a value of the name as an alternative next to the
        // methods, as a class of one's own does through `join_value_and_method`.
        let values_too = own.is_empty();
        for &(b, bt) in &bases[1..] {
            let mut k = 0;
            while let Some(p) = self.own_alternative(b, name, k) {
                k += 1;
                let kind = self.syms.sym(p).kind;
                if !(kind == SymKind::Def || (values_too && matches!(kind, SymKind::Val | SymKind::Var))) || self.is_private(p) {
                    continue;
                }
                let overridden = all.iter().any(|&m| self.same_parameters(c, &bases, m, p, bt));
                if !overridden {
                    all.push(p);
                }
            }
        }
        (own, all)
    }

    /// Settles `entry` of `c` with the alternatives `merged_alternatives` found.
    pub(super) fn settle_merged(&mut self, c: ClassId, entry: SymId, (own, all): (Vec<SymId>, Vec<SymId>)) -> Option<SymId> {
        let name = self.syms.sym(entry).name;
        if all.len() == own.len() && !own.is_empty() {
            return Some(entry);
        }
        if all.len() <= 1 {
            self.syms.class_mut(c).members.remove(&name);
            return None;
        }
        let set = self.syms.new_overloaded(all[0], Owner::Class(c), all);
        self.syms.class_mut(c).members.insert(name, set);
        self.syms.class_mut(c).has_overloads = true;
        // The entry it replaces stands for nothing any more.
        if let SymKind::Overloaded(_) = self.syms.sym(entry).kind {
            self.syms.sym_mut(entry).superseded = true;
        }
        // A set merged once the alternatives are named (a std class entered on demand): the
        // roots of its alternatives are told apart first, as the final pass tells them apart
        // before it names the sets.
        if self.alternatives_named {
            self.suffix_clashing_roots(&[]);
            self.name_alternatives(set);
        }
        Some(set)
    }

    /// The members whose names a root suffixed after the final passes invalidated, named again
    /// before the reach pass reads them: a call marked by a stale plain name would keep the
    /// definitions above the one it runs, which a later build with the names settled leaves out.
    pub fn settle_dispatch_pending(&mut self) {
        if !self.alternatives_named {
            return;
        }
        for m in std::mem::take(&mut self.dispatch_pending) {
            self.dispatch_name(m);
        }
    }

    /// Whether `m` and `p`, members of `c` or of its ancestors, take the same parameters as `c`
    /// sees them, which makes the one that comes first override the other.
    pub(super) fn same_parameters(&mut self, c: ClassId, bases: &[(ClassId, TypeId)], m: SymId, p: SymId, p_owner: TypeId) -> bool {
        let m_owner = match self.syms.sym(m).owner {
            Owner::Class(mc) if mc != c => bases.iter().find(|&&(b, _)| b == mc).map(|&(_, t)| t),
            _ => None,
        };
        let msig = self.parameters_arc(m);
        let psig = self.parameters_arc(p);
        let psig = self.seen_from_enclosing_outer(c, p, psig);
        let lenient_using = self.is_std_scala_member(m) || self.is_std_scala_member(p);
        self.compare_sig_pair(c, msig, m_owner, psig, p_owner, true, lenient_using) != Agreement::Params
    }

    /// The signature of `p`, a member of a trait nested in a class, as the class `c` nested in
    /// classes sees it: the `this` of each class enclosing `p`'s is that of the nearest class
    /// enclosing `c` that derives from it or has it in its self type (`ExprPromises.this.Type[A]`
    /// of `ExprPromiseModule` in `ExprPromisesPlatform`'s object), since a parent's prefix is
    /// rooted there.
    fn seen_from_enclosing_outer(&mut self, c: ClassId, p: SymId, sig: Arc<MethodSig>) -> Arc<MethodSig> {
        let Owner::Class(b) = self.syms.sym(p).owner else { return sig };
        let enclosing = |t: &Self, k: ClassId| {
            let mut out = Vec::new();
            let mut at = t.syms.class(k).owner;
            // A local or anonymous class stands in the class whose body made it (chimney's
            // `new TotallyBuildIterable[M, A] { .. }` in a method of the cake).
            if at == Owner::Local {
                if let Some(env) = t.anon_envs.get(&k) {
                    if let Some(h) = env.frames.iter().rev().find_map(|f| match f { super::Frame::Class(x) => Some(*x), _ => None }) {
                        at = Owner::Class(h);
                    }
                }
            }
            while let Owner::Class(o) = at {
                if t.syms.class(o).kind == ClassKind::Object || out.len() == 8 {
                    break;
                }
                out.push(o);
                at = t.syms.class(o).owner;
            }
            out
        };
        let outers = enclosing(self, c);
        if outers.is_empty() {
            return sig;
        }
        let mut sig = sig;
        for k in enclosing(self, b) {
            if outers.contains(&k) {
                break;
            }
            let Some(&e) = outers.iter().find(|&&e| self.syms.class(e).base_types.iter().any(|&(x, _)| x == k) || self.self_type_derives(e, k)) else { continue };
            let prefix = self.this_prefix(e);
            sig = self.sig_seen_from_outer(sig, prefix, k);
        }
        sig
    }

    /// The signature of `s` for its parameters alone: while the result of `s` is being inferred
    /// they are known already, and that partial signature is read as a complete one is, in the
    /// reader's view with the overlays on (`partial_sig_hooked`: imported by a worker, exported
    /// into the base under the loader's lock).
    pub(super) fn parameters_of(&mut self, s: SymId) -> &MethodSig {
        if self.parameters_known(s) {
            if self.profile.sig_hook {
                return self.partial_sig_hooked(s);
            }
            return self.syms.sym(s).info.sig.as_deref().unwrap();
        }
        self.sig_of(s)
    }

    pub(super) fn parameters_arc(&mut self, s: SymId) -> Arc<MethodSig> {
        if self.parameters_known(s) {
            if self.profile.sig_hook {
                return self.partial_sig_hooked(s).clone();
            }
            return self.syms.sym(s).sig.clone().unwrap();
        }
        self.sig_arc(s)
    }

    fn parameters_known(&self, s: SymId) -> bool {
        let info = self.syms.sym(s);
        info.state() == Completion::InProgress && info.sig.is_some()
    }

    /// A library class that inherits one method name from two unrelated ancestors gets the
    /// entry joining their alternatives that a source class gets from
    /// `check_conflicting_members`: the name is overloaded in it, and the reach dispatches
    /// a call of either to the right one once they are named apart.
    pub(super) fn join_unrelated_inherited(&mut self, c: ClassId) {
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().skip(1).map(|&(b, _)| b).collect();
        let extends = |t: &Self, sub: ClassId, sup: ClassId| t.syms.class(sub).base_types.iter().any(|&(x, _)| x == sup);
        let mut first_method: crate::intern::FxMap<Name, (ClassId, SymId)> = Default::default();
        let mut joined: Vec<SymId> = Vec::new();
        for &b in &bases {
            for m in self.syms.class(b).member_order.clone() {
                let info = self.syms.sym(m);
                if !matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var) || info.mods & crate::ast::mods::PRIVATE != 0 || super::setters::is_concrete_setter(&self.syms, m) {
                    continue;
                }
                let name = info.name;
                match first_method.get(&name) {
                    None => {
                        first_method.insert(name, (b, m));
                    }
                    Some(&(first, like)) => {
                        let apart = first != b && !extends(self, first, b);
                        let is_def = |t: &Self, s: SymId| t.syms.sym(s).kind == SymKind::Def;
                        if apart && !self.syms.class(c).members.contains_key(&name) && !joined.contains(&like) && (is_def(self, m) || is_def(self, like)) {
                            joined.push(like);
                        }
                        // A method next to a value of its name from another trait (tapir's
                        // `Endpoint`: `def method(m)` with `lazy val method`) cannot keep the
                        // plain name in the output, which the value's property takes.
                        if apart && is_def(self, m) != is_def(self, like) {
                            let method = if is_def(self, m) { m } else { like };
                            self.suffixed_roots.insert(method, ());
                        }
                    }
                }
            }
        }
        for like in joined {
            self.add_joined_entry(c, like);
        }
    }

    /// A placeholder entry in `c` for a name that two unrelated ancestors define: the first
    /// lookup collects their alternatives.
    pub(super) fn add_joined_entry(&mut self, c: ClassId, like: SymId) {
        let name = self.syms.sym(like).name;
        let set = self.syms.new_overloaded(like, Owner::Class(c), Vec::new());
        let mut info = self.syms.sym_mut(set);
        info.meets_inherited = true;
        info.merge_pending = true;
        self.syms.class_mut(c).members.insert(name, set);
    }

    // ---- names in the output ----

    /// The name of the method `m` in the output when it is not its Scala name: an alternative
    /// of an overloaded name carries its erased parameter types. The name is that of the
    /// declaration `m` overrides, if any, so that a call through an ancestor reaches it; a
    /// method that was the only one of its name where it was first declared keeps the plain
    /// name all the way down. Members of JS types keep their names: there the alternatives
    /// are the ways to call one JavaScript method.
    /// The name `m` goes by in the output.
    pub fn output_name_of(&mut self, m: SymId) -> Name {
        self.dispatch_name(m).unwrap_or(self.syms.sym(m).name)
    }

    pub(super) fn dispatch_name(&mut self, m: SymId) -> Option<Name> {
        match self.syms.sym(m).dispatch {
            Dispatch::Named => return Some(*self.syms.dispatch_names.get(&m).expect("a named method has its name")),
            Dispatch::Plain | Dispatch::InProgress => return None,
            Dispatch::Unknown => {}
        }
        let (name, owner, overloadable) = {
            let info = self.syms.sym(m);
            (info.name, info.owner, info.kind == SymKind::Def && !info.is_extension)
        };
        if !overloadable || self.nameless_inline(m) {
            self.syms.sym_mut(m).dispatch = Dispatch::Plain;
            return None;
        }
        self.syms.sym_mut(m).dispatch = Dispatch::InProgress;
        let named = match owner {
            Owner::Local => None,
            _ if self.target_name(m).is_some() && self.overrides_nothing(m) => self.target_name(m),
            Owner::Package(p) => {
                let entry = self.syms.pkg(p).entries.get(&name).and_then(|e| e.term);
                let overloaded = entry.map_or(false, |e| self.syms.alternatives(e).is_some());
                overloaded.then(|| self.suffixed_name(m, entry.unwrap()))
            }
            Owner::Class(c) if self.syms.class(c).js != JsKind::Scala => None,
            // The runtime calls `toString`, `hashCode` and `equals` by their plain names.
            Owner::Class(_) if matches!(name, names::TO_STRING | names::HASH_CODE | names::EQUALS) && self.matches_any_member(m) => None,
            Owner::Class(c) => {
                self.complete_class(c);
                match self.overridden_dispatch(c, m) {
                    Some(inherited) => inherited,
                    None => (!self.keeps_plain_name(m)).then(|| {
                        let entry = self.settled_entry(c, name).unwrap_or(m);
                        self.suffixed_name(m, entry)
                    }),
                }
            }
        };
        self.syms.sym_mut(m).dispatch = match named {
            Some(n) => {
                self.syms.dispatch_names.insert(m, n);
                Dispatch::Named
            }
            None => Dispatch::Plain,
        };
        named
    }

    /// `@targetName("name")` on an alternative of an overloaded name: the name erasure and the
    /// output know it by, which is how two alternatives that erase alike are told apart.
    fn target_name(&mut self, m: SymId) -> Option<Name> {
        if !self.syms.sym(m).alternative {
            return None;
        }
        self.target_name_of(m)
    }

    /// The `@targetName` `m` declares, from a jar's signature or a source's annotation, whether or
    /// not it is an alternative here: what tells two methods of one name and one erasure apart
    /// where neither overrides the other, as the interpreter's walk for an override needs
    /// (`Interp::alternative_member`), and the name the JVM's class files give an alternative
    /// (`source_target_names`).
    ///
    /// A source's annotation is `scala.annotation.targetName` by its class, as its type resolves
    /// where `m` is defined (the route a written type takes, the class path's included), under
    /// whatever name the scope gives it (`import scala.annotation.{targetName as tn}`, `import
    /// scala.annotation.*`, `@scala.annotation.targetName`), and not a class of the program that
    /// takes the name.
    pub(crate) fn target_name_of(&mut self, m: SymId) -> Option<Name> {
        self.target_name_among(m, None)
    }

    /// `target_name_of`, resolving only the annotations written under one of `only`'s names.
    fn target_name_among(&mut self, m: SymId, only: Option<&[Name]>) -> Option<Name> {
        if self.loaded.as_ref().map_or(false, |l| l.syms.contains_key(&m)) {
            self.sig_of(m);
            return self.loaded.as_ref().unwrap().target_names.get(&m).copied();
        }
        let info = self.syms.sym(m);
        let (file, owner, at) = (info.file, info.owner, info.span.start);
        let ast = self.ast(file);
        // The annotation takes one string, the name: the others are not resolved.
        let written: Vec<(crate::ast::TyExprId, crate::ast::StrId)> = ast
            .def(info.def?)
            .annots
            .iter()
            .filter(|a| only.map_or(true, |names| names.contains(&a.name)))
            .filter_map(|a| Some((ast.annot_new(a)?.0, *ast.annot_args(a).first()?)))
            .collect();
        if written.is_empty() {
            return None;
        }
        let env = self.env_at(file, owner, at);
        let text = self.with_env(env, |t| {
            let mark = t.diags.items.len();
            let found = written.iter().find(|&&(ty, _)| {
                let resolved = t.resolve_type_ctor(ty);
                matches!(t.types.get(resolved), Type::Class(k, _) | Type::Ctor(k) if t.is_target_name_class(k))
            });
            t.drop_reported_since(mark);
            found.map(|&(_, arg)| ast.str(arg).to_string())
        })?;
        Some(self.interner.intern(&text))
    }

    /// Whether `k` is `scala.annotation.targetName`, the lean std's or the class path's, by its
    /// qualified name: what an annotation's type resolved to is compared, so the class is never
    /// looked up apart from the annotation's own resolution.
    fn is_target_name_class(&self, k: ClassId) -> bool {
        let info = self.syms.class(k);
        let Owner::Package(p) = info.owner else { return false };
        let named = |p: PkgId, n: &str| self.interner.get(self.syms.pkg(p).name) == n;
        self.interner.get(info.name) == "targetName"
            && named(p, "annotation")
            && self.syms.pkg(p).parent.map_or(false, |q| named(q, "scala") && self.syms.pkg(q).parent.map_or(true, |r| r == ROOT_PKG))
    }

    /// The `@targetName`s of the program's and the std's defs, resolved as `target_name_of`
    /// resolves them: the names the JVM's class files give those methods, every one of them as
    /// under scalac, overloaded or not (`jvm::Input::source_target_names`).
    ///
    /// An alternative's annotations are resolved as `target_name_of` resolves them; another def's
    /// only where written `targetName` or under a name its file's imports give `targetName`
    /// (`import scala.annotation.{targetName as tn}`): the std's thousands of `@js` and `@jvm`
    /// stay unresolved. A resolution may enter the annotation's class (a written-out
    /// `@scala.annotation.targetName`'s, from the class path), so a JVM build makes the map before
    /// its reach is walked or a kept one compared, whose tables then cover the class (`main.rs`,
    /// `watch.rs`'s `jvm_target_names`).
    pub(crate) fn source_target_names(&mut self) -> FxMap<SymId, Name> {
        let mut out = FxMap::default();
        let target_name = self.interner.intern("targetName");
        let mut written_names: FxMap<FileId, Vec<Name>> = FxMap::default();
        for i in 0..self.syms.syms.len() {
            let s = SymId(i as u32);
            let info = self.syms.sym(s);
            if info.kind != SymKind::Def {
                continue;
            }
            let (file, alternative) = (info.file, info.alternative || info.meets_inherited);
            let Some(d) = info.def else { continue };
            let ast = self.ast(file);
            if ast.def(d).annots.is_empty() || self.loaded.as_ref().map_or(false, |l| l.syms.contains_key(&s)) {
                continue;
            }
            let found = if alternative {
                self.target_name_of(s)
            } else {
                let names = written_names.entry(file).or_insert_with(|| {
                    let mut names = vec![target_name];
                    for import in ast.imports.iter().chain(&ast.local_imports) {
                        if let crate::ast::ImportSel::Name(n, Some(alias)) = import.sel {
                            if n == target_name && !names.contains(&alias) {
                                names.push(alias);
                            }
                        }
                    }
                    names
                });
                if !ast.def(d).annots.iter().any(|a| names.contains(&a.name)) {
                    continue;
                }
                let names = names.clone();
                self.target_name_among(s, Some(&names))
            };
            if let Some(n) = found {
                out.insert(s, n);
            }
        }
        // An inline accessor of an expanded name, `INLINEACCESSOR(EXPANDED(p$C, x))`
        // (`typer::accessors`), is `p$C$$inline$x` in the class file, the qualifier mangled
        // first, as scalac's backend mangles `PrepareInlineable.accessorNameOf`'s name.
        let accessors: Vec<((ClassId, SymId, bool), SymId)> = self.inline_accessor_syms.iter().map(|(&k, &s)| (k, s)).collect();
        for ((c, target, setter), s) in accessors {
            let (prefix, member) = self.inline_accessor_parts(c, target, setter);
            if !prefix.is_empty() {
                let binary = self.interner.intern(&format!("{}$$inline${}", prefix.join("$"), crate::jvm::names::encode(&member)));
                out.insert(s, binary);
            }
        }
        self.jvm_volatile = self.source_volatile();
        out
    }

    /// The vals, vars and class parameters of the program's and the std's sources that
    /// `@scala.volatile` marks (`Worker::jvm_volatile`, `ACC_VOLATILE` on their fields), resolved
    /// as `target_name_of` resolves `@targetName`: by the class its type resolves to, dealiased,
    /// where the definition stands, whatever a program writes (an alias, a renaming import), not a
    /// class of the program that takes the name; the std's annotations written `volatile` alone.
    /// With `source_target_names`, before the reach, as a resolution may enter the annotation's
    /// class.
    fn source_volatile(&mut self) -> FxMap<SymId, ()> {
        let mut out = FxMap::default();
        let Some(volatile) = self.interner.lookup("volatile") else { return out };
        let mut written_names: FxMap<FileId, Vec<Name>> = FxMap::default();
        for i in 0..self.syms.syms.len() {
            let s = SymId(i as u32);
            let info = self.syms.sym(s);
            if !matches!(info.kind, SymKind::Val | SymKind::Var) || self.loaded.as_ref().map_or(false, |l| l.syms.contains_key(&s)) {
                continue;
            }
            let (file, owner, at, name) = (info.file, info.owner, info.span.start, info.name);
            let ast = self.ast(file);
            // A definition's own annotations, or a class parameter's, its class's clause's.
            let annots: Vec<crate::ast::Annot> = match (info.def, owner) {
                (Some(d), _) => ast.def(d).annots.clone(),
                (None, Owner::Class(c)) => {
                    let Some(cd) = self.syms.class(c).def.filter(|_| self.syms.class(c).file == file) else { continue };
                    let crate::ast::DefKind::Class(cls) = &ast.def(cd).kind else { continue };
                    let Some(p) = cls.clauses.iter().flat_map(|cl| cl.params.iter()).find(|p| p.name == name) else { continue };
                    ast.param_annots(p).to_vec()
                }
                _ => continue,
            };
            if annots.is_empty() {
                continue;
            }
            let names = written_names.entry(file).or_insert_with(|| {
                let mut names = vec![volatile];
                for import in ast.imports.iter().chain(&ast.local_imports) {
                    if let crate::ast::ImportSel::Name(n, Some(alias)) = import.sel {
                        if n == volatile && !names.contains(&alias) {
                            names.push(alias);
                        }
                    }
                }
                names
            });
            // A program's annotation is resolved whatever its spelling (`type V = scala.volatile`);
            // the std's, written `volatile` alone.
            let program = self.is_program_file(file);
            let written: Vec<crate::ast::TyExprId> = annots.iter().filter(|a| program || names.contains(&a.name)).filter_map(|a| Some(ast.annot_new(a)?.0)).collect();
            if written.is_empty() {
                continue;
            }
            let env = self.env_at(file, owner, at);
            let found = self.with_env(env, |t| {
                let mark = t.diags.items.len();
                let found = written.iter().any(|&ty| {
                    let resolved = t.resolve_type_ctor(ty);
                    let resolved = t.dealias(resolved);
                    matches!(t.types.get(resolved), Type::Class(k, _) | Type::Ctor(k) if t.is_scala_class(k, "volatile"))
                });
                t.drop_reported_since(mark);
                found
            });
            if found {
                out.insert(s, ());
            }
        }
        out
    }

    fn overrides_nothing(&mut self, m: SymId) -> bool {
        let mut roots = Vec::new();
        self.root_declarations(m, 0, &mut roots);
        roots == [m]
    }

    /// `m` implements `root` and nothing else, so it goes by its name in the output: the method
    /// of the class that a lambda for a trait with one abstract method becomes.
    pub(super) fn name_like(&mut self, m: SymId, root: SymId) {
        let info = self.syms.sym(root);
        if !(info.alternative || info.meets_inherited || info.dispatch == Dispatch::Named) {
            return;
        }
        self.name_like_always(m, root)
    }

    pub(super) fn name_beside_method(&mut self, m: SymId) {
        let n = self.suffixed_name(m, m);
        self.publish_dispatch_name(m, n);
    }

    pub(super) fn name_like_always(&mut self, m: SymId, root: SymId) {
        if !self.alternatives_named {
            self.named_like.push((m, root));
            return;
        }
        if let Some(n) = self.dispatch_name(root) {
            self.publish_dispatch_name(m, n);
        }
    }

    /// Gives `m` the name `n` in the output: the map's entry and the symbol's flag change
    /// together, under the loader's lock where `m` is a record of the shared region
    /// (`shared_write`); the name is computed before, outside it.
    fn publish_dispatch_name(&mut self, m: SymId, n: Name) {
        self.shared_write(m, |w| {
            w.syms.dispatch_names.insert(m, n);
            w.syms.sym_mut(m).dispatch = Dispatch::Named;
        });
    }

    /// The callee for the alternative that was chosen when it is no method: a val among the
    /// alternatives of a name that an ancestor overloads.
    fn value_alternative(&mut self, recv: Option<TExprId>, recv_ty: Option<TypeId>, sym: SymId) -> Option<Callee> {
        if self.syms.sym(sym).kind == SymKind::Def {
            return None;
        }
        let (recv, recv_ty) = (recv?, recv_ty?);
        let owner_ty = match self.syms.sym(sym).owner {
            Owner::Class(oc) => self.base_type(recv_ty, oc),
            _ => None,
        };
        Some(self.member_callee_in(recv, recv_ty, owner_ty, sym))
    }

    /// Whether the method `m` of a class, which overrides nothing, is the only one of its name
    /// there and in every class that inherits it next to a method it does not override.
    fn is_function_apply(&self, m: SymId) -> bool {
        let info = self.syms.sym(m);
        matches!(info.owner, Owner::Class(c) if info.name == names::APPLY && self.is_function_class(c))
    }

    fn keeps_plain_name(&mut self, m: SymId) -> bool {
        if self.suffixed_roots.contains_key(&m) {
            return false;
        }
        let (name, Owner::Class(c)) = (self.syms.sym(m).name, self.syms.sym(m).owner) else { return true };
        let Some(entry) = self.settled_entry(c, name) else { return true };
        let alts: Vec<SymId> = self.syms.alternatives(entry).map_or_else(Vec::new, |a| a.to_vec());
        // A concrete var's setter has no output, its call being the var's assignment.
        let mut written = 0;
        for a in alts {
            if a == m || (!self.nameless_inline(a) && !super::setters::is_concrete_setter(&self.syms, a)) {
                written += 1;
            }
        }
        written <= 1
    }

    /// An inline method that overrides nothing is expanded where it is called and is no
    /// member of the output: it has no name there and takes none from the alternatives beside
    /// it. One that overrides a method keeps a body for the calls through what it overrides.
    pub(super) fn nameless_inline(&mut self, m: SymId) -> bool {
        self.syms.sym(m).mods & crate::ast::mods::INLINE != 0 && self.overrides_nothing(m)
    }

    /// `root_declarations` of `m` alone, which the class hierarchy settles once `m`'s class is
    /// complete, and which every set merged after the final passes asks again.
    fn roots_of(&mut self, m: SymId) -> Arc<[SymId]> {
        if let Some(r) = self.roots_memo.get(&m) {
            return r.clone();
        }
        let mut out = Vec::new();
        self.root_declarations(m, 0, &mut out);
        let r: Arc<[SymId]> = out.into();
        self.roots_memo.insert(m, r.clone());
        r
    }

    /// The declarations that `m` overrides and that override nothing themselves; `m` itself
    /// when it overrides nothing.
    pub(super) fn root_declarations(&mut self, m: SymId, depth: u32, out: &mut Vec<SymId>) {
        let (name, owner) = (self.syms.sym(m).name, self.syms.sym(m).owner);
        let mut overrides = false;
        if let (Owner::Class(c), true) = (owner, depth < 64) {
            self.complete_class(c);
            let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
            for &(b, bt) in bases.iter().skip(1) {
                let mut k = 0;
                while let Some(p) = self.own_alternative(b, name, k) {
                    k += 1;
                    if self.syms.sym(p).kind == SymKind::Def && !self.is_private(p) && self.same_parameters(c, &bases, m, p, bt) {
                        overrides = true;
                        self.root_declarations(p, depth + 1, out);
                    }
                }
            }
        }
        if !overrides && !out.contains(&m) {
            out.push(m);
        }
    }

    /// Two methods of one name that a class inherits from unrelated ancestors, neither of which
    /// overloads the name, would both be called by that name in the output; so would a method
    /// that overrides an alternative of an overloaded name in one ancestor and the only method
    /// of that name in another. The declarations at the root of such methods get suffixed names
    /// like the alternatives of an overloaded name. Which roots that concerns depends on the
    /// classes of the program, not on its method bodies.
    fn suffix_clashing_roots(&mut self, members: &[SymId]) {
        // The roots of the alternatives of one class, of which one at most can keep the plain
        // name, and the roots of one method, which have to agree on its name.
        let mut groups: Vec<(bool, Vec<SymId>)> = Vec::new();
        for i in 0..self.syms.overloads.len() {
            let (set, alts) = (self.syms.overloads[i].0, self.syms.overloads[i].1.clone());
            let Owner::Class(c) = self.syms.sym(set).owner else { continue };
            if self.syms.sym(set).superseded {
                continue;
            }
            if self.syms.class(c).js != JsKind::Scala || (self.in_jar(self.syms.class(c).file) && self.syms.class(c).owner != Owner::Local) {
                continue;
            }
            // In a class of a library body a function class's `apply`, which the runtime calls a
            // function value by, keeps its plain name: scalac let the class have it.
            let body_local = self.syms.class(c).owner == Owner::Local && self.in_jar(self.syms.class(c).file);
            let mut roots = Vec::new();
            for &a in &alts {
                let mut own = self.roots_of(a).to_vec();
                if body_local {
                    own.retain(|&r| !self.is_function_apply(r));
                }
                for &r in own.iter() {
                    if !roots.contains(&r) {
                        roots.push(r);
                    }
                }
                groups.push((true, own));
            }
            groups.push((false, roots));
        }
        for &m in members {
            let mut roots = Vec::new();
            self.root_declarations(m, 0, &mut roots);
            groups.push((true, roots));
        }
        // A root that gets a suffix may share a group with another plain one: until nothing changes.
        for _ in 0..16 {
            let mut changed = false;
            for (one_method, roots) in &groups {
                let plain: Vec<SymId> = roots.iter().copied().filter(|&r| self.keeps_plain_name(r)).collect();
                let clash = match one_method {
                    true => !plain.is_empty() && plain.len() < roots.len(),
                    false => plain.len() > 1,
                };
                if clash {
                    for r in plain {
                        self.suffixed_roots.insert(r, ());
                        if self.alternatives_named {
                            self.renamed_roots.push(r);
                        }
                        // A root named plain before its set was merged is named again.
                        if self.syms.sym(r).dispatch == Dispatch::Plain {
                            self.syms.sym_mut(r).dispatch = Dispatch::Unknown;
                        }
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // What overrides such a root takes its name.
        let roots: Vec<SymId> = self.suffixed_roots.keys().copied().collect();
        let owners: Vec<ClassId> = roots.iter().filter_map(|&r| match self.syms.sym(r).owner {
            Owner::Class(rc) => Some(rc),
            _ => None,
        }).collect();
        let below = self.subclasses.below(&self.syms, &owners);
        for root in roots {
            let (name, Owner::Class(rc)) = (self.syms.sym(root).name, self.syms.sym(root).owner) else { continue };
            for &d in &below[&rc] {
                let mut k = 0;
                while let Some(m) = self.own_alternative(d, name, k) {
                    k += 1;
                    if self.syms.sym(m).dispatch == Dispatch::Plain {
                        self.syms.sym_mut(m).dispatch = Dispatch::Unknown;
                    }
                    if !self.dispatch_pending.contains(&m) {
                        self.dispatch_pending.push(m);
                    }
                }
            }
        }
    }

    /// The output name of what `m` overrides in the ancestors of `c`: `None` when it overrides
    /// nothing.
    fn overridden_dispatch(&mut self, c: ClassId, m: SymId) -> Option<Option<Name>> {
        let name = self.syms.sym(m).name;
        let bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        let mut inherited: Option<Option<Name>> = None;
        for &(b, bt) in &bases[1..] {
            let mut k = 0;
            while let Some(p) = self.own_alternative(b, name, k) {
                k += 1;
                if self.syms.sym(p).kind != SymKind::Def || self.is_private(p) || !self.same_parameters(c, &bases, m, p, bt) {
                    continue;
                }
                let theirs = self.dispatch_name(p);
                match inherited {
                    None => inherited = Some(theirs),
                    Some(ours) if ours != theirs && self.is_library_class(c) => {
                        self.note_name_clash(c, name);
                    }
                    // A class of a jar's body runs only in the interpreter in link mode, which
                    // calls by signature: the names are those of an output not written.
                    Some(ours) if ours != theirs && (self.link_mode() || self.interp) && self.is_body_file(self.syms.class(c).file) => {
                        self.note_name_clash(c, name);
                    }
                    Some(ours) if ours != theirs => {
                        let msg = format!(
                            "{} in {} overrides methods that are called {} and {} in the JavaScript output, since only one of the ancestors overloads it; declare the alternatives in one trait",
                            self.name_str(name),
                            self.class_description(c),
                            self.name_str(ours.unwrap_or(name)),
                            self.name_str(theirs.unwrap_or(name)),
                        );
                        let (file, span) = (self.syms.sym(m).file, self.syms.sym(m).span);
                        self.diags.error(file, span, msg);
                    }
                    Some(_) => {}
                }
            }
        }
        inherited
    }

    /// Two methods of a library class that the output cannot tell apart by name; the census
    /// records them, since a program that calls neither is unaffected.
    fn note_name_clash(&mut self, c: ClassId, name: Name) {
        let what = format!("{}.{}", self.class_path(c), self.name_str(name));
        if self.loaded.is_some() {
            self.with_loader(|w| {
                let loaded = w.loaded_mut();
                if !loaded.bodies.name_clashes.contains(&what) {
                    loaded.bodies.name_clashes.push(what);
                }
            });
        }
    }

    pub(super) fn settle_member(&mut self, c: ClassId, name: Name) {
        self.settled_entry(c, name);
    }

    /// The entry of `c` for `name` with the inherited alternatives merged in.
    fn settled_entry(&mut self, c: ClassId, name: Name) -> Option<SymId> {
        let &entry = self.syms.class(c).members.get(&name)?;
        self.settled(c, name, entry)
    }

    /// `entry`, read as the member `name` of `c`, settled: merged when it is pending, and
    /// read again when another worker settled it between the two reads. A worker publishes
    /// the class's members before the entry's flag (`Worker::lock_released`), so an entry
    /// found settled that the class no longer holds was replaced, and the class's is current.
    #[inline]
    pub(super) fn settled(&mut self, c: ClassId, name: Name, entry: SymId) -> Option<SymId> {
        if self.syms.sym(entry).merge_pending {
            // A lookup of the name from inside its own merge reads the entry as it stands.
            if self.merging.contains(&entry) {
                return Some(entry);
            }
            return self.merge_inherited(c, entry);
        }
        if self.forked && self.shared_class(c) {
            return self.syms.class(c).members.get(&name).copied();
        }
        Some(entry)
    }

    /// `name$Int$String`: the erased parameter types, by their simple names unless another
    /// alternative of `set` would get the same ones. Alternatives that erasure tells apart by
    /// their results alone carry the result as well: `map$Function1$$List`.
    fn suffixed_name(&mut self, m: SymId, set: SymId) -> Name {
        let simple = self.erased_signature(m, false);
        let others: Vec<SymId> = self.syms.alternatives(set).unwrap_or(&[]).iter().copied().filter(|&a| a != m).collect();
        let alike: Vec<SymId> = others.into_iter().filter(|&a| self.erased_signature(a, false) == simple).collect();
        let suffix = if alike.is_empty() {
            simple
        } else {
            let qualified = self.erased_signature(m, true);
            if alike.iter().any(|&a| self.erased_signature(a, true) == qualified) {
                format!("{}$${}", simple, self.erased_result(m, false))
            } else {
                qualified
            }
        };
        let full = format!("{}${}", self.name_ref(self.syms.sym(m).name), suffix);
        self.interner.intern(&full)
    }

    pub(super) fn erased_result(&mut self, m: SymId, qualified: bool) -> String {
        let ret = self.sig_of(m).ret;
        let mut out = String::new();
        self.erased_name(ret, qualified, 0, &mut out);
        out
    }

    /// The parameter types of `m` after erasure, `$`-separated; `0` for none.
    pub(crate) fn erased_signature(&mut self, m: SymId, qualified: bool) -> String {
        let sig = self.parameters_arc(m);
        let mut out = String::new();
        for p in sig.clauses.iter().flat_map(|c| c.params.iter()) {
            if !out.is_empty() {
                out.push('$');
            }
            if p.by_name {
                out.push_str("Function0");
            } else if p.repeated {
                out.push_str("Seq");
            } else {
                self.erased_name(p.ty, qualified, 0, &mut out);
            }
        }
        if out.is_empty() {
            out.push('0');
        }
        out
    }

    fn erased_name(&mut self, t: TypeId, qualified: bool, depth: u32, out: &mut String) {
        let t = self.deref(t);
        if depth > 16 {
            out.push_str("Any");
            return;
        }
        match self.types.get(t) {
            Type::Class(c, args) => {
                if self.syms.class(c).kind == ClassKind::Opaque {
                    self.complete_class(c);
                }
                let info = self.syms.class(c);
                if let (ClassKind::Opaque, Some(under)) = (info.kind, info.underlying) {
                    let subst: Subst = info.tparams.iter().copied().zip(self.types.items(args).iter().copied()).collect();
                    let under = self.types.subst(under, &subst);
                    return self.erased_name(under, qualified, depth + 1, out);
                }
                if qualified {
                    self.owner_path(info.owner, out);
                }
                out.push_str(self.interner.get(self.syms.class(c).name));
                if c == self.b.array {
                    out.push('_');
                    let elem = self.types.items(args)[0];
                    self.erased_name(elem, qualified, depth + 1, out);
                }
            }
            Type::Ctor(c) => out.push_str(self.interner.get(self.syms.class(c).name)),
            Type::Param(p) | Type::AppParam(p, _) => {
                let upper = self.syms.tparam(p).upper;
                self.erased_name(upper, qualified, depth + 1, out)
            }
            // `Null` erases to what stands next to it, as `C | Null` is a `C` after erasure;
            // members that erase alike (`Functor[F] | Derived[Functor[F]]`, an opaque `Derived`)
            // erase to that, others to their join.
            Type::Union(..) => {
                let mut members = Vec::new();
                self.union_alternatives(t, &mut members);
                members.retain(|&m| m != self.b.t_null);
                let mut names: Vec<String> = Vec::with_capacity(members.len());
                for &m in &members {
                    let mut name = String::new();
                    self.erased_name(m, qualified, depth + 1, &mut name);
                    names.push(name);
                }
                names.dedup();
                let rest = match members[..] {
                    [] => self.b.t_null,
                    [one] => one,
                    _ if names.len() == 1 => return out.push_str(&names[0]),
                    _ => members[1..].iter().fold(members[0], |acc, &m| self.types.union(acc, m)),
                };
                if !matches!(self.types.get(rest), Type::Union(..)) {
                    return self.erased_name(rest, qualified, depth + 1, out);
                }
                match self.class_of(rest) {
                    Some(c) => {
                        let joined = self.types.class(c, &[]);
                        self.erased_name(joined, qualified, depth + 1, out)
                    }
                    None => out.push_str("Any"),
                }
            }
            Type::Alias(a, args) => match self.alias_expansion(a, args) {
                Some(expanded) if expanded != t => self.erased_name(expanded, qualified, depth + 1, out),
                _ => out.push_str("Any"),
            },
            // Either order of the members erases alike.
            Type::Inter(a, b) => {
                let (mut x, mut y) = (String::new(), String::new());
                self.erased_name(a, qualified, depth + 1, &mut x);
                self.erased_name(b, qualified, depth + 1, &mut y);
                out.push_str(if x <= y { &x } else { &y });
            }
            Type::Lit(_) => {
                let class = self.widen_lit(t);
                self.erased_name(class, qualified, depth + 1, out)
            }
            Type::Nothing => out.push_str("Nothing"),
            _ => out.push_str("Any"),
        }
    }

    fn owner_path(&self, owner: Owner, out: &mut String) {
        match owner {
            Owner::Package(p) => {
                let info = self.syms.pkg(p);
                if let Some(parent) = info.parent {
                    self.owner_path(Owner::Package(parent), out);
                    out.push_str(self.interner.get(info.name));
                    out.push('_');
                }
            }
            Owner::Class(c) => {
                let info = self.syms.class(c);
                self.owner_path(info.owner, out);
                out.push_str(self.interner.get(info.name));
                out.push('_');
            }
            Owner::Local => {}
        }
    }

    /// Names every alternative of an overloaded name; two of one class that end up with one
    /// name cannot both be called.
    pub(super) fn name_alternatives(&mut self, set: SymId) {
        let alts: Vec<SymId> = self.syms.alternatives(set).unwrap_or(&[]).to_vec();
        let mut names: Vec<Option<Name>> = alts.iter().map(|&a| self.dispatch_name(a)).collect();
        self.check_single_default(set, &alts);
        if let Owner::Class(c) = self.syms.sym(set).owner {
            match self.syms.class(c).js {
                JsKind::Scala => {}
                // The alternatives of a facade are the ways to call one JavaScript function.
                JsKind::Native => return,
                JsKind::Object => {
                    let own: Vec<SymId> = alts.iter().copied().filter(|&a| self.syms.sym(a).owner == Owner::Class(c)).collect();
                    if let Some(&second) = own.get(1).or(own.first()) {
                        let info = self.syms.sym(second);
                        let (file, span) = (info.file, info.span);
                        let msg = format!(
                            "method {} is overloaded in {}, a JavaScript class: its instances have one method of that name, and choosing between the alternatives at run time is not supported",
                            self.name_str(info.name),
                            self.class_description(c)
                        );
                        self.diags.error(file, span, msg);
                    }
                    return;
                }
            }
        }
        // An inline method that is no member of the output has no name to meet another's.
        let mut nameless = Vec::with_capacity(alts.len());
        for &a in alts.iter() {
            let n = self.nameless_inline(a);
            nameless.push(n);
        }
        let inline = |_: &Self, a: SymId| nameless[alts.iter().position(|&x| x == a).unwrap()];
        let c = match self.syms.sym(set).owner {
            Owner::Class(c) => c,
            _ => {
                for i in 0..alts.len() {
                    for j in i + 1..alts.len() {
                        let named_alike = names[i] == names[j] && !inline(self, alts[i]) && !inline(self, alts[j]);
                        if named_alike || self.identical_parameters(alts[i], alts[j]) {
                            self.report_double_definition(alts[i], alts[j]);
                        }
                    }
                }
                return;
            }
        };
        // An alternative whose name is being computed up the stack has none to compare yet.
        if alts.iter().any(|&a| self.syms.sym(a).dispatch == Dispatch::InProgress) {
            return;
        }
        for i in 0..alts.len() {
            for j in i + 1..alts.len() {
                let own = |t: &Self, a: SymId| t.syms.sym(a).owner == Owner::Class(c);
                let both_own = own(self, alts[i]) && own(self, alts[j]);
                let named_alike = names[i] == names[j] && !inline(self, alts[i]) && !inline(self, alts[j]);
                // Two definitions with the same parameter types differ in their results at most.
                let same_name = named_alike || (both_own && self.identical_parameters(alts[i], alts[j]));
                // A class of a jar passed scalac's check of double definitions: what teq's erasure
                // does not tell apart (a `@targetName` it does not read, a default getter beside a
                // method of that name and result) is named apart like a diamond.
                let library = self.is_library_class(c) || (self.syms.class(c).owner == Owner::Local && self.in_jar(self.syms.class(c).file));
                if !same_name || (both_own && !library && self.report_double_definition(alts[i], alts[j])) {
                    continue;
                }
                let name = self.syms.sym(alts[i]).name;
                // A library's diamond is named apart: each of the two methods takes its erased
                // signature as its suffix, as the alternatives of one declaration do. Where one
                // of the two is declared below the other, or in a jar where the other is the
                // std's or the program's (zio's `ChunkIterator.++` beside `IterableOps.++` in a
                // class that takes both), that one alone takes the suffix, and the other keeps
                // the name the classes that know only it call it by; not when that one
                // overrides a method, whose callers know it by the name of what it overrides
                // (monocle's `PTraversal.some` overrides `PSetter.some` beside `Fold.some`).
                if library {
                    self.note_name_clash(c, name);
                    let class_of = |t: &Self, a: SymId| match t.syms.sym(a).owner {
                        Owner::Class(k) => Some(k),
                        _ => None,
                    };
                    let below = |t: &Self, a: SymId, b: SymId| match (class_of(t, a), class_of(t, b)) {
                        (Some(x), Some(y)) => x != y && t.syms.class(x).base_types.iter().any(|&(k, _)| k == y),
                        _ => false,
                    };
                    let from_jar = |t: &Self, a: SymId| class_of(t, a).map_or(false, |k| t.is_library_class(k));
                    let root = |t: &mut Self, a: SymId| *t.roots_of(a) == [a];
                    let apart: &[usize] = if below(self, alts[i], alts[j]) && root(self, alts[i]) {
                        &[i]
                    } else if below(self, alts[j], alts[i]) && root(self, alts[j]) {
                        &[j]
                    } else if from_jar(self, alts[i]) && !from_jar(self, alts[j]) && root(self, alts[i]) {
                        &[i]
                    } else if from_jar(self, alts[j]) && !from_jar(self, alts[i]) && root(self, alts[j]) {
                        &[j]
                    } else {
                        &[i, j]
                    };
                    for &k in apart {
                        if names[k].is_none() {
                            names[k] = Some(self.name_apart(alts[k], set));
                        }
                    }
                    continue;
                }
                let msg = format!(
                    "{} inherits {} as {} and as {}, which are both called {} in the JavaScript output, since no ancestor declares the two together; declare the alternatives in one trait",
                    self.class_description(c),
                    self.name_str(name),
                    self.sig_string(alts[i]),
                    self.sig_string(alts[j]),
                    self.name_str(names[i].unwrap_or(name)),
                );
                let (file, span) = (self.syms.class(c).file, self.syms.class(c).span);
                self.diags.error(file, span, msg);
            }
        }
    }

    /// Names `m`, a method of a library class that shares its output name with an unrelated
    /// one, by its erased signature, along with the methods overriding it in the classes named
    /// so far, whose names were settled from its plain one: the jars' classes compiled so far,
    /// and the program's and the std's once the final pass has named them.
    fn name_apart(&mut self, m: SymId, set: SymId) -> Name {
        let n = self.suffixed_name(m, set);
        self.syms.dispatch_names.insert(m, n);
        self.syms.sym_mut(m).dispatch = Dispatch::Named;
        let name = self.syms.sym(m).name;
        let Owner::Class(owner) = self.syms.sym(m).owner else { return n };
        let classes: Vec<ClassId> = self.syms.classes.entries().map(|(i, _)| ClassId(i)).collect();
        // A metadata-only read of every class, the ids of its bases alone (`class_raw`).
        let below: Vec<ClassId> = classes
            .into_iter()
            .filter(|&k| k != owner && self.names_settled(k) && self.syms.class_raw(k).base_types.iter().any(|&(b, _)| b == owner))
            .collect();
        for k in below {
            let bases = self.syms.class(k).base_types.clone();
            let Some(&(_, owner_ty)) = bases.iter().find(|&&(b, _)| b == owner) else { continue };
            let mut i = 0;
            while let Some(p) = self.own_alternative(k, name, i) {
                i += 1;
                if p != m && matches!(self.syms.sym(p).kind, SymKind::Def | SymKind::Val) && self.syms.sym(p).dispatch != Dispatch::Named && self.same_parameters(k, &bases, p, m, owner_ty) {
                    self.syms.dispatch_names.insert(p, n);
                    self.syms.sym_mut(p).dispatch = Dispatch::Named;
                }
            }
        }
        n
    }

    /// Whether the members of `k` have the names they go by in the output, as far as anything
    /// settled so far: a jar's class once compiled, any other class once the final pass ran.
    fn names_settled(&self, k: ClassId) -> bool {
        let info = self.syms.class_raw(k);
        info.named_for_output || (self.alternatives_named && !self.in_jar(info.file))
    }

    /// Default arguments are allowed on one of the alternatives that a scope defines.
    fn check_single_default(&mut self, set: SymId, alts: &[SymId]) {
        let owner = self.syms.sym(set).owner;
        let mut with_defaults: Vec<SymId> = Vec::new();
        for &a in alts {
            if self.syms.sym(a).owner != owner {
                continue;
            }
            let sig = self.parameters_of(a);
            if sig.clauses.iter().any(|c| c.params.iter().any(|p| p.has_default)) {
                with_defaults.push(a);
            }
        }
        with_defaults.sort_by_key(|&a| self.syms.sym(a).span.start);
        if let Some(&second) = with_defaults.get(1) {
            let info = self.syms.sym(second);
            let (file, span) = (info.file, info.span);
            let msg = format!("two or more overloaded variants of method {} have default arguments", self.name_str(info.name));
            self.diags.error(file, span, msg);
        }
    }

    /// scalac's E120: two definitions of one scope that erasure does not tell apart. With the
    /// same parameter types the second one is a plain redefinition.
    pub(super) fn report_double_definition(&mut self, first: SymId, second: SymId) -> bool {
        let (first, second) = match self.syms.sym(first).span.start <= self.syms.sym(second).span.start {
            true => (first, second),
            false => (second, first),
        };
        let identical = self.identical_parameters(first, second) && self.syms.sym(first).name != names::INIT;
        let (a, b) = (self.erased_signature(first, true), self.erased_signature(second, true));
        if !identical && (a != b || self.erased_result(first, true) != self.erased_result(second, true)) {
            return false;
        }
        if !identical && self.target_name(first) != self.target_name(second) {
            return true;
        }
        let name = self.syms.sym(first).name;
        let (file, span) = (self.syms.sym(second).file, self.syms.sym(second).span);
        let msg = if identical {
            self.already_defined(name, first)
        } else {
            let line = |t: &Self, s: SymId| {
                let info = t.syms.sym(s);
                crate::source::locate(&t.source(info.file).text, info.span.start as usize).0
            };
            let place = match self.syms.sym(first).owner {
                Owner::Class(c) => format!(" in {}", self.class_description(c)),
                _ => String::new(),
            };
            format!(
                "Conflicting definitions:\ndef {n}{} {place}at line {} and\ndef {n}{} {place}at line {}\nhave the same type after erasure.",
                self.sig_string(first),
                line(self, first),
                self.sig_string(second),
                line(self, second),
                n = self.name_str(name),
                place = if place.is_empty() { String::new() } else { format!("{} ", &place[1..]) },
            )
        };
        self.diags.error(file, span, msg);
        true
    }

    pub(super) fn identical_parameters(&mut self, a: SymId, b: SymId) -> bool {
        let types = |sig: &MethodSig| sig.tparams.is_empty().then(|| sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| (p.ty, p.by_name, p.repeated))).collect::<Vec<_>>());
        let ta = types(self.parameters_of(a));
        let tb = types(self.parameters_of(b));
        ta.is_some() && ta == tb
    }

    /// Settles what no lookup has settled and names the alternatives of every overloaded name,
    /// so that the output does not depend on which of them the program happened to call.
    pub(super) fn name_all_alternatives(&mut self) {
        let mut members: Vec<SymId> = std::mem::take(&mut self.dispatch_pending);
        for (c, entry) in std::mem::take(&mut self.unsettled_std_entries) {
            if self.syms.sym(entry).merge_pending {
                self.merge_inherited(c, entry);
            }
            if self.syms.alternatives(entry).is_none() {
                members.push(entry);
            }
        }
        let mut i = 0;
        while i < self.syms.overloads.len() {
            let set = self.syms.overloads[i].0;
            if let (true, Owner::Class(c)) = (self.syms.sym(set).merge_pending, self.syms.sym(set).owner) {
                if !self.in_jar(self.syms.sym(set).file) {
                    self.merge_inherited(c, set);
                }
            }
            i += 1;
        }
        if self.syms.overloads.is_empty() && members.is_empty() && self.named_like.is_empty() {
            self.alternatives_named = true;
            return;
        }
        // The sets that settling an entry merges while the roots are told apart (a std class
        // entered on demand) are told apart in the loop, before they are named.
        let mut suffixed = self.syms.overloads.len();
        self.suffix_clashing_roots(&members);
        members.extend(std::mem::take(&mut self.dispatch_pending));
        self.alternatives_named = true;
        let mut i = 0;
        while i < self.syms.overloads.len() {
            if i >= suffixed {
                suffixed = self.syms.overloads.len();
                self.suffix_clashing_roots(&[]);
            }
            let set = self.syms.overloads[i].0;
            if !self.in_jar(self.syms.sym(set).file) && !self.syms.sym(set).superseded {
                self.name_alternatives(set);
            }
            i += 1;
        }
        members.extend(std::mem::take(&mut self.dispatch_pending));
        for m in members {
            self.dispatch_name(m);
        }
        // A root told apart by a later round, whose set the loop had passed, is named now:
        // the names are the same whichever order the sets stand in.
        let roots: Vec<SymId> = self.suffixed_roots.keys().copied().collect();
        for r in roots {
            if self.syms.sym(r).dispatch == Dispatch::Unknown {
                self.dispatch_name(r);
            }
        }
        for (m, root) in std::mem::take(&mut self.named_like) {
            self.name_like(m, root);
        }
        self.add_bridges();
    }

    /// Names the alternatives of the overload sets entered since `from`, those of a std file
    /// entered after the final passes, and the members whose names wait on them.
    pub(super) fn name_late_alternatives(&mut self, from: usize) {
        let mut members: Vec<SymId> = std::mem::take(&mut self.dispatch_pending);
        for (c, entry) in std::mem::take(&mut self.unsettled_std_entries) {
            if self.syms.sym(entry).merge_pending {
                self.merge_inherited(c, entry);
            }
            if self.syms.alternatives(entry).is_none() {
                members.push(entry);
            }
        }
        let mut i = from;
        while i < self.syms.overloads.len() {
            let set = self.syms.overloads[i].0;
            if let (true, Owner::Class(c)) = (self.syms.sym(set).merge_pending, self.syms.sym(set).owner) {
                if !self.in_jar(self.syms.sym(set).file) {
                    self.merge_inherited(c, set);
                }
            }
            i += 1;
        }
        // The roots suffixed so far and the new ones name what overrides them, in every class.
        let mut suffixed = self.syms.overloads.len();
        self.suffix_clashing_roots(&members);
        members.extend(std::mem::take(&mut self.dispatch_pending));
        let mut i = from;
        while i < self.syms.overloads.len() {
            if i >= suffixed {
                suffixed = self.syms.overloads.len();
                self.suffix_clashing_roots(&[]);
            }
            let set = self.syms.overloads[i].0;
            if !self.in_jar(self.syms.sym(set).file) && !self.syms.sym(set).superseded {
                self.name_alternatives(set);
            }
            i += 1;
        }
        members.extend(std::mem::take(&mut self.dispatch_pending));
        for m in members {
            self.dispatch_name(m);
        }
        for (m, root) in std::mem::take(&mut self.named_like) {
            self.name_like(m, root);
        }
    }

    /// Where the type arguments a class passes to its parents make an inherited declaration and
    /// an inherited method one member (`myMethod(t: T)` of a `Simple[T]` and `myMethod(t:
    /// String)` of a `Sized[T] extends Simple[T]`, in a `class Bad extends Sized[String]`), the
    /// two may go by different names in the output: the class gets a bridge from the name of
    /// the declaration to the implementation.
    pub(super) fn add_bridges(&mut self) {
        self.bridge_classes(|_, _| true);
    }

    /// The bridges of the classes a retype makes anew, those of `files`
    /// (`Symbols::class_of_files`). Every other class keeps the bridges its check gave it: a
    /// library class is bridged once when it is compiled, with the names settled by then, and a
    /// fresh build settles the names of its library members while it reaches them, after its
    /// own bridges are made.
    pub(super) fn add_bridges_of(&mut self, files: &[FileId]) {
        self.bridge_classes(|syms, c| syms.class_of_files(c, files));
    }

    fn bridge_classes(&mut self, of: impl Fn(&Symbols, ClassId) -> bool) {
        for i in 0..self.syms.classes.len() {
            let c = ClassId(i as u32);
            if self.in_jar(self.syms.class(c).file) || !of(&self.syms, c) {
                continue;
            }
            self.inherit_overloads(c);
        }
        for i in 0..self.prog.classes.len() {
            let c = self.prog.classes[i].id;
            if !of(&self.syms, c) {
                continue;
            }
            let bridges = self.bridges_of(c);
            self.prog.classes[i].bridges = bridges;
        }
    }

    fn inherit_overloads(&mut self, c: ClassId) {
        let inherits = self.syms.class(c).base_types.iter().skip(1).any(|&(b, _)| self.syms.class(b).has_overloads);
        if inherits {
            self.syms.class_mut(c).has_overloads = true;
        }
    }

    /// The bridges of a class checked after the final passes ran: a std class the reach pass
    /// met.
    pub(super) fn bridge_class(&mut self, c: ClassId) {
        self.inherit_overloads(c);
        let bridges = self.bridges_of(c);
        if let Some(tc) = self.prog.classes.rfind_mut(|tc| tc.id == c) {
            tc.bridges = bridges;
        }
    }

    fn bridges_of(&mut self, c: ClassId) -> Vec<(SymId, SymId)> {
        let info = self.syms.class(c);
        if !info.has_overloads || info.kind == ClassKind::Trait || info.js != JsKind::Scala {
            return Vec::new();
        }
        let bases: Vec<(ClassId, TypeId)> = info.base_types.clone();
        let mut bridges: Vec<(SymId, SymId)> = Vec::new();
        for &(b, _) in &bases[1..] {
            for j in 0..self.syms.class(b).member_order.len() {
                let m = self.syms.class(b).member_order[j];
                let minfo = self.syms.sym(m);
                if minfo.kind != SymKind::Def || minfo.is_extension || self.is_private(m) {
                    continue;
                }
                if !(minfo.alternative || minfo.meets_inherited || minfo.dispatch == Dispatch::Named) {
                    continue;
                }
                let Some(implementation) = self.implementation_of(c, &bases, m) else { continue };
                // A value implementing a parameterless method of an overloaded name keeps its
                // plain name as the field's accessor, so a bridge answers the method's name.
                let value = matches!(self.syms.sym(implementation).kind, SymKind::Val | SymKind::Var);
                if implementation == m || (self.syms.sym(implementation).owner == Owner::Class(c) && !value) {
                    continue;
                }
                let (declared, actual) = (self.dispatch_name(m), self.dispatch_name(implementation));
                if declared != actual && !bridges.iter().any(|&(d, _)| self.dispatch_name(d) == declared) {
                    bridges.push((m, implementation));
                }
            }
        }
        bridges
    }

    /// The first concrete method in the linearisation of `c` with the parameters of `m` as `c`
    /// sees them.
    pub(super) fn implementation_of(&mut self, c: ClassId, bases: &[(ClassId, TypeId)], m: SymId) -> Option<SymId> {
        let name = self.syms.sym(m).name;
        let Owner::Class(mc) = self.syms.sym(m).owner else { return None };
        let &(_, m_owner) = bases.iter().find(|&&(b, _)| b == mc)?;
        let parameterless = self.sig_of(m).clauses.iter().all(|cl| cl.is_using);
        for &(b, _) in bases {
            let mut k = 0;
            while let Some(s) = self.own_alternative(b, name, k) {
                k += 1;
                let kind = self.syms.sym(s).kind;
                let value = parameterless && matches!(kind, SymKind::Val | SymKind::Var);
                if !(kind == SymKind::Def || value) || self.is_abstract_member(s) || self.is_private(s) {
                    continue;
                }
                if s == m || value || self.same_parameters(c, bases, s, m, m_owner) {
                    return Some(s);
                }
            }
        }
        None
    }

    // ---- resolution ----

    fn alternatives_of(&mut self, set: SymId, recv_ty: Option<TypeId>, ctor: Option<ClassId>) -> Vec<Alt> {
        let syms: Vec<SymId> = match self.syms.alternatives(set) {
            Some(alts) => alts.to_vec(),
            None => vec![set],
        };
        // An alternative the call site cannot access is no candidate: zio's `protected
        // someOrElse[B]`, kept for binary compatibility beside the public one.
        let restricted = crate::ast::mods::PRIVATE | crate::ast::mods::PROTECTED;
        let syms = if syms.iter().any(|&s| self.syms.sym(s).mods & restricted != 0) {
            let accessible: Vec<SymId> = syms.iter().copied().filter(|&s| self.is_given_accessible(s)).collect();
            if accessible.is_empty() { syms } else { accessible }
        } else {
            syms
        };
        // One of a jar that is `private[p]` or `protected[p]` is none outside `p` (http4s's
        // deprecated `private[client] def translate` beside the public one), a protected one
        // apart from subclasses.
        let qualified = |t: &Self, s: SymId| t.syms.sym(s).mods & crate::ast::mods::QUALIFIED != 0;
        let syms = if syms.iter().any(|&s| qualified(self, s)) {
            let accessible: Vec<SymId> = syms.iter().copied().filter(|&s| self.qualified_reachable(s)).collect();
            if accessible.is_empty() { syms } else { accessible }
        } else {
            syms
        };
        let mut alts: Vec<Alt> = syms
            .into_iter()
            .map(|sym| {
                let owner_subst = match (recv_ty, self.syms.sym(sym).owner) {
                    (Some(rt), Owner::Class(oc)) => match self.base_type(rt, oc) {
                        Some(bt) => self.owner_subst(bt),
                        None => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                Alt { sym, sig: self.sig_arc(sym), owner_subst, ctor: None }
            })
            .collect();
        if let Some(c) = ctor {
            self.complete_class(c);
            let info = self.syms.class(c);
            let sig = Arc::new(MethodSig { tparams: info.tparams.clone(), clauses: info.ctor.clone(), ret: info.base_types[0].1 });
            // An `apply` with the parameters of the constructor takes its place, as the one
            // scalac would synthesize for a case class is not made then; a library's synthetic
            // `apply` has them under type parameters of its own.
            let types = |sig: &MethodSig| -> Vec<TypeId> { sig.clauses.iter().flat_map(|cl| cl.params.iter().map(|p| p.ty)).collect() };
            let ctor_types = types(&sig);
            let mut replaced = false;
            for a in &alts {
                if a.sig.tparams.len() != sig.tparams.len() {
                    continue;
                }
                let subst: Subst = a.sig.tparams.iter().zip(&sig.tparams).map(|(&x, &y)| (x, self.types.param(y))).collect();
                let a_types: Vec<TypeId> = types(&a.sig).iter().map(|&t| self.types.subst(t, &subst)).collect();
                if a_types == ctor_types {
                    replaced = true;
                    break;
                }
            }
            if !replaced {
                alts.push(Alt { sym: set, sig, owner_subst: Vec::new(), ctor: Some(c) });
            }
        }
        alts
    }

    /// The alternative of the member set `set` of `recv_ty` that the argument lists select, with
    /// its owner's type arguments, chosen without applying it: dotty types a structural call
    /// before `addClassOfs` looks at the method its selection resolved to.
    pub(super) fn selected_alternative(&mut self, set: SymId, recv_ty: TypeId, lists: &mut Vec<ArgList>) -> Option<(SymId, Subst)> {
        let alts = self.alternatives_of(set, Some(recv_ty), None);
        let i = if alts.len() == 1 {
            0
        } else {
            let outer_base = std::mem::replace(&mut self.app_base, self.tvars.len() as u32);
            let mut infos = Vec::new();
            let choice = self.resolve_overloaded(&alts, None, lists, &mut infos, None, false, true);
            self.app_base = outer_base;
            choice.ok()?
        };
        Some((alts[i].sym, alts[i].owner_subst.clone()))
    }

    /// Applies the alternative of `set` that the call means.
    #[inline(never)]
    pub(super) fn apply_overloaded(
        &mut self,
        recv: Option<TExprId>,
        recv_ty: Option<TypeId>,
        set: SymId,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        match self.try_overloaded(recv, recv_ty, set, None, false, targs, lists, span, expected) {
            Ok(r) => r,
            Err(failed) => self.report_unresolved(failed, span),
        }
    }

    pub(super) fn report_unresolved(&mut self, failed: Unresolved, span: Span) -> (TExprId, TypeId) {
        if failed.dependent {
            self.dependent_error(span, failed.msg);
        } else {
            self.error(span, failed.msg);
        }
        self.type_args_for_errors(&failed.lists);
        (self.prog.add(TExpr::Unit), ERROR)
    }

    /// Like `apply_overloaded`, but hands the arguments back with the message when no
    /// alternative is meant, so that the caller can try something else first.
    #[inline(never)]
    pub(super) fn try_overloaded(
        &mut self,
        recv: Option<TExprId>,
        recv_ty: Option<TypeId>,
        set: SymId,
        ctor: Option<ClassId>,
        strict: bool,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Result<(TExprId, TypeId), Unresolved> {
        let outer_base = std::mem::replace(&mut self.app_base, self.tvars.len() as u32);
        let first_var = self.tvars.len();
        let mut alts = self.alternatives_of(set, recv_ty, ctor);
        // An alternative whose header the parser could not complete is no candidate while a
        // complete one is.
        let incomplete = |t: &Self, a: &Alt| a.ctor.is_none() && t.syms.sym(a.sym).mods & crate::ast::mods::INCOMPLETE != 0;
        // A set whose every alternative is incomplete is one incomplete method: its arguments
        // are typed on their own and the application has the error type, before any narrowing.
        if self.recovered && !alts.is_empty() && alts.iter().all(|a| incomplete(self, a)) {
            self.app_base = outer_base;
            self.type_args_for_errors(&lists);
            return Ok((self.prog.add(TExpr::Unit), ERROR));
        }
        let left_out = self.recovered && alts.iter().any(|a| incomplete(self, a)) && alts.iter().any(|a| !incomplete(self, a));
        if left_out {
            alts.retain(|a| !incomplete(self, a));
        }
        // A lone complete alternative still has to take the arguments, or the one left out
        // is meant.
        let strict = strict || left_out;
        // Explicit type arguments keep the alternatives with that many type parameters, as
        // scalac narrows the set before typing the arguments: `Array[Byte](71, 73)` is the
        // generic `apply`, whose parameter type then types the literals.
        if let Some(l) = targs {
            let n = self.cur_ast().ty_list(l).len();
            if alts.iter().filter(|a| a.sig.tparams.len() == n).count() > 0 {
                alts.retain(|a| a.sig.tparams.len() == n);
            }
        }
        let mut lists = lists;
        // A library body carries the arguments scalac inferred for a using clause as a plain
        // list: an alternative with fewer plain clauses than the lists takes them as such.
        if self.is_body_file(self.env.file) {
            let plain = lists.iter().filter(|l| !l.using).count();
            for alt in &mut alts {
                if plain > alt.explicit_clauses().count() && alt.sig.clauses.iter().any(|c| c.is_using) {
                    let mut sig = (*alt.sig).clone();
                    for c in &mut sig.clauses {
                        c.is_using = false;
                    }
                    alt.sig = Arc::new(sig);
                }
            }
        }
        let mut infos: Vec<Option<ListInfo>> = Vec::new();
        // The arguments as written, whose types a failure records for signature help.
        let written: Option<Vec<Vec<ArgSrc>>> = self.index.is_some().then(|| lists.iter().map(|l| l.args.clone()).collect());
        let (diag_mark, match_mark) = (self.diags.items.len(), self.deferred_matches.len());
        let prof = self.prof(Kind::Overload, span, About::Sym(set));
        let choice = if targs.is_some() && alts.len() == 1 {
            Ok(0)
        } else {
            self.resolve_overloaded(&alts, targs, &mut lists, &mut infos, expected, strict, true)
        };
        if let Some(p) = prof {
            self.profile.exit_counted(p, overload_outcome(&choice));
        }
        let result = match choice {
            Ok(i) => {
                let alt = &alts[i];
                let retyped = self.pass_typed_lambdas(alt, &mut lists, &infos);
                let r = match alt.ctor {
                    Some(c) => self.apply_companion_apply(c, targs, lists, span, expected),
                    None => {
                        self.check_access(alt.sym, None, span);
                        // A deferred inline alternative of a parameter's member: the arguments
                        // chose it on the declared type, the argument's class implements it.
                        let dispatched = match (recv, recv_ty) {
                            (Some(r), Some(rt)) => self.deferred_implementation(r, rt, alt.sym),
                            _ => None,
                        };
                        match self.value_alternative(recv, recv_ty, alt.sym) {
                            Some(callee) => self.apply_callee(callee, targs, lists, span, expected),
                            None if dispatched.is_some() => {
                                let (sym, owner_ty) = dispatched.unwrap();
                                let callee = self.member_callee_in(recv.unwrap(), recv_ty.unwrap(), Some(owner_ty), sym);
                                self.apply_callee(callee, targs, lists, span, expected)
                            }
                            None => {
                                let prefix = match (recv, recv_ty) {
                                    (Some(r), Some(rt)) => self.dependent_prefix(alt.sym, r, rt),
                                    _ => None,
                                };
                                if let (Some(_), Some(r)) = (prefix, recv) {
                                    self.note_outer_prefix(alt.sym, r);
                                }
                                let call = MethodCall { recv, sym: alt.sym, owner_subst: alt.owner_subst.clone(), ext_recv: None, prefix };
                                self.apply_method(call, None, targs, lists, span, expected, false).unwrap()
                            }
                        }
                    }
                };
                if retyped {
                    self.drop_repeated_diagnostics(diag_mark);
                    self.quiet_repeated_matches(match_mark);
                }
                Ok(r)
            }
            Err(failure) => {
                self.type_plain_args(&mut lists);
                if let Some(written) = written {
                    self.index_failed_args(&written, &lists);
                }
                let msg = self.overload_failure(set, &alts, &failure, &lists, &infos, expected);
                Err(Unresolved { msg, lists, none_applicable: matches!(failure, Failure::NoneApplicable), dependent: left_out })
            }
        };
        self.settle_overload_vars(first_var, result.as_ref().ok().map(|&(_, ty)| ty));
        self.app_base = outer_base;
        result
    }

    /// Variables of the arguments that were typed before an alternative was known belong to
    /// no application that would solve them. One that nothing constrains stays open where
    /// the result mentions it, for the enclosing application to settle.
    fn settle_overload_vars(&mut self, first_var: usize, result: Option<TypeId>) {
        let mut in_result = Vec::new();
        if let Some(ty) = result {
            let ty = self.zonk(ty);
            self.collect_vars(ty, &mut in_result);
        }
        for v in first_var..self.tvars.len() {
            let info = &self.tvars[v];
            let unconstrained = info.lower.is_empty() && info.upper.is_empty();
            if info.inst.is_some() || info.guide || (unconstrained && in_result.contains(&self.tvars.id(v))) {
                continue;
            }
            self.solve_var(self.tvars.id(v));
        }
    }

    /// `new C(args)` for a class with secondary constructors: the primary constructor, when it
    /// is accessible, and the secondaries are the alternatives.
    pub(super) fn construct_overloaded(
        &mut self,
        c: ClassId,
        primary: Option<(SymId, Arc<MethodSig>)>,
        secondaries: Vec<(SymId, Arc<MethodSig>)>,
        targs: Option<ListRef>,
        lists: Vec<ArgList>,
        span: Span,
        expected: Option<TypeId>,
    ) -> (TExprId, TypeId) {
        let outer_base = std::mem::replace(&mut self.app_base, self.tvars.len() as u32);
        let first_var = self.tvars.len();
        let mut alts: Vec<Alt> = Vec::new();
        if let Some((sym, sig)) = &primary {
            alts.push(Alt { sym: *sym, sig: sig.clone(), owner_subst: Vec::new(), ctor: Some(c) });
        }
        for (s, sig) in secondaries {
            alts.push(Alt { sym: s, sig, owner_subst: Vec::new(), ctor: None });
        }
        let mut lists = lists;
        let mut infos: Vec<Option<ListInfo>> = Vec::new();
        let (diag_mark, match_mark) = (self.diags.items.len(), self.deferred_matches.len());
        let prof = self.prof(Kind::Overload, span, alts.first().map_or(About::Class(c), |a| About::Sym(a.sym)));
        let choice = self.resolve_overloaded(&alts, targs, &mut lists, &mut infos, expected, false, true);
        if let Some(p) = prof {
            self.profile.exit_counted(p, overload_outcome(&choice));
        }
        let result = match choice {
            Ok(i) => {
                let alt = &alts[i];
                let retyped = self.pass_typed_lambdas(alt, &mut lists, &infos);
                let r = match alt.ctor {
                    Some(_) => {
                        let call = MethodCall { recv: None, sym: SymId(u32::MAX), owner_subst: Vec::new(), ext_recv: None, prefix: None };
                        self.apply_method(call, Some((c, alt.sig.clone())), targs, lists, span, expected, false).unwrap()
                    }
                    None => {
                        self.check_access(alt.sym, None, span);
                        let call = MethodCall { recv: None, sym: alt.sym, owner_subst: Vec::new(), ext_recv: None, prefix: None };
                        self.apply_method(call, None, targs, lists, span, expected, false).unwrap()
                    }
                };
                if retyped {
                    self.drop_repeated_diagnostics(diag_mark);
                    self.quiet_repeated_matches(match_mark);
                }
                r
            }
            Err(failure) => {
                self.type_plain_args(&mut lists);
                let first = alts.first().map_or(SymId(u32::MAX), |a| a.sym);
                let msg = self.overload_failure(first, &alts, &failure, &lists, &infos, expected);
                self.report_unresolved(Unresolved { msg, lists, none_applicable: false, dependent: false }, span)
            }
        };
        self.settle_overload_vars(first_var, Some(result.1));
        self.app_base = outer_base;
        result
    }

    /// The alternative that `super.name(args)` means in the class `c`, when the classes behind
    /// it have methods of that name with different parameters; `None` when they are all one
    /// method, which the caller finds by its name.
    pub(super) fn super_alternative(
        &mut self,
        c: ClassId,
        bases: &[(ClassId, TypeId)],
        name: Name,
        targs: Option<ListRef>,
        lists: &mut Vec<ArgList>,
        expected: Option<TypeId>,
    ) -> Option<SymId> {
        if !self.syms.class(c).has_overloads {
            return None;
        }
        // One method per parameter list, the first in the linearisation.
        let mut distinct: Vec<(SymId, TypeId)> = Vec::new();
        let all_bases: Vec<(ClassId, TypeId)> = self.syms.class(c).base_types.clone();
        for &(b, bt) in bases {
            let mut k = 0;
            while let Some(m) = self.own_alternative(b, name, k) {
                k += 1;
                if self.syms.sym(m).kind != SymKind::Def || self.is_private(m) {
                    continue;
                }
                if !distinct.iter().any(|&(d, _)| self.same_parameters(c, &all_bases, d, m, bt)) {
                    distinct.push((m, bt));
                }
            }
        }
        if distinct.len() < 2 {
            return None;
        }
        let alts: Vec<Alt> = distinct
            .iter()
            .map(|&(sym, bt)| Alt { sym, sig: self.sig_arc(sym), owner_subst: self.owner_subst(bt), ctor: None })
            .collect();
        let mut infos = Vec::new();
        match self.resolve_overloaded(&alts, targs, lists, &mut infos, expected, false, false) {
            Ok(i) => Some(alts[i].sym),
            Err(_) => None,
        }
    }

    /// Which of several extension methods that take the receiver equally well a call means:
    /// to Scala they are overloaded methods whose first argument list is the receiver, so the
    /// argument lists that follow decide. `None` leaves the choice to the caller.
    pub(super) fn pick_by_arguments(
        &mut self,
        calls: &[MethodCall],
        recv: TExprId,
        recv_ty: TypeId,
        lists: &mut Vec<ArgList>,
        expected: Option<TypeId>,
    ) -> Option<usize> {
        self.resolve_extension_overload(calls, None, recv, recv_ty, lists, expected).ok()
    }

    /// The one of same-named extensions of an instance that the receiver, as the first list,
    /// and the lists after it choose, with the call's explicit type arguments, as scalac's
    /// `resolveOverloaded` chooses among the alternatives on the arguments typed alone; where
    /// none or several apply, its E134 or E051.
    pub(super) fn resolve_extension_overload(
        &mut self,
        calls: &[MethodCall],
        targs: Option<ListRef>,
        recv: TExprId,
        recv_ty: TypeId,
        lists: &mut Vec<ArgList>,
        expected: Option<TypeId>,
    ) -> Result<usize, String> {
        let alts: Vec<Alt> = calls
            .iter()
            .map(|call| Alt { sym: call.sym, sig: self.sig_arc(call.sym), owner_subst: call.owner_subst.clone(), ctor: None })
            .collect();
        let span = lists.first().map_or(Span::default(), |l| l.span);
        lists.insert(0, ArgList { args: vec![ArgSrc::Typed(recv, recv_ty)], using: false, span });
        let mut infos = Vec::new();
        let choice = self.resolve_overloaded(&alts, targs, lists, &mut infos, expected, false, false);
        let result = match choice {
            Ok(i) => Ok(i),
            Err(failure) => {
                self.type_plain_args(lists);
                let set = alts[0].sym;
                Err(self.overload_failure(set, &alts, &failure, lists, &infos, expected))
            }
        };
        lists.remove(0);
        result
    }

    /// The alternative the lists choose (`resolve_choice`); in an index session the names of its
    /// arguments a candidate takes a function of are recorded then, for the candidates that
    /// remain on every way out: the one chosen, those left ambiguous, or where none applies any
    /// that took one (`ListInfo::function_names`), so that a later list eliminating a candidate
    /// takes back what an earlier list's argument would have recorded for it.
    #[allow(clippy::too_many_arguments)]
    fn resolve_overloaded(
        &mut self,
        alts: &[Alt],
        targs: Option<ListRef>,
        lists: &mut Vec<ArgList>,
        infos: &mut Vec<Option<ListInfo>>,
        expected: Option<TypeId>,
        strict: bool,
        open: bool,
    ) -> Result<usize, Failure> {
        let choice = self.resolve_choice(alts, targs, lists, infos, expected, strict, open);
        if self.index.is_some() {
            let remaining: Option<&[usize]> = match &choice {
                Ok(i) => Some(std::slice::from_ref(i)),
                Err(Failure::Ambiguous(best)) => Some(best),
                Err(Failure::NoneApplicable) => None,
            };
            for info in infos.iter_mut().flatten() {
                for (name, te, takers) in std::mem::take(&mut info.function_names) {
                    if remaining.map_or(true, |r| takers.iter().any(|t| r.contains(t))) {
                        self.index_function_name(name, te);
                    }
                }
            }
        }
        choice
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_choice(
        &mut self,
        alts: &[Alt],
        targs: Option<ListRef>,
        lists: &mut Vec<ArgList>,
        infos: &mut Vec<Option<ListInfo>>,
        expected: Option<TypeId>,
        strict: bool,
        open: bool,
    ) -> Result<usize, Failure> {
        let mut cands: Vec<usize> = (0..alts.len()).collect();
        let explicit: Vec<TypeId> = match targs {
            Some(l) => {
                let ids = self.cur_ast().ty_list(l).to_vec();
                ids.iter().map(|&t| self.resolve_type_ctor(t)).collect()
            }
            None => Vec::new(),
        };
        if !explicit.is_empty() {
            let fitting: Vec<usize> =
                cands.iter().copied().filter(|&i| alts[i].sig.tparams.len() == explicit.len()).collect();
            if !fitting.is_empty() {
                cands = fitting;
            }
            // dotc: among several that take the type arguments, those whose bounds they meet.
            if cands.len() > 1 {
                let within: Vec<usize> = cands.iter().copied().filter(|&i| self.within_bounds(&alts[i], &explicit)).collect();
                if !within.is_empty() {
                    cands = within;
                }
            }
        }
        infos.clear();
        infos.resize_with(lists.len(), || None);
        let explicit_lists: Vec<usize> = (0..lists.len()).filter(|&i| !lists[i].using).collect();
        if explicit_lists.is_empty() {
            let compatible = self.narrow_by_expected(alts, &cands, &explicit, expected);
            // dotc's `tryParameterless`: where no alternative or no single one fits, a reference
            // without arguments means the one that takes none, or else one declared with `()`.
            return self.pick(alts, compatible, &explicit).or_else(|failure| {
                let parameterless: Vec<usize> =
                    (0..alts.len()).filter(|&i| alts[i].sig.clauses.is_empty() && alts[i].sig.tparams.is_empty()).collect();
                let empty_parens = (0..alts.len()).find(|&i| matches!(alts[i].sig.clauses.as_slice(), [c] if !c.is_using && c.params.is_empty()));
                match (parameterless.as_slice(), empty_parens) {
                    ([one], _) => Ok(*one),
                    (_, Some(i)) => Ok(i),
                    _ => Err(failure),
                }
            });
        }
        let mut applicable = cands;
        for (k, &li) in explicit_lists.iter().enumerate() {
            let shapes = self.arg_shapes(&lists[li]);
            // A function literal of several parameters stands for one that takes a tuple only
            // where no alternative takes a function of that many. dotc's first pass leaves out
            // implicits (`resolveOverloaded`, Applications.scala 2474) and with them the SAM
            // conversion of a function literal to a partial function (`argOK` 1053), which a
            // `{ case ... }` literal does not need: another literal fits a `PartialFunction`
            // only in the second pass, where the first found no alternative, so that
            // `foo(x => x match { .. })` beside `foo(f: Int => Int)` is the total function.
            let cands = std::mem::take(&mut applicable);
            let mut fitting: Vec<usize> = Vec::new();
            let (mut partial, mut untupled) = (false, false);
            'passes: for p in [false, true] {
                for u in [false, true] {
                    fitting = cands.iter().copied().filter(|&i| self.shape_fits(&alts[i], k, &shapes, u, p)).collect();
                    if !fitting.is_empty() {
                        (partial, untupled) = (p, u);
                        break 'passes;
                    }
                }
            }
            if fitting.len() <= 1 && !(strict && fitting.len() == 1) {
                let kept = match (fitting.first(), partial) {
                    (Some(_), false) => self.kept_from_partial(alts, &cands, &fitting, k, &shapes),
                    _ => Vec::new(),
                };
                infos[li] = Some(ListInfo { types: vec![None; shapes.len()], lambdas: vec![None; shapes.len()], shapes, function_names: Vec::new() });
                return match fitting.first() {
                    Some(&i) if kept.is_empty() => Ok(i),
                    Some(&i) => {
                        let mut pool = kept;
                        pool.push(i);
                        Ok(self.adapt_by_result(alts, i, &pool, &explicit, explicit_lists.len(), expected))
                    }
                    None if k == 0 => self.tupled_alternative(alts, &cands, &lists[li], &explicit),
                    None => Err(Failure::NoneApplicable),
                };
            }
            infos[li] = Some(self.pretype_args(alts, &fitting, k, &mut lists[li], shapes, open));
            if self.profile.on {
                self.profile.fitting = fitting.len() as u32;
            }
            let upto: Vec<&ListInfo> = explicit_lists[..=k].iter().map(|&l| infos[l].as_ref().unwrap()).collect();
            let filter_applicable = |t: &mut Self, cands: &[usize]| -> Vec<usize> {
                cands
                    .iter()
                    .copied()
                    .filter(|&i| {
                        let mark = t.snapshot();
                        let ok = t.is_applicable(&alts[i], &explicit, &upto);
                        t.rollback(mark);
                        ok
                    })
                    .collect()
            };
            applicable = filter_applicable(self, &fitting);
            // As dotc, a second round with implicit conversions where no alternative applies,
            // and with the partial functions the first pass kept a function literal from.
            let widened = applicable.is_empty() && !partial && {
                let shapes = &upto[k].shapes;
                let wider: Vec<usize> = cands.iter().copied().filter(|&i| self.shape_fits(&alts[i], k, shapes, untupled, true)).collect();
                let more = wider.len() > fitting.len();
                fitting = wider;
                more
            };
            if applicable.is_empty() && (!self.view_compat || widened) {
                let prof = self.prof(Kind::Retry, lists[li].span, About::Sym(alts[fitting[0]].sym));
                let outer = std::mem::replace(&mut self.view_compat, true);
                applicable = filter_applicable(self, &fitting);
                self.view_compat = outer;
                if let Some(p) = prof {
                    self.profile.exit(p, if applicable.is_empty() { Outcome::NotFound } else { Outcome::Found });
                }
            }
            if self.profile.on {
                self.profile.applicable = applicable.len() as u32;
            }
            if applicable.is_empty() {
                return Err(Failure::NoneApplicable);
            }
            let best = self.narrow_most_specific(alts, applicable.clone(), &explicit, Some(k));
            if best.len() == 1 {
                let mut pool = applicable.clone();
                if !partial && !widened {
                    let kept = self.kept_from_partial(alts, &cands, &fitting, k, &upto[k].shapes);
                    pool.extend(kept);
                }
                return Ok(self.adapt_by_result(alts, best[0], &pool, &explicit, explicit_lists.len(), expected));
            }
            if k + 1 == explicit_lists.len() {
                return self.break_tie(alts, best, &applicable, explicit_lists.len());
            }
        }
        unreachable!()
    }

    fn pick(&mut self, alts: &[Alt], compatible: Vec<usize>, explicit: &[TypeId]) -> Result<usize, Failure> {
        if compatible.is_empty() {
            return Err(Failure::NoneApplicable);
        }
        let best = self.narrow_most_specific(alts, compatible, explicit, None);
        match best.as_slice() {
            [one] => Ok(*one),
            _ => Err(Failure::Ambiguous(best)),
        }
    }

    /// What dotc falls back on between alternatives of which none is most specific: those
    /// whose result takes no further arguments, then those that declare no default arguments.
    fn break_tie(&mut self, alts: &[Alt], best: Vec<usize>, applicable: &[usize], applied: usize) -> Result<usize, Failure> {
        let uncurried: Vec<usize> =
            applicable.iter().copied().filter(|&i| alts[i].explicit_clauses().count() <= applied).collect();
        if let [one] = uncurried.as_slice() {
            return Ok(*one);
        }
        let no_defaults: Vec<usize> = applicable.iter().copied().filter(|&i| !alts[i].has_defaults()).collect();
        if let [one] = no_defaults.as_slice() {
            return Ok(*one);
        }
        Err(Failure::Ambiguous(best))
    }

    /// Several arguments for the one parameter of an alternative are passed as a tuple, when
    /// no alternative takes them as they are; of several alternatives that take one argument
    /// the most specific is chosen, as dotc resolves the overload again with the arguments
    /// tupled (`+? ("page", 1)` of http4s's `+?[K, T](param: (K, T))` beside `+?[K](name: K)`).
    fn tupled_alternative(&mut self, alts: &[Alt], cands: &[usize], list: &ArgList, explicit: &[TypeId]) -> Result<usize, Failure> {
        let positional = list.args.len() > 1
            && self.arg_shapes(list).iter().all(|a| a.name.is_none() && a.shape != Shape::Splice);
        let n = list.args.len();
        let single: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&i| {
                let Some(p) = alts[i].clause(0).filter(|cl| cl.params.len() == 1 && !cl.params[0].repeated) else {
                    return false;
                };
                let mark = self.snapshot();
                let subst = self.alt_subst(&alts[i], &[]);
                let pty = self.types.subst(p.params[0].ty, &subst);
                let holds = self.holds_tuple(pty, n);
                self.rollback(mark);
                holds
            })
            .collect();
        if !positional || single.is_empty() {
            return Err(Failure::NoneApplicable);
        }
        match self.narrow_most_specific(alts, single, explicit, Some(0)).as_slice() {
            [one] => Ok(*one),
            best => Err(Failure::Ambiguous(best.to_vec())),
        }
    }

    /// A reference without arguments: the alternatives whose type as a value fits what is
    /// expected, a method counting as the function it expands to.
    fn narrow_by_expected(&mut self, alts: &[Alt], cands: &[usize], explicit: &[TypeId], expected: Option<TypeId>) -> Vec<usize> {
        let Some(expected) = expected else { return cands.to_vec() };
        cands
            .iter()
            .copied()
            .filter(|&i| {
                // An open variable stands for the function type among its bounds, if any.
                let arity = alts[i].clause(0).map_or(0, |cl| cl.params.len());
                let exp = self.function_bound(expected, arity);
                let exp = self.deref(exp);
                if matches!(self.types.get(exp), Type::Var(_)) || exp == ANY || exp == self.b.t_unit {
                    return true;
                }
                let mark = self.snapshot();
                let subst = self.alt_subst(&alts[i], explicit);
                let value = self.value_type(&alts[i], &subst, 0);
                let ok = self.is_sub(value, exp);
                self.rollback(mark);
                ok
            })
            .collect()
    }

    /// The type an alternative has as a value once `applied` of its argument lists are given.
    fn value_type(&mut self, alt: &Alt, subst: &Subst, applied: usize) -> TypeId {
        let clauses: Vec<&ClauseSig> = alt.explicit_clauses().skip(applied).collect();
        let mut ty = self.types.subst(alt.sig.ret, subst);
        for clause in clauses.into_iter().rev() {
            let params: Vec<TypeId> = clause
                .params
                .iter()
                .map(|p| {
                    let t = self.types.subst(p.ty, subst);
                    match (p.repeated, self.seq_class()) {
                        (true, Some(seq)) => self.types.class(seq, &[t]),
                        _ => t,
                    }
                })
                .collect();
            ty = self.fun_type(&params, ty);
        }
        ty
    }

    fn within_bounds(&mut self, alt: &Alt, explicit: &[TypeId]) -> bool {
        let mut subst = alt.owner_subst.clone();
        subst.extend(alt.sig.tparams.iter().copied().zip(explicit.iter().copied()));
        alt.sig.tparams.iter().zip(explicit).all(|(&tp, &arg)| {
            let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
            let mark = self.snapshot();
            let upper = self.types.subst(upper, &subst);
            let lower = self.types.subst(lower, &subst);
            let ok = self.is_sub(arg, upper) && self.is_sub(lower, arg);
            self.rollback(mark);
            ok
        })
    }

    /// The type parameters of an alternative as fresh variables within their bounds, or as the
    /// type arguments that were written.
    fn alt_subst(&mut self, alt: &Alt, explicit: &[TypeId]) -> Subst {
        let mut subst = alt.owner_subst.clone();
        for (i, &tp) in alt.sig.tparams.iter().enumerate() {
            let t = match explicit.get(i) {
                Some(&t) => t,
                None => self.fresh_var(),
            };
            subst.push((tp, t));
        }
        for &tp in &alt.sig.tparams {
            let (upper, lower) = (self.syms.tparam(tp).upper, self.syms.tparam(tp).lower);
            if upper == ANY && lower == NOTHING {
                continue;
            }
            let var = self.types.param(tp);
            let var = self.types.subst(var, &subst);
            if upper != ANY {
                let u = self.types.subst(upper, &subst);
                self.is_sub(var, u);
            }
            if lower != NOTHING {
                let l = self.types.subst(lower, &subst);
                self.is_sub(l, var);
            }
        }
        subst
    }

    fn arg_shapes(&self, list: &ArgList) -> Vec<ArgShape> {
        list.args.iter().map(|a| self.arg_shape(a)).collect()
    }

    fn arg_shape(&self, arg: &ArgSrc) -> ArgShape {
        let ast = self.cur_ast();
        let e = match arg {
            ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => *e,
            ArgSrc::Named(name, ..) => return ArgShape { name: Some(*name), shape: Shape::Plain },
            ArgSrc::Typed(..) => return ArgShape { name: None, shape: Shape::Plain },
            ArgSrc::ForLambda(..) => return ArgShape { name: None, shape: Shape::Lambda(1, false) },
        };
        let (name, mut value) = match ast.expr(e) {
            Expr::NamedArg(n, v) => (Some(n), v),
            _ => (None, e),
        };
        loop {
            match ast.expr(value) {
                Expr::Parens(inner) => value = inner,
                Expr::Block(stmts) => match ast.stmt_list(stmts) {
                    [Stmt::Expr(inner)] => value = *inner,
                    _ => break,
                },
                _ => break,
            }
        }
        let shape = match ast.expr(value) {
            Expr::Lambda(params, _) => {
                let lps = &ast.lambda_params[params.range()];
                // `() => e` has no parameter types to tell the alternatives apart by: its shape
                // does, a trait taking it through SAM conversion included.
                if lps.is_empty() || lps.iter().any(|p| p.ty.is_none()) {
                    Shape::Lambda(lps.len(), lps.len() == 1 && lps[0].name == names::CASE_PARAM)
                } else {
                    Shape::Plain
                }
            }
            Expr::Typed(_, t) if matches!(ast.ty(t), TyExpr::Repeated(_)) => Shape::Splice,
            _ => Shape::Plain,
        };
        ArgShape { name, shape }
    }

    /// The parameter each argument of a list goes to, when the list fits the clause: by
    /// position or by name, a repeated parameter taking the rest, defaults filling what is left.
    fn map_args(clause: &ClauseSig, args: &[ArgShape]) -> Option<Mapping> {
        let n = clause.params.len();
        let plain = args.len() == n
            && args.iter().all(|a| a.name.is_none() && a.shape != Shape::Splice)
            && !clause.params.last().map_or(false, |p| p.repeated);
        if plain {
            return Some(Mapping::InOrder);
        }
        let mut filled = vec![false; n];
        let mut spliced = vec![false; n];
        let mut out = Vec::with_capacity(args.len());
        let mut pos = 0usize;
        for a in args {
            let i = match a.name {
                Some(name) => {
                    let i = clause.params.iter().position(|p| p.name == name)?;
                    if filled[i] {
                        return None;
                    }
                    pos = pos.max(i + 1);
                    i
                }
                None => {
                    let i = if pos < n {
                        pos
                    } else if n > 0 && clause.params[n - 1].repeated {
                        n - 1
                    } else {
                        return None;
                    };
                    if filled[..i].iter().any(|f| !f) {
                        return None;
                    }
                    if !clause.params[i].repeated {
                        pos += 1;
                    }
                    i
                }
            };
            // A spread is the whole of a repeated parameter's arguments, as scalac has it.
            if a.shape == Shape::Splice && (!clause.params[i].repeated || filled[i]) {
                return None;
            }
            if filled[i] && spliced[i] {
                return None;
            }
            filled[i] = true;
            spliced[i] = a.shape == Shape::Splice;
            out.push(i);
        }
        let complete = clause.params.iter().zip(&filled).all(|(p, &f)| f || p.has_default || p.repeated);
        complete.then_some(Mapping::To(out))
    }

    /// The alternatives of `cands` beside `fitting` that the shapes `args` fit once a function
    /// literal is admitted to a partial function and tupled, whichever pass took it: those dotc's
    /// first pass kept the literal from, which its `adaptByResult` still reads (Applications.scala
    /// 2448), as it reads every alternative by its result alone. `val s: String = choose(x => x + 1)`
    /// takes the partial function's alternative where the function's result is an `Int`, and
    /// `val r: Int = f((x, y) => x + y)` the partial function of a pair beside
    /// `f(g: (Int, Int) => Int): String`.
    fn kept_from_partial(&mut self, alts: &[Alt], cands: &[usize], fitting: &[usize], k: usize, args: &[ArgShape]) -> Vec<usize> {
        if !args.iter().any(|a| matches!(a.shape, Shape::Lambda(_, false))) {
            return Vec::new();
        }
        cands.iter().copied().filter(|&i| !fitting.contains(&i) && self.shape_fits(&alts[i], k, args, true, true)).collect()
    }

    /// Whether the shapes of the arguments `args` fit the clause `k` of `alt`: `partial` admits
    /// a function literal that is no `{ case ... }` to a partial function (`takes_function`).
    fn shape_fits(&mut self, alt: &Alt, k: usize, args: &[ArgShape], untupled: bool, partial: bool) -> bool {
        let Some(clause) = alt.clause(k) else {
            // `x.toString()` next to `toString(radix)`: the empty parentheses of Java.
            if k == 0 && args.is_empty() && self.has_java_parens(alt.sym) {
                return true;
            }
            // A result that is a function takes the arguments through its `apply`, as does one
            // with an `apply` member, which the application resolves in its turn.
            return k == 0
                && match self.as_function(alt.sig.ret) {
                    Some((ps, _)) => ps.len() == args.len(),
                    None => self.find_member(alt.sig.ret, names::APPLY).is_some(),
                };
        };
        let Some(mapping) = Self::map_args(clause, args) else { return false };
        args.iter().enumerate().all(|(j, a)| match a.shape {
            Shape::Lambda(arity, any_arity) => self.takes_function(clause.params[mapping.param(j)].ty, arity, any_arity, untupled, partial),
            _ => true,
        })
    }

    /// Whether a function literal of `arity` parameters can stand where `pty` is expected; one
    /// that is no `{ case ... }` stands for a partial function only where `partial` is set.
    fn takes_function(&mut self, pty: TypeId, arity: usize, any_arity: bool, untupled: bool, partial: bool) -> bool {
        // An alias kept by name (zio's `URIO[R, A]`) takes a literal where what it stands for does.
        let t = self.deref(pty);
        let t = if matches!(self.types.get(t), Type::Alias(..)) { self.dealias(t) } else { t };
        let Type::Class(c, args) = self.types.get(t) else { return !matches!(self.types.get(t), Type::Lit(_)) };
        if self.is_function_class(c) {
            let n = self.types.items(args).len() - 1;
            return any_arity || n == arity || (untupled && n == 1 && arity > 1);
        }
        // A lambda of one parameter, or of the elements of a tuple, is a partial function
        // defined everywhere, or where the cases of a match that is its body apply (dotc's
        // `ExpandSAMs`).
        if self.is_partial_function(c) {
            return any_arity || (partial && (arity == 1 || (untupled && arity > 1)));
        }
        // A context function takes a function literal, which is wrapped, when its result does.
        if self.is_context_function_class(c) {
            return match self.as_context_function(t) {
                Some((_, result)) => self.takes_function(result, arity, any_arity, untupled, partial),
                None => true,
            };
        }
        // A trait or an abstract class takes a function literal through SAM conversion alone,
        // so one with several abstract methods (tapir's `Mapping`) rules the alternative out,
        // as dotc's shape pass does.
        let sam = |t: &mut Self, n: usize| t.class_is_sam(c, n);
        let takes = if any_arity {
            (0..=3).any(|n| sam(self, n))
        } else {
            sam(self, arity) || (untupled && arity > 1 && sam(self, 1))
        };
        // A class the function class derives from (`AnyRef`) takes the literal as a function.
        takes || (!any_arity && c == self.b.any_ref) || (!any_arity && {
            let f = self.function_class(arity);
            self.complete_class(f);
            self.syms.class(f).base_types.iter().any(|&(b, _)| b == c)
        })
    }

    /// Whether class `c` is a SAM type of `arity` parameters, remembered per class: the shape
    /// pass asks for every function literal against a trait parameter.
    fn class_is_sam(&mut self, c: ClassId, arity: usize) -> bool {
        if let Some(&known) = self.sam_arity.get(&(c, arity as u8)) {
            return known;
        }
        let t = self.types.class(c, &[]);
        let known = self.sam_method(t, arity).is_some();
        self.sam_arity.insert((c, arity as u8), known);
        known
    }

    /// Types the arguments of a list that can be typed before an alternative is chosen, each
    /// once: against the parameter type where the candidates agree on it, on its own otherwise.
    /// A function literal is left to its shape unless the candidates agree on the types of its
    /// parameters: typed with those, its result tells the candidates apart.
    fn pretype_args(&mut self, alts: &[Alt], cands: &[usize], k: usize, list: &mut ArgList, shapes: Vec<ArgShape>, open: bool) -> ListInfo {
        let mappings: Vec<Option<Mapping>> =
            cands.iter().map(|&i| alts[i].clause(k).and_then(|cl| Self::map_args(cl, &shapes))).collect();
        let mut types = vec![None; shapes.len()];
        let mut lambdas = vec![None; shapes.len()];
        let mut function_names = Vec::new();
        for j in 0..shapes.len() {
            // A spread's sequence is typed on its own: the alternatives are told apart by its
            // elements, and the application types it again against the one chosen.
            if shapes[j].shape == Shape::Splice {
                if let Some(ArgSrc::Ast(e) | ArgSrc::Hoisted(e)) = list.args.get(j) {
                    if let Expr::Typed(inner, _) = self.cur_ast().expr(*e) {
                        // Typed for its type alone: what the typing wrote goes.
                        let mark = self.attempt();
                        let (_, ty) = self.type_expr(inner, None);
                        self.retract(mark);
                        types[j] = Some(ty);
                    }
                }
                continue;
            }
            let is_lambda = matches!(shapes[j].shape, Shape::Lambda(..));
            let is_cases = matches!(shapes[j].shape, Shape::Lambda(_, true));
            let mut common: Option<(TypeId, Vec<TypeId>)> = None;
            // A `{ case ... }` literal is typed as a partial function when a candidate takes one
            // (dotty's `pretypeArgs`, `Applications.scala` 2862 to 2869), and the typed literal
            // goes to whichever candidate is chosen, a partial function being a function.
            let mut any_partial = false;
            for (ci, &i) in cands.iter().enumerate() {
                let alt = &alts[i];
                let agreed = (|| {
                    let mapping = mappings[ci].as_ref()?;
                    let p = &alt.clause(k)?.params[mapping.param(j)];
                    let pty = self.types.subst(p.ty, &alt.owner_subst);
                    let (key, params) = if is_lambda {
                        let params = match self.as_partial_function(pty) {
                            Some((a, _)) => {
                                any_partial = true;
                                vec![a]
                            }
                            None => self.as_function(pty)?.0,
                        };
                        (self.tuple_or_unit(&params), params)
                    } else {
                        (pty, Vec::new())
                    };
                    if self.mentions_type_params(key, &alt.sig.tparams) {
                        return None;
                    }
                    match &common {
                        Some((t, _)) if *t != key => None,
                        _ => Some((key, params)),
                    }
                })();
                common = agreed;
                if common.is_none() {
                    break;
                }
            }
            let e = match list.args[j] {
                ArgSrc::Ast(e) | ArgSrc::Hoisted(e) => e,
                ArgSrc::Typed(te, ty) | ArgSrc::Named(_, te, ty) => {
                    types[j] = Some(self.proxy_declared(te).unwrap_or(ty));
                    continue;
                }
                // A generator's function is a function literal dotty's `Desugar.makeFor` wrote,
                // typed ahead as a written one is (`pretypeArgs`): its result tells `map`'s
                // alternatives apart (`for (k, v) <- m yield k + v` an `Iterable` of a `Map`).
                ArgSrc::ForLambda(binder, rest) => {
                    if let Some((_, params)) = common {
                        let result = self.fresh_var();
                        let expected = self.fun_type(&params, result);
                        let (te, ty) = self.type_for_lambda(binder, rest, expected, list.span);
                        if let Some((_, r)) = self.as_function(ty) {
                            lambdas[j] = Some((te, ty, r));
                        }
                    }
                    continue;
                }
            };
            let value = match self.cur_ast().expr(e) {
                Expr::NamedArg(_, v) => v,
                _ => e,
            };
            // The names the argument's value comes out of and the candidates that take a function
            // of it, which a name alone gives them (`ListInfo::function_names`).
            let mut names = Vec::new();
            let mut takers = Vec::new();
            if self.index.is_some() {
                super::index::result_names(self.cur_ast(), value, &mut names);
                if !names.is_empty() {
                    for (ci, &i) in cands.iter().enumerate() {
                        let Some(p) = mappings[ci].as_ref().and_then(|m| Some(&alts[i].clause(k)?.params[m.param(j)])) else { continue };
                        let pty = self.types.subst(p.ty, &alts[i].owner_subst);
                        if self.expects_function(pty, 4) {
                            takers.push(i);
                        }
                    }
                }
            }
            if is_lambda {
                let Some((_, params)) = common else { continue };
                let result = self.fresh_var();
                let partial = is_cases && any_partial;
                let partial_function = if partial { self.partial_function_class() } else { None };
                let expected = match (partial, partial_function, params.as_slice()) {
                    (true, Some(pf), [a]) => self.types.class(pf, &[*a, result]),
                    _ => self.fun_type(&params, result),
                };
                let prof = self.prof(Kind::Retry, self.cur_ast().expr_span(value), About::None);
                let (te, ty) = self.type_expr(value, Some(expected));
                if let Some(p) = prof {
                    self.profile.typed_ahead(p, te);
                }
                let result = match self.as_partial_function(ty) {
                    Some((_, r)) => Some(r),
                    None => self.as_function(ty).map(|(_, r)| r),
                };
                if let Some(r) = result {
                    lambdas[j] = Some((te, ty, r));
                }
                continue;
            }
            // Against an open variable an argument keeps the type arguments open that nothing
            // in it settles, for the parameter of the chosen alternative to settle; that takes an
            // application which solves them afterwards.
            let expected = match common {
                Some((t, _)) => Some(t),
                None if open => Some(self.fresh_var()),
                None => None,
            };
            let (te, ty) = self.type_expr(value, expected);
            if !takers.is_empty() {
                function_names.extend(names.iter().map(|&n| (n, te, takers.clone())));
            }
            let ty = match self.enum_case_new {
                Some((new, precise)) if new == te => precise,
                _ => ty,
            };
            let te = match list.args[j] {
                ArgSrc::Hoisted(_) => {
                    let span = self.cur_ast().expr_span(value);
                    self.hoist(te, ty, span)
                }
                _ => te,
            };
            list.args[j] = match shapes[j].name {
                Some(name) => ArgSrc::Named(name, te, ty),
                None => ArgSrc::Typed(te, ty),
            };
            // The alternatives are ranked by a parameter of the body under expansion at its
            // declared type, as scalac chose among them at the definition.
            types[j] = Some(self.proxy_declared(te).unwrap_or(ty));
        }
        ListInfo { shapes, types, lambdas, function_names }
    }

    /// A function literal that was typed ahead goes to the chosen alternative as it is, as
    /// dotty's cache passes it (`ProtoTypes` 556 to 572): a typed partial function to a partial
    /// function or a function, a typed function to a function. Only a function literal that is no
    /// `{ case ... }` and meets a partial function is typed again, what its first typing reported
    /// not reported twice and its matches checked as the second typing has them
    /// (`quiet_repeated_matches`; a trait taking it is never typed ahead, `as_function`).
    fn pass_typed_lambdas(&mut self, alt: &Alt, lists: &mut [ArgList], infos: &[Option<ListInfo>]) -> bool {
        let mut retyped = false;
        let explicit_lists: Vec<usize> = (0..lists.len()).filter(|&i| !lists[i].using).collect();
        for (k, &li) in explicit_lists.iter().enumerate() {
            let Some(info) = infos.get(li).and_then(|i| i.as_ref()) else { continue };
            let Some(clause) = alt.clause(k) else { continue };
            let Some(mapping) = Self::map_args(clause, &info.shapes) else { continue };
            for (j, lambda) in info.lambdas.iter().enumerate() {
                let Some((te, ty, _)) = *lambda else { continue };
                let pty = clause.params[mapping.param(j)].ty;
                let takes_partial = self.as_partial_function(pty).is_some();
                let typed_partial = self.as_partial_function(ty).is_some();
                // A named one that is no partial function does not meet one: dotty's cache holds
                // the `NamedArg` around the closure, which `Typer.adaptToSubType` does not convert,
                // a bare closure alone (`blockEndingInClosure`), so the argument is a mismatch,
                // reported where the named argument is written.
                if let (true, false, Some(name), ArgSrc::Ast(e) | ArgSrc::Hoisted(e)) = (takes_partial, typed_partial, info.shapes[j].name, lists[li].args[j]) {
                    let span = self.cur_ast().expr_span(e);
                    let formal = self.types.subst(pty, &alt.owner_subst);
                    self.adapt(te, ty, formal, span);
                    lists[li].args[j] = ArgSrc::Named(name, te, ERROR);
                    continue;
                }
                if (takes_partial && typed_partial) || (!takes_partial && self.as_function(pty).is_some()) {
                    lists[li].args[j] = match info.shapes[j].name {
                        Some(name) => ArgSrc::Named(name, te, ty),
                        None => ArgSrc::Typed(te, ty),
                    };
                } else {
                    retyped = true;
                    if self.profile.on {
                        self.profile.retried(te);
                    }
                }
            }
        }
        retyped
    }

    fn mentions_type_params(&mut self, t: TypeId, tparams: &[TParamId]) -> bool {
        if tparams.is_empty() {
            return false;
        }
        let blank: Subst = tparams.iter().map(|&p| (p, ERROR)).collect();
        self.types.subst(t, &blank) != t
    }

    /// One type that stands for a parameter list, to compare lists by.
    fn tuple_or_unit(&mut self, ps: &[TypeId]) -> TypeId {
        match ps {
            [] => self.b.t_unit,
            [one] => *one,
            _ => self.tuple_type(ps),
        }
    }

    /// Whether the typed arguments of the lists given so far are compatible with the
    /// parameters of `alt`. Leaves its constraints for the caller to roll back.
    fn is_applicable(&mut self, alt: &Alt, explicit: &[TypeId], lists: &[&ListInfo]) -> bool {
        let subst = self.alt_subst(alt, explicit);
        for (k, info) in lists.iter().enumerate() {
            let Some(clause) = alt.clause(k) else {
                // `x.toString()` next to `toString(radix)`: the empty parentheses of Java.
                if k == 0 && info.shapes.is_empty() && self.has_java_parens(alt.sym) {
                    return true;
                }
                return k == 0 && self.function_result_takes(alt, &subst, info);
            };
            let Some(mapping) = Self::map_args(clause, &info.shapes) else { return false };
            for j in 0..info.shapes.len() {
                let declared = clause.params[mapping.param(j)].ty;
                // A spread's sequence has to be one of the repeated parameter's elements, which
                // are not widened one by one.
                if info.shapes[j].shape == Shape::Splice {
                    let Some(ty) = info.types[j] else { continue };
                    let elem = self.types.subst(declared, &subst);
                    let Some(seq) = self.seq_class() else { continue };
                    let wanted = self.types.class(seq, &[elem]);
                    let mark = self.snapshot();
                    let fits = self.is_sub(ty, wanted);
                    if !fits {
                        self.rollback(mark);
                        return false;
                    }
                    continue;
                }
                if !self.view_compat && info.types[j].map_or(false, |ty| self.class_rules_out(ty, declared)) {
                    return false;
                }
                let pty = self.types.subst(declared, &subst);
                if let Some((_, _, result)) = info.lambdas[j] {
                    let wanted = match self.as_partial_function(pty) {
                        Some((_, r)) => Some(r),
                        None => self.as_function(pty).map(|(_, r)| r),
                    };
                    if wanted.map_or(false, |r| !self.is_compatible(result, r)) {
                        return false;
                    }
                }
                let Some(ty) = info.types[j] else { continue };
                if !self.is_compatible(ty, pty) {
                    return false;
                }
            }
        }
        true
    }

    fn function_result_takes(&mut self, alt: &Alt, subst: &Subst, info: &ListInfo) -> bool {
        let ret = self.types.subst(alt.sig.ret, subst);
        let Some((ps, _)) = self.as_function(ret) else { return self.find_member(ret, names::APPLY).is_some() };
        ps.len() == info.types.len()
            && ps.iter().zip(&info.types).all(|(&p, t)| t.map_or(true, |t| self.is_compatible(t, p)))
    }

    /// An argument whose type is a class cannot go to a parameter whose type is a class it
    /// does not derive from, numbers aside, which widen. This settles most alternatives
    /// without a subtype test.
    fn class_rules_out(&mut self, ty: TypeId, declared: TypeId) -> bool {
        let Type::Class(wanted, _) = self.types.get(declared) else { return false };
        let ty = self.deref(ty);
        let Type::Class(actual, _) = self.types.get(ty) else { return false };
        if actual == wanted {
            return false;
        }
        let Some(info) = self.syms.class_done(actual) else { return false };
        let settled = |k: ClassKind| !matches!(k, ClassKind::Opaque | ClassKind::Builtin);
        settled(info.kind)
            && settled(self.syms.class(wanted).kind)
            && !info.base_types.iter().any(|&(b, _)| b == wanted)
    }

    /// An argument of type `ty` can be passed for a parameter of type `pty`: it conforms, or
    /// it is a number that widens to it.
    pub(super) fn is_compatible(&mut self, ty: TypeId, pty: TypeId) -> bool {
        let ty = self.capture_wildcards(ty);
        let mark = self.snapshot();
        if self.is_sub(ty, pty) {
            return true;
        }
        self.rollback(mark);
        // An argument where a context function is expected is wrapped in one: it fits when it
        // fits the result (`(s: S) => s.x` for a `Ctx ?=> S => A`).
        if let Some((_, result)) = self.as_context_function(pty) {
            if self.as_context_function(ty).is_none() && self.is_compatible(ty, result) {
                return true;
            }
        }
        // dotc's `SAMArgOK`: a function type fits a trait with a single abstract method whose
        // function type it conforms to, so that an alternative taking `String => String` is
        // as specific as one taking `java.util.function.Function[String, String]`.
        if let Some((params, _ret)) = self.as_function(ty) {
            if let Some((_, _, sig, subst)) = self.sam_method(pty, params.len()) {
                let sam_params: Vec<TypeId> = sig.clauses[0].params.iter().map(|p| self.types.subst(p.ty, &subst)).collect();
                let sam_ret = self.types.subst(sig.ret, &subst);
                let sam_fn = self.fun_type(&sam_params, sam_ret);
                let mark = self.snapshot();
                if self.is_sub(ty, sam_fn) {
                    return true;
                }
                self.rollback(mark);
            }
        }
        let from = self.deref(ty);
        let from = self.widen_lit(from);
        let to = self.deref(pty);
        let to = match self.types.get(to) {
            Type::Union(..) => self.numeric_member(to).unwrap_or(to),
            _ => to,
        };
        if self.numeric_widening(from, to) {
            return true;
        }
        self.view_compat && self.view_exists(ty, pty, Span::default())
    }

    // ---- specificity ----

    /// dotc's `narrowMostSpecific`: the alternatives that no other one beats.
    fn narrow_most_specific(&mut self, alts: &[Alt], cands: Vec<usize>, explicit: &[TypeId], k: Option<usize>) -> Vec<usize> {
        if cands.len() <= 1 {
            return cands;
        }
        let mut survivors: Vec<usize> = vec![cands[0]];
        for &alt in &cands[1..] {
            match self.compare_alts(alts, survivors[0], alt, explicit, k) {
                1 => {}
                -1 => {
                    let mut kept = vec![alt];
                    for &s in &survivors {
                        if self.compare_alts(alts, s, alt, explicit, k) != -1 {
                            kept.push(s);
                        }
                    }
                    survivors = kept;
                }
                _ => survivors.push(alt),
            }
        }
        let best = survivors[0];
        let mut out = vec![best];
        for &s in &survivors[1..] {
            if self.compare_alts(alts, s, best, explicit, k) >= 0 {
                out.push(s);
            }
        }
        out
    }

    /// 1 when `a` is preferred over `b`, -1 for the reverse, 0 for a draw: one point for being
    /// defined in a class that derives from the other's, one for being as specific.
    fn compare_alts(&mut self, alts: &[Alt], a: usize, b: usize, explicit: &[TypeId], k: Option<usize>) -> i32 {
        let (x, y) = (&alts[a], &alts[b]);
        let owner_score = match (self.syms.sym(x.sym).owner, self.syms.sym(y.sym).owner) {
            (Owner::Class(cx), Owner::Class(cy)) if cx != cy => {
                let derives = |t: &Self, sub: ClassId, sup: ClassId| t.syms.class(sub).base_types.iter().any(|&(c, _)| c == sup);
                if derives(self, cx, cy) {
                    1
                } else if derives(self, cy, cx) {
                    -1
                } else {
                    0
                }
            }
            _ => 0,
        };
        let wins_x = self.is_as_specific(x, y, explicit, k);
        let wins_y = self.is_as_specific(y, x, explicit, k);
        let by_type = match owner_score {
            1 if wins_x || !wins_y => 1,
            -1 if wins_y || !wins_x => -1,
            0 if wins_x != wins_y => {
                if wins_x {
                    1
                } else {
                    -1
                }
            }
            _ => 0,
        };
        if by_type != 0 {
            return by_type;
        }
        // An alternative that starts with a using clause loses to one that does not.
        let leading_using = |alt: &Alt| alt.sig.clauses.first().map_or(false, |c| c.is_using);
        match (leading_using(x), leading_using(y)) {
            (false, true) => 1,
            (true, false) => -1,
            _ => 0,
        }
    }

    /// SLS 6.26.3: a method is as specific as another alternative if that one is applicable to
    /// arguments of the method's parameter types, a vararg method only against another one; a
    /// member that takes no arguments is as specific as any method.
    fn is_as_specific(&mut self, a: &Alt, b: &Alt, explicit: &[TypeId], applied: Option<usize>) -> bool {
        let k = applied.unwrap_or(0);
        let own_subst = self.explicit_subst(a, explicit);
        let Some(clause) = a.clause(k) else {
            // Where arguments are applied, a method that takes them itself comes before a
            // member whose result would take them through its `apply`.
            if b.clause(k).is_some() {
                return applied.is_none();
            }
            let mark = self.snapshot();
            let subst = self.alt_subst(b, explicit);
            let (ra, rb) = (self.types.subst(a.sig.ret, &own_subst), self.types.subst(b.sig.ret, &subst));
            let ok = self.is_compatible(ra, rb);
            self.rollback(mark);
            return ok;
        };
        if clause.params.is_empty() && (b.is_method() || !b.sig.tparams.is_empty()) {
            return true;
        }
        let vararg = clause.params.last().map_or(false, |p| p.repeated);
        let Some(other) = b.clause(k) else { return applied.is_some() };
        if vararg && !other.params.last().map_or(false, |p| p.repeated) {
            return false;
        }
        let shapes = vec![ArgShape { name: None, shape: Shape::Plain }; clause.params.len()];
        let Some(mapping) = Self::map_args(other, &shapes) else { return false };
        let mark = self.snapshot();
        let subst = self.alt_subst(b, explicit);
        let ok = clause.params.iter().enumerate().all(|(j, p)| {
            let ty = self.types.subst(p.ty, &own_subst);
            let pty = self.types.subst(other.params[mapping.param(j)].ty, &subst);
            self.is_compatible(ty, pty)
        });
        self.rollback(mark);
        ok
    }

    /// The parameter types of the alternative under comparison: its owner's arguments and,
    /// with explicit type arguments, those for its own type parameters; without them a type
    /// parameter stays abstract, as SLS 6.26.3 reads a polymorphic method.
    fn explicit_subst(&self, alt: &Alt, explicit: &[TypeId]) -> Subst {
        if explicit.is_empty() || explicit.len() != alt.sig.tparams.len() {
            return alt.owner_subst.clone();
        }
        let mut subst = alt.owner_subst.clone();
        subst.extend(alt.sig.tparams.iter().copied().zip(explicit.iter().copied()));
        subst
    }

    /// dotc's `adaptByResult`: when the result of the chosen alternative does not fit the
    /// expected type and that of another applicable one does, that one is meant.
    fn adapt_by_result(&mut self, alts: &[Alt], chosen: usize, applicable: &[usize], explicit: &[TypeId], applied: usize, expected: Option<TypeId>) -> usize {
        if applicable.len() < 2 {
            return chosen;
        }
        let Some(exp) = self.concrete_expected(expected).filter(|&t| t != ANY && t != self.b.t_unit) else {
            return chosen;
        };
        let conforms = |t: &mut Self, i: usize| {
            let mark = t.snapshot();
            let subst = t.alt_subst(&alts[i], explicit);
            let value = t.value_type(&alts[i], &subst, applied);
            let ok = t.is_compatible(value, exp);
            t.rollback(mark);
            ok
        };
        if conforms(self, chosen) {
            return chosen;
        }
        let others: Vec<usize> = applicable.iter().copied().filter(|&i| i != chosen && conforms(self, i)).collect();
        match self.narrow_most_specific(alts, others, explicit, Some(0)).as_slice() {
            [one] => *one,
            _ => chosen,
        }
    }

    // ---- messages ----

    /// The checks of the matches a function literal's first typing left, where it was typed
    /// again (`pass_typed_lambdas`), report nothing: the later check of the same match is the
    /// literal's as passed, as dotc checks the cases of the tree it keeps (a plain literal typed
    /// ahead as a function and passed to a partial function is one there, `ExpandSAMs`). They
    /// are quietened in place, as the journals of the open attempts count them by position.
    fn quiet_repeated_matches(&mut self, mark: usize) {
        let matches = &mut self.deferred_matches;
        for i in mark..matches.len() {
            let (file, span) = (matches[i].file, matches[i].span);
            if matches[i + 1..].iter().any(|m| m.file == file && m.span == span) {
                matches[i].quiet = true;
            }
        }
    }

    fn drop_repeated_diagnostics(&mut self, mark: usize) {
        let items = &mut self.diags.items;
        let mut i = mark;
        while i < items.len() {
            let repeated = items[mark..i].iter().any(|d| d.file == items[i].file && d.span == items[i].span && d.msg == items[i].msg);
            if repeated {
                items.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// The types the arguments `written` got in a call no alternative took, which signature
    /// help filters the alternatives by while the call is being written.
    #[cold]
    #[inline(never)]
    fn index_failed_args(&mut self, written: &[Vec<ArgSrc>], lists: &[ArgList]) {
        for (w, l) in written.iter().zip(lists) {
            for (a, typed) in w.iter().zip(&l.args) {
                let (Some(mut e), ArgSrc::Typed(te, ty) | ArgSrc::Named(_, te, ty)) = (a.ast(), *typed) else { continue };
                if let Expr::NamedArg(_, v) = self.cur_ast().expr(e) {
                    e = v;
                }
                let span = self.cur_ast().expr_span(e);
                self.index_value(span, te, ty);
            }
        }
    }

    /// Types the arguments of the first list that are still untyped and no function literals,
    /// for the message to name their types.
    fn type_plain_args(&mut self, lists: &mut [ArgList]) {
        let Some(list) = lists.iter_mut().find(|l| !l.using) else { return };
        for j in 0..list.args.len() {
            let shape = self.arg_shape(&list.args[j]);
            let ArgSrc::Ast(e) = list.args[j] else { continue };
            if shape.shape != Shape::Plain {
                continue;
            }
            let value = match self.cur_ast().expr(e) {
                Expr::NamedArg(_, v) => v,
                _ => e,
            };
            let (te, ty) = self.type_expr(value, None);
            list.args[j] = match shape.name {
                Some(name) => ArgSrc::Named(name, te, ty),
                None => ArgSrc::Typed(te, ty),
            };
        }
    }

    /// scalac's E134 and E051.
    fn overload_failure(
        &mut self,
        set: SymId,
        alts: &[Alt],
        failure: &Failure,
        lists: &[ArgList],
        infos: &[Option<ListInfo>],
        expected: Option<TypeId>,
    ) -> String {
        let listed: Vec<usize> = match failure {
            Failure::NoneApplicable => (0..alts.len()).collect(),
            Failure::Ambiguous(among) => among.clone(),
        };
        let mut out = match failure {
            Failure::NoneApplicable => "None of the overloaded alternatives of ".to_string(),
            Failure::Ambiguous(_) => "Ambiguous overload. The overloaded alternatives of ".to_string(),
        };
        let first = alts.first().map_or(set, |a| a.sym);
        out.push_str(&self.method_description_of(first));
        out.push_str(" with types");
        let mut sorted = listed.clone();
        sorted.sort_by_key(|&i| std::cmp::Reverse((alts[i].ctor.is_none(), self.syms.sym(alts[i].sym).span.start)));
        for i in sorted {
            out.push_str("\n ");
            out.push_str(&self.sig_text(&alts[i].sig));
        }
        out.push('\n');
        out.push_str(match failure {
            Failure::NoneApplicable => "match ",
            Failure::Ambiguous(among) if among.len() == 2 => "both match ",
            Failure::Ambiguous(_) => "all match ",
        });
        match lists.iter().position(|l| !l.using) {
            Some(li) => {
                let mut described: Vec<String> = Vec::new();
                for (j, arg) in lists[li].args.iter().enumerate() {
                    let known = infos.get(li).and_then(|i| i.as_ref()).and_then(|i| i.types[j]).or(match arg {
                        ArgSrc::Typed(_, t) | ArgSrc::Named(_, _, t) => Some(*t),
                        _ => None,
                    });
                    described.push(match known {
                        Some(t) => {
                            let t = self.solve_bounded_in(t);
                            self.show(t)
                        }
                        None => "<?>".to_string(),
                    });
                }
                out.push_str(&format!("arguments ({})", described.join(", ")));
            }
            None => {
                let exp = match self.concrete_expected(expected) {
                    Some(t) => self.show(t),
                    None => "Any".to_string(),
                };
                out.push_str(&format!("expected type {}", exp));
            }
        }
        out
    }

    fn method_description_of(&self, sym: SymId) -> String {
        let info = self.syms.sym(sym);
        match info.owner {
            Owner::Class(c) if info.name == names::INIT => {
                format!("constructor {} in {}", self.name_str(self.syms.class(c).name), self.class_description(c))
            }
            Owner::Class(c) => format!("method {} in {}", self.name_str(info.name), self.class_description(c)),
            _ => format!("method {}", self.name_str(info.name)),
        }
    }
}

/// The classes below each class, which `suffix_clashing_roots` asks for every set merged after
/// the final passes: the classes entered since the last question are indexed then, and a class
/// not yet complete, whose base types may still grow, is looked at anew each time.
#[derive(Default)]
pub struct SubclassIndex {
    /// How far the shared region's classes and the worker's own were indexed.
    scanned_shared: usize,
    scanned_own: usize,
    below: FxMap<ClassId, Vec<ClassId>>,
    incomplete: Vec<ClassId>,
}

impl SubclassIndex {
    fn index(&mut self, syms: &Symbols, d: ClassId) {
        for &(b, _) in &syms.class(d).base_types {
            if b != d {
                let ds = self.below.entry(b).or_default();
                if ds.last() != Some(&d) {
                    ds.push(d);
                }
            }
        }
    }

    /// Per class of `owners`, the classes that have it among their base types, in the order of
    /// their ids.
    fn below(&mut self, syms: &Symbols, owners: &[ClassId]) -> FxMap<ClassId, Vec<ClassId>> {
        let complete = |d: ClassId| syms.class(d).state() == Completion::Done;
        // The shared region's classes, then the worker's own: what was added since the last
        // scan of each.
        let shared = syms.classes.shared_len();
        let base = syms.classes.own_base() as usize;
        let own = base + syms.classes.own().len();
        let fresh = (self.scanned_shared..shared).chain(self.scanned_own.max(base)..own);
        for d in fresh {
            let d = ClassId(d as u32);
            if complete(d) {
                self.index(syms, d);
            } else {
                self.incomplete.push(d);
            }
        }
        self.scanned_shared = shared;
        self.scanned_own = own;
        let mut incomplete = std::mem::take(&mut self.incomplete);
        incomplete.retain(|&d| {
            if complete(d) {
                self.index(syms, d);
                false
            } else {
                true
            }
        });
        let mut out: FxMap<ClassId, Vec<ClassId>> = FxMap::default();
        for &rc in owners {
            let mut ds = self.below.get(&rc).cloned().unwrap_or_default();
            ds.extend(incomplete.iter().copied().filter(|&d| d != rc && syms.class(d).base_types.iter().any(|&(b, _)| b == rc)));
            ds.sort_unstable();
            ds.dedup();
            out.insert(rc, ds);
        }
        self.incomplete = incomplete;
        out
    }
}
