//! The dependencies of an analysis build (`--analysis-version 3`; docs/TARGETS.md, "The analysis
//! graph"): per class of the answer's files, what its code depends on as scalac 3.8.4's
//! `ExtractDependencies` sends it to zinc's callback, for sbt-teq's adapter to hand over
//! (`TeqAnalysis.scala`): the names it uses with their scopes (`usedName`), the classes of the
//! build it depends on with the context (`classDependency`), and the classes of the class path
//! with the binary entry that holds each and its binary name (`binaryDependency`).
//!
//! scalac's collector walks the typed tree of a unit after its typer: every reference to a
//! member (`addMemberRefDependency`: the member's name, and a dependency on the class that
//! holds it), every type the tree names or the typer inferred with the types it reaches
//! (`TypeDependencyTraverser`: a named type's symbol, an alias's or a member's info, the
//! prefix), a template's parents and a SAM closure's class (by inheritance), an import's
//! selectors, a wildcard export's class, a match's selector (a sealed class there with the
//! `PatMatTarget` scope); the `Inlining` phase adds what an inline call's expansion refers to.
//! Each record goes to the closest non-local class around the tree, a package-level tree's to
//! the last class of the unit (`responsibleForImports`), and a top-level definition's to its
//! package block's `<file>$package`.
//!
//! teq reads the same from what its typing committed: the signatures, parents and aliases of
//! the symbol table, the bodies the typer kept (a rejected alternative is in none of them), with
//! what the capture recorded where the typer lowered a construct (`tir::capture`: a call's type
//! arguments, an operator's name, a given's call, an inline call behind its expansion), rendered
//! once the build is over, so that a retype's answer is the retyped program's.

use super::Worker;
use crate::ast::ListRef;
use crate::intern::{FxMap, Name};
use crate::source::FileId;
use crate::symbols::*;
use crate::tir::capture::{Form, Wrap};
use crate::tir::*;
use crate::types::*;
use crate::watch::json_string;
use std::collections::{BTreeMap, BTreeSet};

/// The computation a record of the typer's belongs to: what completes a definition's
/// signature, types its body, completes a class (its parents, its type parameters, its
/// constructor), checks a class's body (its initialisers, its constructor's defaults, its
/// parents' arguments, its imports) or completes an alias. A retype replaces what it types
/// again (a body, a class's body, a signature it reopens) and keeps the rest with their records.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Comp {
    Sig(SymId),
    Body(SymId),
    Class(ClassId),
    ClassBody(ClassId),
    Alias(AliasId),
}

/// What the typer resolved that its types do not keep: an alias a written type names (the type
/// holds what it stands for), a package a written type's path goes through.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Ev {
    Alias(AliasId),
    Pkg(PkgId),
    /// A name a written type refers to that teq resolves to no symbol of its own: `&`, `|`,
    /// `AnyRef` (scalac's aliases of the compiler's own).
    Name(Name),
    /// An object or a value an import's path goes through, by its symbol.
    ImportPath(SymId),
    /// An import's selector `n` of what its path reaches: the term and the type named `n`.
    ImportSel(ImportPrefix, Name),
    /// `p.n` of an object `c` that exports `n`: scalac's forwarder of `c`, which the typer
    /// resolves to the original.
    Forwarder(ClassId, Name),
    /// A local class defined in the code typed, whether or not anything instantiates it.
    LocalClass(ClassId),
}

/// What an import's path reaches: a package, an object's class, a stable value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ImportPrefix {
    Pkg(PkgId),
    Class(ClassId),
    Value(SymId),
}

/// What a node of the typed IR stands for where the typer simplified it: an object reached
/// through a library's val that aliases it (`List` of `scala`'s package object), a case
/// class's `apply` the typer wrote as the constructor's call.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Node {
    AliasVal(SymId),
    Apply(ClassId),
    /// A member of the class selected through the name the class exports it by: scalac's
    /// forwarder of that name, which the typer resolves to the original.
    Export(ClassId, Name, TExprId),
    /// A constant member (`final val x = 1`, `inline val`) a reference to which the typer wrote
    /// as its literal.
    Constant(SymId),
}

/// The records of the typer's resolutions, per computation of a non-local definition (the
/// records of a local one's go to the computation it is typed in), and what nodes stand for.
/// A node's record is read where a walk of the kept bodies reaches it, so one of a typing the
/// typer abandoned is never read.
#[derive(Default)]
pub struct Recorder {
    current: Option<Comp>,
    records: FxMap<Comp, Vec<Ev>>,
    nodes: FxMap<TExprId, Node>,
    /// The nodes of each computation, dropped with its records.
    node_comps: FxMap<Comp, Vec<TExprId>>,
}

impl Recorder {
    fn record(&mut self, ev: Ev) {
        if let Some(c) = self.current {
            let list = self.records.entry(c).or_default();
            if !list.contains(&ev) {
                list.push(ev);
            }
        }
    }

    fn node(&mut self, e: TExprId, n: Node) {
        self.nodes.insert(e, n);
        if let Some(c) = self.current {
            self.node_comps.entry(c).or_default().push(e);
        }
    }

    /// Takes over another worker's records at the merge.
    pub fn absorb(&mut self, other: &mut Recorder) {
        for (e, n) in std::mem::take(&mut other.nodes) {
            self.nodes.entry(e).or_insert(n);
        }
        for (c, es) in std::mem::take(&mut other.node_comps) {
            self.node_comps.entry(c).or_default().extend(es);
        }
        for (c, evs) in std::mem::take(&mut other.records) {
            let mine = self.records.entry(c).or_default();
            for ev in evs {
                if !mine.contains(&ev) {
                    mine.push(ev);
                }
            }
        }
    }

    /// Renames what a worker's records name by the merge's maps.
    pub fn remap(
        &mut self,
        sym: impl Fn(SymId) -> SymId,
        class: impl Fn(ClassId) -> ClassId,
        alias: impl Fn(AliasId) -> AliasId,
        pkg: impl Fn(PkgId) -> PkgId,
        expr: impl Fn(TExprId) -> TExprId,
    ) {
        let nodes = std::mem::take(&mut self.nodes);
        for (e, n) in nodes {
            let n = match n {
                Node::AliasVal(s) => Node::AliasVal(sym(s)),
                Node::Apply(c) => Node::Apply(class(c)),
                Node::Export(c, n, r) => Node::Export(class(c), n, expr(r)),
                Node::Constant(s) => Node::Constant(sym(s)),
            };
            self.nodes.insert(expr(e), n);
        }
        let comp = |c: Comp| match c {
            Comp::Sig(s) => Comp::Sig(sym(s)),
            Comp::Body(s) => Comp::Body(sym(s)),
            Comp::Class(k) => Comp::Class(class(k)),
            Comp::ClassBody(k) => Comp::ClassBody(class(k)),
            Comp::Alias(a) => Comp::Alias(alias(a)),
        };
        let node_comps = std::mem::take(&mut self.node_comps);
        for (c, es) in node_comps {
            self.node_comps.entry(comp(c)).or_default().extend(es.into_iter().map(&expr));
        }
        let records = std::mem::take(&mut self.records);
        for (c, evs) in records {
            let evs = evs
                .into_iter()
                .map(|e| match e {
                    Ev::Alias(a) => Ev::Alias(alias(a)),
                    Ev::Pkg(p) => Ev::Pkg(pkg(p)),
                    Ev::Name(n) => Ev::Name(n),
                    Ev::ImportPath(s) => Ev::ImportPath(sym(s)),
                    Ev::ImportSel(prefix, n) => Ev::ImportSel(
                        match prefix {
                            ImportPrefix::Pkg(p) => ImportPrefix::Pkg(pkg(p)),
                            ImportPrefix::Class(k) => ImportPrefix::Class(class(k)),
                            ImportPrefix::Value(s) => ImportPrefix::Value(sym(s)),
                        },
                        n,
                    ),
                    Ev::Forwarder(k, n) => Ev::Forwarder(class(k), n),
                    Ev::LocalClass(k) => Ev::LocalClass(class(k)),
                })
                .collect();
            self.records.insert(comp(c), evs);
        }
    }

    /// Drops the records of the computations `gone` says a retype does again, their nodes'
    /// among them.
    pub fn forget(&mut self, gone: impl Fn(Comp) -> bool) {
        self.records.retain(|&c, _| !gone(c));
        let dropped: Vec<Comp> = self.node_comps.keys().copied().filter(|&c| gone(c)).collect();
        for c in dropped {
            for e in self.node_comps.remove(&c).unwrap_or_default() {
                self.nodes.remove(&e);
            }
        }
    }

    /// The bytes the records hold.
    pub fn held(&self) -> usize {
        crate::held::table(&self.records)
            + self.records.values().map(crate::held::array).sum::<usize>()
            + crate::held::table(&self.nodes)
            + crate::held::table(&self.node_comps)
            + self.node_comps.values().map(crate::held::array).sum::<usize>()
    }
}

/// The saved computation of a `deps_enter`, which `deps_leave` restores: `None` where nothing
/// was entered.
pub struct Entered(Option<Option<Comp>>);

impl<'a> Worker<'a> {
    /// Makes the computation `comp` of the non-local definition `sym` the one records go to.
    #[inline]
    pub(super) fn deps_enter_sym(&mut self, sym: SymId, comp: Comp) -> Entered {
        if self.deps.is_none() {
            return Entered(None);
        }
        self.deps_enter_sym_now(sym, comp)
    }

    #[cold]
    #[inline(never)]
    fn deps_enter_sym_now(&mut self, sym: SymId, comp: Comp) -> Entered {
        let nonlocal = match self.syms.sym(sym).owner {
            Owner::Package(_) => true,
            Owner::Class(c) => !self.class_is_local(c),
            Owner::Local => false,
        };
        if !nonlocal {
            return Entered(None);
        }
        let d = self.deps.as_mut().expect("deps on");
        Entered(Some(d.current.replace(comp)))
    }

    #[inline]
    pub(super) fn deps_enter_class(&mut self, c: ClassId) -> Entered {
        if self.deps.is_none() {
            return Entered(None);
        }
        self.deps_enter_class_now(c)
    }

    #[cold]
    #[inline(never)]
    fn deps_enter_class_now(&mut self, c: ClassId) -> Entered {
        if self.class_is_local(c) {
            return Entered(None);
        }
        let d = self.deps.as_mut().expect("deps on");
        Entered(Some(d.current.replace(Comp::Class(c))))
    }

    /// Makes the check of the non-local class `c`'s body the computation records go to.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_enter_class_body(&mut self, c: ClassId) -> Entered {
        if self.deps.is_none() || self.class_is_local(c) {
            return Entered(None);
        }
        let d = self.deps.as_mut().expect("deps on");
        Entered(Some(d.current.replace(Comp::ClassBody(c))))
    }

    #[inline]
    pub(super) fn deps_enter_alias(&mut self, a: AliasId) -> Entered {
        if self.deps.is_none() {
            return Entered(None);
        }
        self.deps_enter_alias_now(a)
    }

    #[cold]
    #[inline(never)]
    fn deps_enter_alias_now(&mut self, a: AliasId) -> Entered {
        let nonlocal = match self.syms.alias(a).owner {
            Owner::Package(_) => true,
            Owner::Class(c) => !self.class_is_local(c),
            Owner::Local => false,
        };
        if !nonlocal {
            return Entered(None);
        }
        let d = self.deps.as_mut().expect("deps on");
        Entered(Some(d.current.replace(Comp::Alias(a))))
    }

    #[inline]
    pub(super) fn deps_leave(&mut self, e: Entered) {
        if let (Some(outer), Some(d)) = (e.0, self.deps.as_mut()) {
            d.current = outer;
        }
    }

    fn class_is_local(&self, mut c: ClassId) -> bool {
        loop {
            match self.syms.class(c).owner {
                Owner::Local => return true,
                Owner::Package(_) => return false,
                Owner::Class(o) => c = o,
            }
        }
    }

    /// What the node `e` stands for where the typer simplified it.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_node(&mut self, e: TExprId, n: Node) {
        if let Some(d) = self.deps.as_mut() {
            d.node(e, n);
        }
    }

    /// An import of a class's or a block's body, as scalac's collector reads its tree: the
    /// packages, objects and values its path goes through, its selector's term and type, a
    /// rename's name.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_import(&mut self, imp: &crate::ast::Import) {
        if self.deps.is_none() {
            return;
        }
        let mut evs: Vec<Ev> = Vec::new();
        let mut prefix: Option<ImportPrefix> = None;
        for &seg in &imp.path {
            let found = match prefix {
                None if seg == crate::names::ROOT => Some(super::resolve::TermRef::Package(ROOT_PKG)),
                None => self.lookup_term(seg),
                Some(ImportPrefix::Pkg(p)) => match self.demand_pkg(p, seg) {
                    Some(sub) => Some(super::resolve::TermRef::Package(sub)),
                    None => self.pkg_term(p, seg),
                },
                Some(ImportPrefix::Class(o)) => self.module_term(o, seg),
                Some(ImportPrefix::Value(v)) => {
                    let Some(t) = self.syms.sym(v).sig.as_ref().map(|sig| sig.ret) else { return };
                    self.find_member(t, seg).map(|(m, _)| super::resolve::TermRef::Global(m))
                }
            };
            let Some(r) = found else { return };
            if let super::resolve::TermRef::Package(p) = r {
                if p != ROOT_PKG {
                    evs.push(Ev::Pkg(p));
                }
                prefix = Some(ImportPrefix::Pkg(p));
                continue;
            }
            let Some(s) = r.sym() else { return };
            evs.push(Ev::ImportPath(s));
            prefix = Some(match self.syms.sym(s).kind {
                SymKind::Object(c) => ImportPrefix::Class(c),
                _ => ImportPrefix::Value(s),
            });
        }
        let Some(prefix) = prefix else { return };
        if let crate::ast::ImportSel::Name(n, rename) = imp.sel {
            evs.push(Ev::ImportSel(prefix, n));
            if let Some(r) = rename.filter(|&r| r != n && r != crate::names::WILDCARD) {
                evs.push(Ev::Name(r));
            }
        }
        if let Some(d) = self.deps.as_mut() {
            for ev in evs {
                d.record(ev);
            }
        }
    }

    /// A local class the code typed defines.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_local_class(&mut self, c: ClassId) {
        if let Some(d) = self.deps.as_mut() {
            d.record(Ev::LocalClass(c));
        }
    }

    /// A name of a written type that resolves to none of teq's symbols.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_written_name(&mut self, name: &str) {
        let n = self.interner.intern(name);
        if let Some(d) = self.deps.as_mut() {
            d.record(Ev::Name(n));
        }
    }

    /// A written type resolved to `r`: an alias's identity, which the type loses.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_type_ref(&mut self, r: super::resolve::TypeRef) {
        if let super::resolve::TypeRef::Alias(a) = r {
            if let Some(d) = self.deps.as_mut() {
                d.record(Ev::Alias(a));
            }
        }
    }

    /// `p.T` of a path `p`: the alias of `p`'s class, or the original of the forwarder the
    /// class exports it by, which the type holds expanded.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_member_alias(&mut self, prefix: TypeId, name: Name) {
        let widened = match self.types.get(prefix) {
            Type::Term(s) => self.syms.sym(s).sig.as_ref().map(|sig| sig.ret),
            _ => Some(prefix),
        };
        let Some(Type::Class(c, _) | Type::This(c)) = widened.map(|t| self.types.get(t)) else { return };
        if let Some(&a) = self.syms.class(c).type_aliases.get(&name) {
            if let Some(d) = self.deps.as_mut() {
                d.record(Ev::Alias(a));
            }
            return;
        }
        self.deps_export_type(c, name);
    }

    /// `p.n` of a type `n` the class `c` exports: scalac's forwarder of `c` besides the
    /// original the typer resolves it to.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_export_type(&mut self, c: ClassId, name: Name) {
        if !self.syms.class(c).has_exports || self.syms.class(c).type_aliases.contains_key(&name) || self.syms.class(c).nested.contains_key(&name) {
            return;
        }
        let Some(r) = self.exports_of(c).and_then(|e| e.types.get(&name).copied()) else { return };
        // The forwarder, and the forwarders of the exports it goes through to the original.
        let mut chain = vec![(c, name)];
        let (mut at, mut n) = (c, name);
        for _ in 0..16 {
            let Some((o, m)) = self.export_source(at, n) else { break };
            let own = self.syms.class(o).type_aliases.contains_key(&m) || self.syms.class(o).nested.contains_key(&m);
            if own || !self.syms.class(o).has_exports || self.exports_of(o).map_or(true, |e| !e.types.contains_key(&m)) {
                break;
            }
            chain.push((o, m));
            (at, n) = (o, m);
        }
        let Some(d) = self.deps.as_mut() else { return };
        for (k, m) in chain {
            d.record(Ev::Forwarder(k, m));
        }
        if let super::resolve::TypeRef::Alias(a) = r {
            d.record(Ev::Alias(a));
        }
    }

    /// The object and the name the export of `name` by the class `c` selects: `(B, U)` of
    /// `export B.{U => W}` for `W`.
    fn export_source(&mut self, c: ClassId, name: Name) -> Option<(ClassId, Name)> {
        let (file, def) = {
            let info = self.syms.class(c);
            (info.file, info.def?)
        };
        let ast = self.ast(file);
        let crate::ast::DefKind::Class(cls) = &ast.def(def).kind else { return None };
        let clauses: Vec<(Vec<Name>, crate::ast::ImportSel)> = ast.exports[cls.exports.range()].iter().map(|i| (i.path.clone(), i.sel.clone())).collect();
        for (path, sel) in clauses {
            let selected = match sel {
                crate::ast::ImportSel::Name(n, rename) if rename.unwrap_or(n) == name => n,
                crate::ast::ImportSel::Wildcard => name,
                _ => continue,
            };
            let Some(super::exports::ExportQualifier::Object(o)) = self.export_qualifier(c, &path) else { continue };
            let reaches = self.syms.class(o).type_aliases.contains_key(&selected)
                || self.syms.class(o).nested.contains_key(&selected)
                || self.exports_of(o).map_or(false, |e| e.types.contains_key(&selected));
            if reaches {
                return Some((o, selected));
            }
        }
        None
    }

    /// `q.n` of a written type whose path `q` is an object that exports `n`.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_export_path(&mut self, q: crate::ast::TyExprId, name: Name) {
        let Some(s) = self.resolve_type_path(q).and_then(|r| r.sym()) else { return };
        if let SymKind::Object(c) = self.syms.sym(s).kind {
            self.deps_export_type(c, name);
        }
    }

    /// The path of a written type (`scala.collection.mutable.Map`): its packages.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_type_path(&mut self, q: crate::ast::TyExprId) {
        // Through the objects of the path (`p.O.C`) to its package.
        let mut q = q;
        for _ in 0..16 {
            match self.resolve_type_path(q) {
                Some(super::resolve::TermRef::Package(p)) => return self.deps_package_path(p),
                Some(_) => match self.cur_ast().ty(q) {
                    crate::ast::TyExpr::Select(inner, _) => q = inner,
                    _ => return,
                },
                None => return,
            }
        }
    }

    /// A path through the package `p` and the packages it is in (`a.b.C` of a term or a type):
    /// each package's name.
    #[cold]
    #[inline(never)]
    pub(super) fn deps_package_path(&mut self, mut p: PkgId) {
        let mut pkgs = Vec::new();
        while p != ROOT_PKG {
            pkgs.push(p);
            match self.syms.pkg(p).parent {
                Some(parent) => p = parent,
                None => break,
            }
        }
        if let Some(d) = self.deps.as_mut() {
            for p in pkgs {
                d.record(Ev::Pkg(p));
            }
        }
    }
}

/// zinc's `DependencyContext`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Context {
    MemberRef,
    Inheritance,
    LocalInheritance,
}

impl Context {
    fn name(self) -> &'static str {
        match self {
            Context::MemberRef => "DependencyByMemberRef",
            Context::Inheritance => "DependencyByInheritance",
            Context::LocalInheritance => "LocalDependencyByInheritance",
        }
    }
}

/// What one class depends on, as the answer states it.
#[derive(Default)]
struct ClassDeps {
    /// The names used: scalac keys a term's name and a type's apart, and a sealed class a
    /// match's selector reaches takes its type name's `PatMatTarget` scope.
    names: BTreeMap<String, NameUse>,
    classes: BTreeSet<(String, Context)>,
    binaries: BTreeSet<(String, String, Context)>,
}

/// How a class uses a name: as a term's, as a type's, as a sealed class's a match's selector
/// reaches (the type's with `PatMatTarget`).
#[derive(Default, Clone, Copy)]
struct NameUse {
    term: bool,
    typ: bool,
    patmat: bool,
}

/// Where a class depended on is, as scalac's `recordClassDependency` finds it through the
/// class's associated file.
#[derive(Clone)]
enum Target {
    /// A class of the build's sources: the name zinc keys it by and its source file.
    Source(String, FileId),
    /// A class of the class path: the file that holds it (a jar, a class file, `jrt:` a class of
    /// the JDK's runtime image) and its binary name.
    Binary(String, String),
    /// A class no file holds: `Any`, `Nothing`, `Null`, `Singleton`, the compiler's own.
    Nowhere,
}

/// The class a record goes to: a class of the program, or a package block's `<file>$package`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Owned {
    Class(ClassId),
    Package(FileId, PkgId),
}

/// What a traversal has met.
#[derive(Default)]
struct Seen {
    types: FxMap<TypeId, ()>,
    syms: FxMap<SymId, ()>,
    classes: FxMap<ClassId, ()>,
    aliases: FxMap<AliasId, ()>,
    tparams: FxMap<TParamId, ()>,
}

/// A top-level definition: a term or an alias.
#[derive(Clone, Copy)]
enum PackageMember {
    Sym(SymId),
    Alias(AliasId),
}

/// Who a file's trees outside its classes are recorded for: a class, a `<file>$package`, or
/// the class of an `@main` method.
#[derive(Clone, Copy)]
enum Responsible {
    Owned(Owned),
    Main(FileId, PkgId, Name),
}

/// What a call's arguments are passed to, for the default getters of the ones left out.
#[derive(Clone, Copy)]
enum Callee {
    Method(SymId),
    Ctor(ClassId),
    Apply(ClassId),
}

/// The method a builtin binary operator of teq's IR is in scalac's tree.
fn prim_name(op: PrimOp) -> &'static str {
    use PrimOp::*;
    match op {
        IntAdd | LongAdd | DoubleAdd | FloatAdd => "+",
        IntSub | LongSub | DoubleSub | FloatSub => "-",
        IntMul | LongMul | DoubleMul | FloatMul => "*",
        IntDiv | LongDiv | DoubleDiv | FloatDiv => "/",
        IntRem | LongRem | DoubleRem | FloatRem => "%",
        IntAnd | LongAnd | BoolStrictAnd => "&",
        IntOr | LongOr | BoolStrictOr => "|",
        IntXor | LongXor | BoolXor => "^",
        IntShl | LongShl => "<<",
        IntShr | LongShr => ">>",
        IntUshr | LongUshr => ">>>",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        RefEq | Eq => "==",
        RefNe | Ne => "!=",
        BoolAnd => "&&",
        BoolOr => "||",
    }
}

/// The method a builtin unary operator is, where it is one (a conversion is none).
fn unary_name(op: UnOp) -> Option<&'static str> {
    match op {
        UnOp::IntNeg | UnOp::LongNeg | UnOp::DoubleNeg | UnOp::FloatNeg => Some("unary_-"),
        UnOp::BoolNot => Some("unary_!"),
        UnOp::IntNot | UnOp::LongNot => Some("unary_~"),
        _ => None,
    }
}

/// The dependencies of an answer: per file the JSON object of its `deps`, and the table of the
/// binary entries they name by index (the answer's `entries`).
pub struct Rendered {
    pub files: FxMap<FileId, String>,
    pub entries: String,
}

/// The dependencies of the classes of `files` (the answer's files, a `package` block counted as
/// its file, `unit_file`).
/// The time the last `render` took, for the timings of a build and a session.
pub static RENDER_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The dependencies of `files`, or why the answer cannot state them.
pub fn render(w: &mut Worker, files: &[FileId], unit_file: &[FileId]) -> Result<Rendered, Vec<String>> {
    let start = std::time::Instant::now();
    let reported = w.diags.items.len();
    let mut c = Collector::new(w, unit_file);
    let wanted: FxMap<FileId, ()> = files.iter().map(|&f| (f, ())).collect();
    c.collect(&wanted);
    // What the collection looks up is resolved already: it reports nothing of its own.
    debug_assert_eq!(c.w.diags.items.len(), reported, "the dependencies' collection reported a diagnostic");
    let mut out: FxMap<FileId, String> = FxMap::default();
    let mut entries: Vec<String> = Vec::new();
    let mut entry_index: FxMap<String, usize> = FxMap::default();
    for &f in files {
        let owners = c.by_file.remove(&f).unwrap_or_default();
        let mut text = String::from("{");
        let mut first = true;
        for name in owners {
            let Some(d) = c.deps.get(&name) else { continue };
            if !first {
                text.push(',');
            }
            first = false;
            json_string(&name, &mut text);
            for (i, key) in ["names", "patmat"].iter().enumerate() {
                text.push_str(if i == 0 { ":{\"" } else { "],\"" });
                text.push_str(key);
                text.push_str("\":[");
                let mut any = false;
                for (n, u) in d.names.iter() {
                    // A term's name, and a type's that no match reaches, with `Default`; a sealed
                    // class's type name with `PatMatTarget` besides.
                    let listed = if i == 0 { u.term || (u.typ && !u.patmat) } else { u.patmat };
                    if !listed {
                        continue;
                    }
                    if any {
                        text.push(',');
                    }
                    any = true;
                    json_string(n, &mut text);
                }
            }
            text.push_str("],\"classes\":[");
            for (i, (on, ctx)) in d.classes.iter().enumerate() {
                if i > 0 {
                    text.push(',');
                }
                text.push('[');
                json_string(on, &mut text);
                text.push_str(",\"");
                text.push_str(ctx.name());
                text.push_str("\"]");
            }
            text.push_str("],\"binaries\":[");
            for (i, (entry, binary, ctx)) in d.binaries.iter().enumerate() {
                if i > 0 {
                    text.push(',');
                }
                let index = *entry_index.entry(entry.clone()).or_insert_with(|| {
                    entries.push(entry.clone());
                    entries.len() - 1
                });
                text.push('[');
                text.push_str(&index.to_string());
                text.push(',');
                json_string(binary, &mut text);
                text.push_str(",\"");
                text.push_str(ctx.name());
                text.push_str("\"]");
            }
            text.push_str("]}");
        }
        text.push('}');
        out.insert(f, text);
    }
    let mut table = String::from("[");
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            table.push(',');
        }
        json_string(e, &mut table);
    }
    table.push(']');
    RENDER_NANOS.store(start.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
    Ok(Rendered { files: out, entries: table })
}

struct Collector<'c, 'a> {
    w: &'c mut Worker<'a>,
    unit_file: &'c [FileId],
    /// Per class, by the name zinc keys it by, what it depends on.
    deps: FxMap<String, ClassDeps>,
    /// Per answer file, the names of the classes whose dependencies it holds.
    by_file: FxMap<FileId, BTreeSet<String>>,
    /// The class records go to, by name, and the file its source is.
    owner: String,
    owner_file: FileId,
    /// What the traversals of trees met for the current owner, which gives it nothing new
    /// again; and what the traversal of an info under way met, where the info's own type
    /// parameters are named by position and so record otherwise (scalac's `scratchSeen`).
    tree_seen: Seen,
    info_seen: Seen,
    targets: FxMap<ClassId, Target>,
    /// The class each type parameter of a class belongs to, and none for a method's.
    tparam_class: FxMap<TParamId, Option<ClassId>>,
    /// Each class's kept bodies, by class.
    tclass_of: FxMap<ClassId, usize>,
    /// The method type parameters a signature binds while it is traversed as a member's info,
    /// where scalac's types refer to them by position (`TypeParamRef`): no name of theirs.
    bound: Vec<TParamId>,
    /// The term parameters of the signature traversed, with their types as the prefix sees
    /// them: a singleton of one is scalac's `TermParamRef`, which names no symbol.
    bound_terms: Vec<(SymId, TypeId)>,
    /// The locals an inline expansion binds its receiver and arguments to.
    proxies: FxMap<SymId, ()>,
    /// Whether the type under way is reached through a package's path.
    via_package_path: bool,
    /// The local and anonymous classes walked, and the nodes: each once per owner.
    walked_classes: FxMap<ClassId, ()>,
    walked: FxMap<TExprId, ()>,
    walked_pats: FxMap<TPatId, ()>,
    /// The classes whose bodies are walked, innermost last: what `this` is.
    this: Vec<ClassId>,
    /// The mangled full name of the `<file>$package` whose top-level definitions are walked, for
    /// the local classes they make.
    package_this: Option<String>,
    /// The package of the class records go to.
    owner_pkg: Option<PkgId>,
    /// The class records go to, none for a `<file>$package`.
    owner_class: Option<ClassId>,
    /// The inline accessor's name of each member an inline body reads through one, by the class
    /// that holds the accessor (`Worker::inline_accessors`), made once it is first asked for.
    accessor_names: Option<FxMap<(ClassId, SymId), String>>,
    /// Whether the code walked is an inline method's stored body.
    in_inline_body: bool,
    /// The classes whose inline bodies the code walked comes from, innermost last: the stored
    /// body's method's, an expansion's callees'. Their accessors are what the code reads through.
    inline_hosts: Vec<ClassId>,
    /// The top-level classes scalac's desugaring puts into their block's `<file>$package` (a
    /// given's class, an implicit class, the companion of an opaque type), with it.
    wrapped: FxMap<ClassId, (FileId, PkgId)>,
    /// The `New` nodes that stand for a case class's `apply`.
    applied: FxMap<TExprId, ()>,
    /// The calls of an original that stand for an export forwarder's call.
    forwarded: FxMap<TExprId, ()>,
    /// The `New` nodes that stand for a case class's `copy`.
    copies: FxMap<TExprId, ()>,
    /// The packages the build's sources are in, made on the first package named.
    source_packages: Option<FxMap<PkgId, ()>>,
    /// The classes of the givens that are objects in scalac (`given Show[Int] with`, no
    /// parameters), by class.
    given_objects: FxMap<ClassId, ()>,
}

impl<'c, 'a> Collector<'c, 'a> {
    fn new(w: &'c mut Worker<'a>, unit_file: &'c [FileId]) -> Self {
        let mut tparam_class: FxMap<TParamId, Option<ClassId>> = FxMap::default();
        for (i, info) in w.syms.classes.iter().enumerate() {
            for &p in &info.tparams {
                tparam_class.insert(p, Some(ClassId(i as u32)));
            }
        }
        let tclass_of: FxMap<ClassId, usize> = w.prog.classes.iter().enumerate().map(|(i, tc)| (tc.id, i)).collect();
        let mut given_objects: FxMap<ClassId, ()> = FxMap::default();
        for info in w.syms.syms.iter() {
            if let (SymKind::Given, Some(c)) = (info.kind, info.impl_class) {
                if info.sig.as_ref().map_or(true, |s| s.tparams.is_empty() && s.clauses.is_empty()) {
                    given_objects.insert(c, ());
                }
            }
        }
        Collector {
            w,
            unit_file,
            deps: FxMap::default(),
            by_file: FxMap::default(),
            owner: String::new(),
            owner_file: FileId(0),
            tree_seen: Seen::default(),
            info_seen: Seen::default(),
            targets: FxMap::default(),
            tparam_class,
            tclass_of,
            bound: Vec::new(),
            walked_classes: FxMap::default(),
            walked: FxMap::default(),
            walked_pats: FxMap::default(),
            this: Vec::new(),
            package_this: None,
            owner_pkg: None,
            owner_class: None,
            accessor_names: None,
            in_inline_body: false,
            inline_hosts: Vec::new(),
            bound_terms: Vec::new(),
            proxies: FxMap::default(),
            via_package_path: false,
            wrapped: FxMap::default(),
            applied: FxMap::default(),
            forwarded: FxMap::default(),
            copies: FxMap::default(),
            source_packages: None,
            given_objects,
        }
    }

    fn unit(&self, f: FileId) -> FileId {
        self.unit_file.get(f.0 as usize).copied().unwrap_or(f)
    }

    // ---- what is collected, owner by owner ----

    fn collect(&mut self, files: &FxMap<FileId, ()>) {
        let mut wanted: Vec<FileId> = files.keys().copied().collect();
        wanted.sort();
        for f in 0..self.w.files.len() {
            let file = FileId(f as u32);
            if !self.w.program_file(file) {
                continue;
            }
            for unit in crate::tasty::write::units_of(self.w, file) {
                if let crate::tasty::write::UnitKind::Package { pkg, defs, .. } = unit.kind {
                    for d in defs {
                        if let Some(&c) = self.w.def_classes.get(f, &d) {
                            self.wrapped.insert(c, (file, pkg));
                        }
                    }
                }
            }
        }
        for &f in &wanted {
            self.package_level(f);
        }
        self.recorded(files);
        let w = &*self.w;
        let mut classes: Vec<ClassId> = Vec::new();
        for i in 0..w.syms.classes.len() {
            let c = ClassId(i as u32);
            let info = w.syms.class(c);
            if info.def.is_none() || !files.contains_key(&self.unit(info.file)) || !w.program_file(info.file) {
                continue;
            }
            if self.is_local(c) || matches!(info.kind, ClassKind::Anon | ClassKind::Builtin | ClassKind::Opaque) {
                continue;
            }
            // An enum's case without parameters is a value of the companion, no class of its own.
            if info.kind == ClassKind::EnumCase && info.singleton.is_some() {
                continue;
            }
            classes.push(c);
        }
        for c in classes {
            self.enter(Owned::Class(c));
            self.template(c);
        }
        // The top-level definitions of each package block, `<file>$package`'s.
        let mut tops: Vec<(Owned, SymId)> = Vec::new();
        for (i, info) in self.w.syms.syms.iter().enumerate() {
            let Owner::Package(p) = info.owner else { continue };
            if info.def.is_none() || !files.contains_key(&self.unit(info.file)) || !self.w.program_file(info.file) {
                continue;
            }
            if matches!(info.kind, SymKind::Object(_) | SymKind::Overloaded(_)) || info.impl_class.map_or(false, |c| self.given_objects.contains_key(&c)) {
                continue;
            }
            tops.push((Owned::Package(info.file, p), SymId(i as u32)));
        }
        let mut packages: Vec<Owned> = tops.iter().map(|&(o, _)| o).collect();
        for (i, info) in self.w.syms.aliases.iter().enumerate() {
            let _ = i;
            if let Owner::Package(p) = info.owner {
                if info.def.is_some() && files.contains_key(&self.unit(info.file)) && self.w.program_file(info.file) {
                    packages.push(Owned::Package(info.file, p));
                }
            }
        }
        for (&c, &(f, p)) in self.wrapped.iter() {
            if files.contains_key(&self.unit(f)) && self.w.syms.class(c).def.is_some() {
                packages.push(Owned::Package(f, p));
            }
        }
        packages.sort_by_key(|o| format!("{:?}", o));
        packages.dedup();
        for owned in packages {
            self.enter(owned);
            self.package_template();
        }
        for (owned, s) in tops {
            self.enter(owned);
            self.member(s);
            self.inline_definition(s);
        }
        let mut aliases: Vec<(Owned, AliasId)> = Vec::new();
        for (i, info) in self.w.syms.aliases.iter().enumerate() {
            let Owner::Package(p) = info.owner else { continue };
            if info.def.is_none() || !files.contains_key(&self.unit(info.file)) || !self.w.program_file(info.file) {
                continue;
            }
            aliases.push((Owned::Package(info.file, p), AliasId(i as u32)));
        }
        for (owned, a) in aliases {
            self.enter(owned);
            self.alias_def(a);
        }
        let funs: Vec<FunId> = self.w.prog.top_funs.iter().copied().collect();
        for f in funs {
            let s = self.w.prog.funs[f.idx()].sym;
            let info = self.w.syms.sym(s);
            let (file, owner) = (info.file, info.owner);
            let Owner::Package(p) = owner else { continue };
            if !files.contains_key(&self.unit(file)) {
                continue;
            }
            self.enter(Owned::Package(file, p));
            self.fun(f);
        }
        self.main_classes(files);
        let vals: Vec<(SymId, TExprId)> = self.w.prog.top_vals.iter().copied().collect();
        for (s, e) in vals {
            let info = self.w.syms.sym(s);
            let (file, owner) = (info.file, info.owner);
            let Owner::Package(p) = owner else { continue };
            if !files.contains_key(&self.unit(file)) || info.def.is_none() || self.is_module_val(s) {
                continue;
            }
            self.enter(Owned::Package(file, p));
            self.expr(e);
        }
    }

    /// The trees of a file outside its classes (its package clauses and imports), which scalac
    /// records under the last class of the file (`responsibleForImports`: the last type
    /// definition a walk of the tree meets, a package block's `<file>$package` at the end of
    /// its block).
    fn package_level(&mut self, f: FileId) {
        let units = self.units_of_file(f);
        match self.responsible_for_imports(f) {
            Some(Responsible::Main(file, p, name)) => self.enter_main(file, p, name),
            Some(Responsible::Owned(owned)) => self.enter(owned),
            None => return,
        }
        for &u in &units {
            let names: Vec<Name> = self.w.ast(u).package.clone();
            for n in names {
                self.use_name(self.w.name_str(n));
            }
            let packages = self.clause_packages(u);
            let imports: Vec<(Vec<Name>, crate::ast::ImportSel)> =
                self.w.ast(u).imports.iter().chain(&self.w.ast(u).language_imports).map(|i| (i.path.clone(), i.sel.clone())).collect();
            for (path, sel) in imports {
                self.import(u, &path, &sel, &packages);
            }
        }
    }

    /// The packages of a file's `package` clauses, innermost first: where the first name of a
    /// file import's path is found before the root's (a clause `a.b` opens `a.b` alone).
    fn clause_packages(&self, u: FileId) -> Vec<PkgId> {
        let ast = self.w.ast(u);
        let mut out = Vec::new();
        let mut p = ROOT_PKG;
        let mut seg = 0;
        for &end in &ast.package_clauses {
            while seg < end as usize {
                match self.w.syms.pkg(p).entries.get(&ast.package[seg]).and_then(|e| e.pkg) {
                    Some(q) => p = q,
                    None => return out,
                }
                seg += 1;
            }
            out.push(p);
        }
        out.reverse();
        out
    }

    fn units_of_file(&self, f: FileId) -> Vec<FileId> {
        (0..self.unit_file.len().max(self.w.files.len()))
            .map(|i| FileId(i as u32))
            .filter(|&u| (u.0 as usize) < self.w.files.len() && self.unit(u) == f)
            .collect()
    }

    /// The owner scalac's collector gives a file's trees outside its classes.
    fn responsible_for_imports(&mut self, f: FileId) -> Option<Responsible> {
        let units = self.units_of_file(f);
        let mut last: Option<(u32, Owned)> = None;
        for &u in &units {
            let units = crate::tasty::write::units_of(self.w, u);
            // A block's `<file>$package` comes after the block's classes too.
            let mut end = 0u32;
            for unit in &units {
                if let crate::tasty::write::UnitKind::Class { class, module } = unit.kind {
                    for c in class.into_iter().chain(module) {
                        end = end.max(self.w.syms.class(c).span.end);
                    }
                }
            }
            for unit in units {
                match unit.kind {
                    crate::tasty::write::UnitKind::Class { class, module } => {
                        for c in class.into_iter().chain(module) {
                            let span = self.w.syms.class(c).span;
                            if last.map_or(true, |(k, _)| span.start >= k) {
                                last = Some((span.start, Owned::Class(c)));
                            }
                        }
                    }
                    crate::tasty::write::UnitKind::Package { pkg, defs, .. } => {
                        let ast = self.w.ast(u);
                        let at = if u == f { u32::MAX } else { defs.iter().map(|&d| ast.def(d).span.end).max().unwrap_or(0).max(end) };
                        if last.map_or(true, |(k, _)| at >= k) {
                            last = Some((at, Owned::Package(u, pkg)));
                        }
                    }
                }
            }
        }
        let mains = self.main_methods(f);
        match (mains.last(), last) {
            (Some(&m), _) => {
                let info = self.w.syms.sym(m);
                let Owner::Package(p) = info.owner else { return None };
                Some(Responsible::Main(info.file, p, info.name))
            }
            (None, Some((_, owned))) => Some(Responsible::Owned(owned)),
            (None, None) => None,
        }
    }

    /// An import clause of one selector of the unit `u`: its path's packages, objects and
    /// values, and the selector's term and type members (a rename's name besides), a wildcard's
    /// nothing more. The path's first name is the typer's binding of it in the file, or else
    /// the first of `packages` (innermost first) that has it, or else the root's.
    fn import(&mut self, u: FileId, path: &[Name], sel: &crate::ast::ImportSel, packages: &[PkgId]) {
        let Some((&head, rest)) = path.split_first() else { return };
        let Some(mut prefix) = self.import_head(u, head, packages) else { return };
        for &seg in rest {
            let next = match prefix {
                ImportPrefix::Pkg(p) => {
                    // The class path's packages and objects are entered where something asks for them.
                    let entry = self.w.syms.pkg(p).entries.get(&seg).cloned().unwrap_or_default();
                    if let Some(sub) = entry.pkg.or_else(|| if entry.term.is_none() { self.w.demand_pkg(p, seg) } else { None }) {
                        Some(self.import_package(sub))
                    } else {
                        entry.term.or_else(|| self.w.pkg_term(p, seg).and_then(|r| r.sym())).and_then(|t| self.import_term(t))
                    }
                }
                ImportPrefix::Class(o) => {
                    let nested = self.w.syms.class(o).nested.get(&seg).and_then(|&k| self.w.syms.class(k).module_sym);
                    nested.or_else(|| self.w.module_term(o, seg).and_then(|r| r.sym())).and_then(|t| self.import_term(t))
                }
                ImportPrefix::Value(v) => {
                    let ty = self.w.syms.sym(v).sig.as_ref().map(|sig| sig.ret);
                    ty.and_then(|t| self.w.find_member(t, seg)).and_then(|(m, _)| self.import_term(m))
                }
            };
            match next {
                Some(p) => prefix = p,
                None => return,
            }
        }
        let crate::ast::ImportSel::Name(n, rename) = *sel else { return };
        self.import_selector(prefix, n);
        if let Some(r) = rename {
            if r != n && self.w.name_ref(r) != "_" {
                self.use_name(self.w.name_str(r));
            }
        }
    }

    /// A file import's first name, as the typer binds it where the file's imports are resolved
    /// (a package, an object, a stable value, an earlier import's), or else in the first of
    /// `packages` that has it, or else at the root.
    fn import_head(&mut self, u: FileId, head: Name, packages: &[PkgId]) -> Option<ImportPrefix> {
        if head == crate::names::ROOT {
            return Some(ImportPrefix::Pkg(ROOT_PKG));
        }
        let bound = if self.w.file_imports[u.0 as usize].is_some() {
            let env = super::Env { file: u, frames: Vec::new(), imports: Vec::new() };
            self.w.with_env(env, |w| w.lookup_term(head))
        } else {
            None
        };
        match bound {
            Some(super::resolve::TermRef::Package(p)) => Some(self.import_package(p)),
            Some(r) => r.sym().and_then(|t| self.import_term(t)),
            None => {
                let p = packages.iter().copied().find(|&q| self.package_has(q, head)).unwrap_or(ROOT_PKG);
                let entry = self.w.syms.pkg(p).entries.get(&head).cloned().unwrap_or_default();
                match entry.pkg.or_else(|| if entry.term.is_none() { self.w.demand_pkg(p, head) } else { None }) {
                    Some(sub) => Some(self.import_package(sub)),
                    None => entry.term.or_else(|| self.w.pkg_term(p, head).and_then(|r| r.sym())).and_then(|t| self.import_term(t)),
                }
            }
        }
    }

    /// A package an import's path goes through: its name.
    fn import_package(&mut self, p: PkgId) -> ImportPrefix {
        if p != ROOT_PKG {
            let name = self.package_name(p);
            self.use_name(name);
        }
        ImportPrefix::Pkg(p)
    }

    /// An object or a stable value an import's path goes through: its reference, and what the
    /// rest of the path selects on.
    fn import_term(&mut self, t: SymId) -> Option<ImportPrefix> {
        let kind = self.w.syms.sym(t).kind;
        self.term_node(t, None);
        Some(match kind {
            SymKind::Object(c) => ImportPrefix::Class(c),
            _ => ImportPrefix::Value(t),
        })
    }

    /// Whether a package has a member or a subpackage `n`, of the build or of the class path.
    fn package_has(&mut self, p: PkgId, n: Name) -> bool {
        let entry = self.w.syms.pkg(p).entries.get(&n).cloned().unwrap_or_default();
        entry.pkg.is_some() || entry.term.is_some() || self.w.pkg_term(p, n).is_some() || self.w.demand_pkg(p, n).is_some()
    }

    /// An import's selector `n`: the term and the type member of the name.
    fn import_selector(&mut self, prefix: ImportPrefix, n: Name) {
        match prefix {
            ImportPrefix::Class(o) if self.exported_by(o, n) => {
                // The object's forwarders of the name, a term's and a type's.
                if let Some(scope) = self.w.exports_of(o) {
                    if scope.terms.contains_key(&n) || scope.extensions.contains_key(&n) {
                        self.use_term_name(self.w.name_str(n));
                        self.class_dep(o, Context::MemberRef);
                    }
                    if scope.types.contains_key(&n) {
                        self.use_name(self.w.name_str(n));
                        self.class_dep(o, Context::MemberRef);
                    }
                }
            }
            ImportPrefix::Class(o) => {
                let (term, class, alias) = {
                    let info = self.w.syms.class(o);
                    (info.members.get(&n).copied(), info.nested.get(&n).copied(), info.type_aliases.get(&n).copied())
                };
                // An object of the class path has its members entered where one is asked for.
                let term = term.or_else(|| self.w.module_term(o, n).and_then(|r| r.sym()));
                if let Some(t) = term {
                    let alts: Vec<SymId> = self.w.syms.alternatives(t).map(|a| a.to_vec()).unwrap_or_else(|| vec![t]);
                    if let Some(&first) = alts.first() {
                        self.member_ref(first);
                    }
                }
                if let Some(k) = class {
                    self.class_ref(k);
                }
                if let Some(a) = alias {
                    self.alias_ref(a);
                }
            }
            ImportPrefix::Pkg(p) => {
                let entry = self.w.syms.pkg(p).entries.get(&n).cloned().unwrap_or_default();
                if let Some(t) = entry.term {
                    self.member_ref(t);
                }
                if let Some(k) = entry.class {
                    self.class_ref(k);
                }
                if let Some(a) = entry.alias {
                    self.alias_ref(a);
                }
            }
            ImportPrefix::Value(v) => {
                let Some(t) = self.w.syms.sym(v).sig.as_ref().map(|sig| sig.ret) else { return };
                if let Some((m, _)) = self.w.find_member(t, n) {
                    self.member_ref(m);
                }
                // The type member of the name, of the value's class.
                if let Some(a) = self.member_alias(t, n) {
                    self.alias_ref(a);
                }
            }
        }
    }

    /// Whether the object `o` has the name `n` by an export alone, no definition of its own.
    fn exported_by(&mut self, o: ClassId, n: Name) -> bool {
        let declared = {
            let info = self.w.syms.class(o);
            !info.has_exports || info.members.contains_key(&n) || info.type_aliases.contains_key(&n) || info.nested.contains_key(&n)
        };
        if declared {
            return false;
        }
        self.w.exports_of(o).map_or(false, |scope| scope.terms.contains_key(&n) || scope.types.contains_key(&n) || scope.extensions.contains_key(&n))
    }

    /// `p.n` through the object `c` that exports `n`: its forwarder and the object.
    fn forwarder_ref(&mut self, c: ClassId, n: Name) {
        self.use_name(self.w.name_str(n));
        self.class_dep(c, Context::MemberRef);
        if let Some(m) = self.w.syms.class(c).module_sym {
            self.term_node(m, None);
        }
    }

    /// Whether the definition is scalac's module val: an object's, a given's without
    /// parameters, whose class is walked as a class of its own.
    fn is_module_val(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        matches!(info.kind, SymKind::Object(_)) || info.impl_class.map_or(false, |c| self.given_objects.contains_key(&c))
    }

    /// A reference to the alias `a` as a member: its name and its owner.
    fn alias_ref(&mut self, a: AliasId) {
        let (name, owner, file) = {
            let info = self.w.syms.alias(a);
            (info.name, info.owner, info.file)
        };
        self.use_name(self.w.name_str(name));
        match owner {
            Owner::Class(c) => self.class_dep(c, Context::MemberRef),
            Owner::Package(p) => {
                if let Some(t) = self.package_member_target(PackageMember::Alias(a), file, p) {
                    self.package_object_dep(t);
                }
            }
            Owner::Local => {}
        }
    }

    /// The template of a package block's `<file>$package`: `Object` called, its constructor.
    fn package_template(&mut self) {
        let object = self.w.b.any_ref;
        let t = self.w.b.t_any_ref;
        self.tree_type(t);
        self.class_dep(object, Context::Inheritance);
        self.ctor_call(object, t);
        let unit = self.w.b.t_unit;
        self.tree_type(unit);
    }

    /// What the typer recorded for the definitions of `files`, under their classes.
    fn recorded(&mut self, files: &FxMap<FileId, ()>) {
        let Some(rec) = self.w.deps.as_ref() else { return };
        let mut comps: Vec<(Comp, Vec<Ev>)> = rec.records.iter().map(|(&c, e)| (c, e.clone())).collect();
        comps.sort_by_key(|(c, _)| format!("{:?}", c));
        for (comp, evs) in comps {
            let owned = match comp {
                Comp::Sig(s) | Comp::Body(s) => {
                    let info = self.w.syms.sym(s);
                    match info.owner {
                        Owner::Class(c) => Owned::Class(c),
                        Owner::Package(p) => Owned::Package(info.file, p),
                        Owner::Local => continue,
                    }
                }
                // An opaque type is an alias to scalac: its records go to the class it is in.
                Comp::Class(c) if self.w.syms.class(c).kind == ClassKind::Opaque => match self.w.syms.class(c).owner {
                    Owner::Class(o) => Owned::Class(o),
                    Owner::Package(p) => Owned::Package(self.w.syms.class(c).file, p),
                    Owner::Local => continue,
                },
                // A value case of an enum is an anonymous class of the companion to scalac, whose
                // records go to the class around it.
                Comp::Class(c) | Comp::ClassBody(c) if self.w.syms.class(c).kind == ClassKind::EnumCase && self.w.syms.class(c).singleton.is_some() => {
                    match self.w.syms.class(c).owner {
                        Owner::Class(o) => Owned::Class(o),
                        Owner::Package(p) => Owned::Package(self.w.syms.class(c).file, p),
                        Owner::Local => continue,
                    }
                }
                Comp::Class(c) | Comp::ClassBody(c) => Owned::Class(c),
                Comp::Alias(a) => {
                    let info = self.w.syms.alias(a);
                    match info.owner {
                        Owner::Class(c) => Owned::Class(c),
                        Owner::Package(p) => Owned::Package(info.file, p),
                        Owner::Local => continue,
                    }
                }
            };
            let file = match owned {
                Owned::Class(c) => self.w.syms.class(c).file,
                Owned::Package(f, _) => f,
            };
            if !files.contains_key(&self.unit(file)) || !self.w.program_file(file) {
                continue;
            }
            if let Owned::Class(c) = owned {
                if self.w.syms.class(c).def.is_none() || matches!(self.w.syms.class(c).kind, ClassKind::Anon | ClassKind::Builtin | ClassKind::Opaque) {
                    continue;
                }
            }
            self.enter(owned);
            for ev in evs {
                match ev {
                    Ev::Alias(a) => self.named_alias(a),
                    Ev::Pkg(p) => self.package_ref(p),
                    Ev::Name(n) => self.written_name(n),
                    Ev::ImportPath(s) => self.term_node(s, None),
                    Ev::ImportSel(prefix, n) => self.import_selector(prefix, n),
                    Ev::Forwarder(c, n) => self.forwarder_ref(c, n),
                    Ev::LocalClass(c) => {
                        // Its definition, where `this` is the class records go to.
                        let k = self.owner_class;
                        self.this.extend(k);
                        self.walk_class(c);
                        if k.is_some() {
                            self.this.pop();
                        }
                    }
                }
            }
        }
    }

    /// The `@main` methods of `files`' package blocks, in their order: each has a class of its
    /// own, which scalac's `MainProxies` adds after the block's definitions.
    fn main_methods(&self, file: FileId) -> Vec<SymId> {
        let mut out: Vec<(u32, SymId)> = Vec::new();
        for &(m, object) in self.w.entry_points.iter() {
            let info = self.w.syms.sym(m);
            if object.is_none() && matches!(info.owner, Owner::Package(_)) && self.unit(info.file) == file && info.is_main {
                out.push((info.span.start, m));
            }
        }
        out.sort();
        out.into_iter().map(|(_, m)| m).collect()
    }

    /// The class of a `@main` method: `final class m { def main(args: Array[String]): Unit =
    /// try <file>$package.m() catch { case error: CommandLineParser.ParseError =>
    /// CommandLineParser.showError(error) } }`.
    fn main_classes(&mut self, files: &FxMap<FileId, ()>) {
        let mut wanted: Vec<FileId> = files.keys().copied().collect();
        wanted.sort();
        for f in wanted {
            for m in self.main_methods(f) {
                let (file, p, name) = {
                    let info = self.w.syms.sym(m);
                    let Owner::Package(p) = info.owner else { continue };
                    (info.file, p, info.name)
                };
                self.enter_main(file, p, name);
                let object = self.w.b.any_ref;
                let t = self.w.b.t_any_ref;
                self.tree_type(t);
                self.class_dep(object, Context::Inheritance);
                self.ctor_call(object, t);
                let (unit, string) = (self.w.b.t_unit, self.w.b.t_string);
                self.tree_type(unit);
                let array = self.w.b.array;
                self.named_class(array);
                self.tree_type(string);
                // `<file>$package.m()`.
                self.member_ref(m);
                self.info(m);
                if let Some(module) = self.package_member_module_name(PackageMember::Sym(m), file, p) {
                    self.use_name(module);
                }
                // `case error: CommandLineParser.ParseError => CommandLineParser.showError(error)`.
                self.use_name("error".to_string());
                if let Some((parser_val, parser)) = self.object_named("scala.util.CommandLineParser") {
                    let _ = parser_val;
                    let error = self.w.syms.class(parser).nested.get(&self.w.interner.intern("ParseError")).copied();
                    if let Some(e) = error {
                        self.named_class(e);
                    }
                    let show = self.w.interner.intern("showError");
                    if let Some(s) = self.w.module_term(parser, show).and_then(|r| r.sym()) {
                        self.term_node(s, None);
                    }
                }
            }
        }
    }

    fn enter_main(&mut self, file: FileId, p: PkgId, name: Name) {
        let path = self.w.pkg_path(p);
        let class = if path.is_empty() { self.w.name_str(name) } else { format!("{}.{}", path, self.w.name_str(name)) };
        let unit = self.unit(file);
        self.package_this = None;
        self.owner_class = None;
        self.owner_pkg = Some(p);
        if class != self.owner {
            self.tree_seen = Seen::default();
            self.walked.clear();
            self.walked_pats.clear();
            self.walked_classes.clear();
        }
        self.owner = class.clone();
        self.owner_file = unit;
        self.by_file.entry(unit).or_default().insert(class.clone());
        self.deps.entry(class).or_default();
    }

    /// Makes `owned` the class the records go to.
    fn enter(&mut self, owned: Owned) {
        let (name, file) = match owned {
            Owned::Class(c) => (self.zinc_name(c), self.w.syms.class(c).file),
            Owned::Package(f, p) => (self.package_object_name(f, p), f),
        };
        self.package_this = match owned {
            Owned::Package(..) => Some(format!("{}$", name)),
            Owned::Class(_) => None,
        };
        self.owner_class = match owned {
            Owned::Class(c) => Some(c),
            Owned::Package(..) => None,
        };
        self.owner_pkg = match owned {
            Owned::Package(_, p) => Some(p),
            Owned::Class(c) => {
                let top = self.top_level(c);
                match (self.wrapped.get(&top), self.w.syms.class(top).owner) {
                    (Some(&(_, p)), _) | (None, Owner::Package(p)) => Some(p),
                    _ => None,
                }
            }
        };
        let unit = self.unit(file);
        if name != self.owner {
            self.tree_seen = Seen::default();
            self.walked.clear();
            self.walked_pats.clear();
            self.walked_classes.clear();
        }
        self.owner = name.clone();
        self.owner_file = unit;
        self.by_file.entry(unit).or_default().insert(name.clone());
        self.deps.entry(name).or_default();
    }

    fn here(&mut self) -> &mut ClassDeps {
        self.deps.get_mut(&self.owner).expect("an owner entered")
    }

    // ---- naming ----

    /// The package and the name scalac knows one of teq's builtin classes by: `AnyRef` is
    /// `java.lang.Object`, the builtin `String` `java.lang.String`.
    fn scalac_path(&self, c: ClassId) -> Option<(&'static str, &'static str)> {
        if c == self.w.b.any_ref {
            Some(("java.lang", "Object"))
        } else if c == self.w.b.string {
            Some(("java.lang", "String"))
        } else {
            None
        }
    }

    /// Whether scalac's class for `c` is a module class: an object's, one nested in a class
    /// (teq's inner class with its lazy val), a given's without parameters.
    fn is_module(&self, c: ClassId) -> bool {
        let info = self.w.syms.class(c);
        info.kind == ClassKind::Object || info.inner_object.is_some() || info.local_module.is_some() || self.given_objects.contains_key(&c)
    }

    /// scalac's `fullName` of a class: its owners' names outermost first, an object's class
    /// with its `$`.
    fn full_name(&self, c: ClassId) -> String {
        if let Some((pkg, name)) = self.scalac_path(c) {
            return format!("{}.{}", pkg, name);
        }
        let info = self.w.syms.class(c);
        let own = format!("{}{}", self.w.name_ref(info.name), if self.is_module(c) { "$" } else { "" });
        if let Some(&(f, p)) = self.wrapped.get(&c) {
            return format!("{}$.{}", self.package_object_name(f, p), own);
        }
        match info.owner {
            Owner::Class(o) => format!("{}.{}", self.full_name(o), own),
            Owner::Package(p) => {
                let path = self.w.pkg_path(p);
                if path.is_empty() {
                    own
                } else {
                    format!("{}.{}", path, own)
                }
            }
            Owner::Local => own,
        }
    }

    /// The name zinc keys a class by (`classNameAsString`): `fullName` without an object's `$`.
    fn zinc_name(&self, c: ClassId) -> String {
        let full = self.full_name(c);
        if self.is_module(c) {
            full.strip_suffix('$').map(str::to_string).unwrap_or(full)
        } else {
            full
        }
    }

    /// The name zinc knows the object of a package block's top-level definitions by: scalac's
    /// `p.package` for a `package object p`'s members (`package_object_holder`),
    /// `p.<stem>$package` otherwise.
    fn package_object_name(&self, f: FileId, p: PkgId) -> String {
        if !self.w.package_object_holder(f) {
            return self.package_object_binary(f, p);
        }
        let path = self.w.pkg_path(p);
        if path.is_empty() {
            "package".to_string()
        } else {
            format!("{}.package", path)
        }
    }

    /// `p.<stem>$package`, the binary name of the object of a package block's top-level
    /// definitions, a package object's too.
    fn package_object_binary(&self, f: FileId, p: PkgId) -> String {
        let stem = crate::tasty::write::file_stem(&self.w.source(self.unit(f)).path);
        let path = self.w.pkg_path(p);
        if path.is_empty() {
            format!("{}$package", stem)
        } else {
            format!("{}.{}$package", path, stem)
        }
    }

    fn is_local(&self, mut c: ClassId) -> bool {
        loop {
            match self.w.syms.class(c).owner {
                Owner::Local => return true,
                Owner::Package(_) => return false,
                Owner::Class(o) => c = o,
            }
        }
    }

    // ---- records ----

    fn use_name(&mut self, name: String) {
        self.here().names.entry(name).or_default().typ = true;
    }

    /// A name a type's tree wrote that its type does not hold. `|` and `&` are aliases of the
    /// compiler whose infos are lambdas of two parameters bounded by `Nothing` and `Any`.
    fn written_name(&mut self, n: Name) {
        let name = self.w.name_str(n);
        if name == "|" || name == "&" {
            self.named_builtin("Nothing");
            self.named_builtin("Any");
        }
        self.use_name(name);
    }

    fn use_term_name(&mut self, name: String) {
        self.here().names.entry(name).or_default().term = true;
    }

    fn use_sealed(&mut self, name: String) {
        self.here().names.entry(name).or_default().patmat = true;
    }

    /// A dependency on the class `to` in `ctx`, as `recordClassDependency` sends it: on a class of
    /// the class path as a binary dependency, on one of the build as a class dependency unless
    /// it is a member reference within the same source.
    fn class_dep(&mut self, to: ClassId, ctx: Context) {
        match self.target(to) {
            Target::Source(name, file) => {
                if ctx != Context::MemberRef || file != self.owner_file {
                    self.here().classes.insert((name, ctx));
                }
            }
            Target::Binary(entry, binary) => {
                self.here().binaries.insert((entry, binary, ctx));
            }
            Target::Nowhere => {}
        }
    }

    /// A dependency on the `<file>$package` a top-level definition is a member of.
    fn package_object_dep(&mut self, t: Target) {
        match t {
            Target::Source(name, file) => {
                if file != self.owner_file {
                    self.here().classes.insert((name, Context::MemberRef));
                }
            }
            Target::Binary(entry, binary) => {
                self.here().binaries.insert((entry, binary, Context::MemberRef));
            }
            Target::Nowhere => {}
        }
    }

    /// The `<file>$package` a top-level definition is a member of: of the build's block it is
    /// defined in, or of the TASTy file of the class path that holds it.
    fn package_member_target(&mut self, m: PackageMember, file: FileId, p: PkgId) -> Option<Target> {
        if self.w.program_file(file) {
            return Some(Target::Source(self.package_object_name(file, p), self.unit(file)));
        }
        let loaded = self.w.loaded.as_ref()?;
        let lfile = match m {
            PackageMember::Sym(s) => loaded.syms.get(&s)?.file,
            PackageMember::Alias(a) => loaded.aliases.get(&a)?.file,
        };
        let cp = loaded.files.as_slice().get(lfile as usize)?.cp;
        let name = loaded.cp.entry_name(cp);
        let stem = name.strip_suffix(".tasty")?;
        let binary = format!("{}$", stem.replace('/', "."));
        Some(Target::Binary(self.entry_path(cp), binary))
    }

    /// The name of the module val of the `<file>$package` a top-level definition is a member of,
    /// by its binary name: a pickle does not tell a package object's from another's, so a build
    /// over its products names it as the build of its source does.
    fn package_member_module_name(&mut self, m: PackageMember, file: FileId, p: PkgId) -> Option<String> {
        match self.package_member_target(m, file, p)? {
            Target::Source(..) => {
                let name = self.package_object_binary(file, p);
                Some(name.rsplit('.').next().unwrap_or(&name).to_string())
            }
            Target::Binary(_, binary) => {
                let simple = binary.rsplit('.').next().unwrap_or(&binary);
                Some(simple.strip_suffix('$').unwrap_or(simple).to_string())
            }
            Target::Nowhere => None,
        }
    }

    // ---- where a class depended on is ----

    fn target(&mut self, c: ClassId) -> Target {
        if let Some(t) = self.targets.get(&c) {
            return t.clone();
        }
        let t = self.find_target(c);
        self.targets.insert(c, t.clone());
        t
    }

    fn find_target(&mut self, c: ClassId) -> Target {
        // The object teq makes for a Java class's static members stands for the class.
        let statics_of = self.w.loaded.as_ref().and_then(|l| l.java.classes.get(&c)).and_then(|j| j.companion_of);
        if let Some(k) = statics_of {
            return self.target(k);
        }
        if let Some(k) = self.w.syms.class(c).companion.filter(|&k| self.is_module(c) && self.w.is_java_class(k)) {
            return self.target(k);
        }
        let info = self.w.syms.class(c);
        let file = info.file;
        if self.w.program_file(file) && info.def.is_some() {
            return Target::Source(self.zinc_name(self.outermost_nonlocal(c)), self.unit(file));
        }
        let mut binary = self.binary_class_name(c);
        // The statics of a Java class that teq's std writes as an object: scalac's module of a
        // Java class is the class.
        if self.is_module(c) && binary.ends_with('$') && self.in_java_package(c) && self.classpath_entry(c).is_none() {
            binary.pop();
        }
        // A class read from the class path: the file its top-level class came from.
        if let Some(entry) = self.classpath_entry(c) {
            return Target::Binary(entry, binary);
        }
        // A class of teq's own standard library (its builtin layer under `--std=scala-library`):
        // the class file scalac finds for it, the JDK's or the class path's.
        self.std_origin(c, &binary).map_or(Target::Nowhere, |entry| Target::Binary(entry, binary))
    }

    /// The class a local one belongs to for zinc: the closest non-local class around it.
    fn outermost_nonlocal(&self, c: ClassId) -> ClassId {
        c
    }

    /// The binary name of a class (`p.Outer$Inner`, `p.Obj$`).
    fn binary_class_name(&self, c: ClassId) -> String {
        if let Some((pkg, name)) = self.scalac_path(c) {
            return format!("{}.{}", pkg, name);
        }
        let info = self.w.syms.class(c);
        let own = format!("{}{}", crate::jvm::names::encode(self.w.name_ref(info.name)), if self.is_module(c) { "$" } else { "" });
        if let Some(&(f, p)) = self.wrapped.get(&c) {
            return format!("{}${}", self.package_object_binary(f, p), own);
        }
        match info.owner {
            Owner::Class(o) => {
                let outer = self.binary_class_name(o);
                if outer.ends_with('$') {
                    format!("{}{}", outer, own)
                } else {
                    format!("{}${}", outer, own)
                }
            }
            Owner::Package(p) => {
                let path: Vec<String> = self.w.pkg_path(p).split('.').filter(|s| !s.is_empty()).map(crate::jvm::names::encode).collect();
                if path.is_empty() {
                    own
                } else {
                    format!("{}.{}", path.join("."), own)
                }
            }
            Owner::Local => own,
        }
    }

    fn in_java_package(&self, c: ClassId) -> bool {
        let Owner::Package(p) = self.w.syms.class(self.top_level(c)).owner else { return false };
        let path = self.w.pkg_path(p);
        path == "java" || path.starts_with("java.")
    }

    /// The top-level class a nested one is defined in.
    fn top_level(&self, mut c: ClassId) -> ClassId {
        while let Owner::Class(o) = self.w.syms.class(c).owner {
            c = o;
        }
        c
    }

    /// The binary entry of a class read from the class path: its jar, or the class file of its
    /// top-level class in a directory (beside the `.tasty` scalac's class path found).
    fn classpath_entry(&mut self, c: ClassId) -> Option<String> {
        let top = self.top_level(c);
        let loaded = self.w.loaded.as_ref()?;
        // A Java class is read from a class file of its own, a nested one's too.
        let java = loaded.java.classes.get(&c).or_else(|| loaded.java.classes.get(&top)).map(|j| j.source);
        let own = if loaded.java.classes.contains_key(&c) { c } else { top };
        let cp = if let Some(l) = loaded.classes.get(&top) {
            loaded.files.as_slice().get(l.file as usize)?.cp
        } else if let Some(source) = java {
            match source {
                super::loader::javaclass::JSource::Cp(f) => f,
                super::loader::javaclass::JSource::Jdk(entry) => return self.w.jdk_runtime_path(entry).map(|p| format!("jrt:{}", p)),
                super::loader::javaclass::JSource::Embedded => return Some(format!("jrt:/modules/java.base/{}.class", self.binary_class_name(own).replace('.', "/"))),
            }
        } else {
            return None;
        };
        Some(self.entry_path(cp))
    }

    fn entry_path(&mut self, f: crate::classpath::CpFile) -> String {
        let cp = &self.w.loaded.as_ref().expect("a class path").cp;
        let root = cp.paths[f.jar as usize].clone();
        if !cp.in_directory(f) {
            return root;
        }
        let name = cp.entry_name(f).to_string();
        let class = name.strip_suffix(".tasty").map(|s| format!("{}.class", s)).unwrap_or_else(|| name.clone());
        let path = format!("{}/{}", root.trim_end_matches('/'), class);
        // A class of the own directory, or one without a class file (a TASTy-only directory's, an
        // alias-only `$package`): its `.tasty`, which zinc finds the class by through the
        // analysis, by its binary name (docs/TARGETS.md, "The contract between the plugin and the
        // compiler").
        if name.ends_with(".tasty") && (cp.is_own_entry(f.jar as usize) || !std::path::Path::new(&path).exists()) {
            return format!("{}/{}", root.trim_end_matches('/'), name);
        }
        path
    }

    /// Where scalac finds a class that teq's builtin layer defines: a `java.` class in the JDK's
    /// runtime image, a `scala.` one in the class path's jar that holds it.
    fn std_origin(&mut self, c: ClassId, binary: &str) -> Option<String> {
        let top = self.top_level(c);
        let simple = binary.rsplit('.').next().unwrap_or(binary).to_string();
        if self.scalac_path(c).is_some() {
            return Some(format!("jrt:/modules/java.base/{}.class", binary.replace('.', "/")));
        }
        let Owner::Package(p) = self.w.syms.class(top).owner else { return None };
        let path = self.w.pkg_path(p);
        if path.starts_with("java.") || path == "java" {
            // The module the JDK's `ct.sym` names, where the typing opened it; `java.base`, which
            // holds every package of teq's builtin layer, where it did not.
            return Some(match self.w.jdk_runtime_path_of(p, &simple) {
                Some(p) => format!("jrt:{}", p),
                None => format!("jrt:/modules/java.base/{}.class", binary.replace('.', "/")),
            });
        }
        let file_path = binary.replace('.', "/");
        let top_path = self.binary_class_name(top).replace('.', "/");
        let loaded = self.w.loaded.as_ref()?;
        let f = loaded.cp.tasty_named(top_path.trim_end_matches('$')).or_else(|| loaded.cp.class_named(&file_path))?;
        let _ = file_path;
        Some(self.entry_path(f))
    }

    // ---- scalac's `addMemberRefDependency` ----

    /// A reference to the term `s`: its name, and a dependency on the class that holds it (an
    /// object's own class for the object).
    fn member_ref(&mut self, s: SymId) {
        let info = self.w.syms.sym(s);
        let (name, owner, kind, file) = (info.name, info.owner, info.kind, info.file);
        if self.ignored_sym(s) {
            return;
        }
        let used = match self.accessor_name(s) {
            Some(accessor) => accessor,
            None => self.zinc_mangled(s),
        };
        self.use_term_name(used);
        let given_object = self.w.syms.sym(s).impl_class.filter(|c| self.given_objects.contains_key(c));
        match (kind, given_object) {
            (SymKind::Object(c), _) | (_, Some(c)) => self.class_dep(c, Context::MemberRef),
            _ => match owner {
                Owner::Class(c) => self.class_dep(c, Context::MemberRef),
                Owner::Package(p) => {
                    if let Some(t) = self.package_member_target(PackageMember::Sym(s), file, p) {
                        self.package_object_dep(t);
                    }
                }
                Owner::Local => {}
            },
        }
        let _ = name;
    }

    /// A reference to the class `c` as a named type: its name and a dependency on it.
    fn class_ref(&mut self, c: ClassId) {
        if self.ignored_class(c) {
            return;
        }
        let name = match self.scalac_path(c) {
            Some((_, name)) => name.to_string(),
            None => self.w.name_str(self.w.syms.class(c).name),
        };
        self.use_name(name);
        self.class_dep(c, Context::MemberRef);
    }

    fn package_ref(&mut self, p: PkgId) {
        if p == ROOT_PKG {
            return;
        }
        let name = self.package_name(p);
        self.use_term_name(name);
    }

    /// A package's name as scalac's symbol has it: a package of the build's sources as they
    /// write it, one only the class path holds as its directory is named (`$plus`).
    fn package_name(&mut self, p: PkgId) -> String {
        let name = self.w.name_str(self.w.syms.pkg(p).name);
        if self.source_packages.is_none() {
            let mut own: FxMap<PkgId, ()> = FxMap::default();
            for f in 0..self.w.files.len() {
                let file = FileId(f as u32);
                if !self.w.program_file(file) {
                    continue;
                }
                let mut q = ROOT_PKG;
                for &seg in &self.w.ast(file).package {
                    match self.w.syms.pkg(q).entries.get(&seg).and_then(|e| e.pkg) {
                        Some(sub) => {
                            own.insert(sub, ());
                            q = sub;
                        }
                        None => break,
                    }
                }
            }
            self.source_packages = Some(own);
        }
        if self.source_packages.as_ref().map_or(false, |own| own.contains_key(&p)) {
            name
        } else {
            crate::jvm::names::encode(&name)
        }
    }

    /// `zincMangledName`: a constructor as its class's full name with `;` for `.` and `;init;`
    /// after it, anything else by its name, an object's without the `$`.
    fn zinc_mangled(&self, s: SymId) -> String {
        let info = self.w.syms.sym(s);
        if info.name == crate::names::INIT {
            if let Owner::Class(c) = info.owner {
                return format!("{};init;", self.mangled_full_name(c).replace('.', ";"));
            }
        }
        self.w.name_str(info.name)
    }

    /// `fullName.mangledString`: the class's full name with each part encoded as its class
    /// file's name is (`$plus` for `+`).
    fn mangled_full_name(&self, c: ClassId) -> String {
        if let Some((pkg, name)) = self.scalac_path(c) {
            return format!("{}.{}", pkg, name);
        }
        let info = self.w.syms.class(c);
        let own = format!("{}{}", crate::jvm::names::encode(self.w.name_ref(info.name)), if self.is_module(c) { "$" } else { "" });
        if let Some(&(f, p)) = self.wrapped.get(&c) {
            return format!("{}$.{}", self.package_object_name(f, p), own);
        }
        match info.owner {
            Owner::Class(o) => format!("{}.{}", self.mangled_full_name(o), own),
            Owner::Package(p) => {
                let path = self.w.pkg_path(p);
                let path: Vec<String> = path.split('.').filter(|s| !s.is_empty()).map(crate::jvm::names::encode).collect();
                if path.is_empty() {
                    own
                } else {
                    format!("{}.{}", path.join("."), own)
                }
            }
            Owner::Local => own,
        }
    }

    /// A local the typer made for its own lowering (a temporary of named arguments or of an
    /// operand, an eta-expansion's parameter, a placeholder's): scalac's desugarings make
    /// theirs with other names (`$1$`, `x$1`, `_$1`), and neither names a member.
    fn synthetic_local(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        info.owner == Owner::Local && self.w.name_ref(info.name).contains('$')
    }

    /// `ignoreDependency`: a symbol that does not exist, an anonymous function or class.
    fn ignored_sym(&self, s: SymId) -> bool {
        let info = self.w.syms.sym(s);
        if matches!(info.kind, SymKind::Overloaded(_)) {
            return true;
        }
        let name = self.w.name_ref(info.name);
        name.starts_with("$anonfun")
    }

    fn ignored_class(&self, c: ClassId) -> bool {
        self.w.syms.class(c).kind == ClassKind::Anon
    }

    // ---- scalac's `TypeDependencyTraverser` ----

    /// Every symbol a type names, with its info where it is no class, and its prefix.
    fn type_deps(&mut self, t: TypeId) {
        if t == ERROR || self.seen().types.insert(t, ()).is_some() {
            return;
        }
        let t = if self.w.types.has_vars(t) { self.w.zonk(t) } else { t };
        match self.w.types.get(t) {
            Type::Any => self.named_builtin("Any"),
            Type::Nothing => self.named_builtin("Nothing"),
            Type::Error | Type::Blocked(_) => {}
            Type::Wild => {
                self.named_builtin("Nothing");
                self.named_builtin("Any");
            }
            Type::BoundedWild(lo, hi) => {
                self.type_deps(lo);
                self.type_deps(hi);
            }
            Type::Class(c, args) => {
                self.named_class(c);
                for &a in self.w.types.items(args).to_vec().iter() {
                    self.type_deps(a);
                }
            }
            Type::Ctor(c) => self.named_class(c),
            Type::Param(p) => self.named_tparam(p),
            Type::AppParam(p, args) => {
                self.named_tparam(p);
                for &a in self.w.types.items(args).to_vec().iter() {
                    self.type_deps(a);
                }
            }
            Type::Lambda(params, body) | Type::Poly(params, body) => {
                let ps: Vec<TypeId> = self.w.types.items(params).to_vec();
                let mut bound = Vec::new();
                for &p in &ps {
                    if let Type::Param(id) = self.w.types.get(p) {
                        bound.push(id);
                    }
                }
                let depth = self.bind(&bound);
                for &id in &bound {
                    let (lo, hi) = (self.w.syms.tparam(id).lower, self.w.syms.tparam(id).upper);
                    self.type_deps(lo);
                    self.type_deps(hi);
                }
                self.type_deps(body);
                self.bound.truncate(depth);
            }
            Type::Var(_) | Type::AppVar(..) => {}
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.type_deps(a);
                self.type_deps(b);
            }
            Type::Lit(l) => {
                let class = match self.w.types.lit_val(l) {
                    LitVal::Int(_) => self.w.b.t_int,
                    LitVal::Long(_) => self.w.b.t_long,
                    LitVal::Double(_) => self.w.b.t_double,
                    LitVal::Char(_) => self.w.b.t_char,
                    LitVal::Bool(_) => self.w.b.t_boolean,
                    LitVal::Str(_) => self.w.b.t_string,
                };
                self.type_deps(class);
            }
            Type::This(c) => {
                let self_type = self.w.syms.this_type(c);
                self.type_deps(self_type);
                self.opaque_refinement(c);
            }
            Type::Term(s) => match self.bound_terms.iter().rev().find(|&&(p, _)| p == s).map(|&(_, t)| t) {
                Some(t) => self.type_deps(t),
                None => self.named_term(s, None),
            },
            Type::Select(prefix, s) => self.named_term(s, Some(prefix)),
            Type::Member(prefix, name) => {
                if let Some(a) = self.member_alias(prefix, name) {
                    self.named_alias(a);
                }
                self.type_deps(prefix);
            }
            Type::AppMember(m, args) => {
                self.type_deps(m);
                for &a in self.w.types.items(args).to_vec().iter() {
                    self.type_deps(a);
                }
            }
            Type::Decl(a) => self.named_alias(a),
            Type::Alias(a, args) => {
                self.named_alias(a);
                for &x in self.w.types.items(args).to_vec().iter() {
                    self.type_deps(x);
                }
            }
            Type::Refined(parent, r) => {
                self.type_deps(parent);
                match self.w.types.refinement(r) {
                    Refinement::Alias(_, t) => self.type_deps(t),
                    Refinement::Bounds(_, lo, hi) => {
                        self.type_deps(lo);
                        self.type_deps(hi);
                    }
                    Refinement::Term(_, _, list) => {
                        for &x in self.w.types.items(list).to_vec().iter() {
                            self.type_deps(x);
                        }
                    }
                    Refinement::Val(_, _, t) => self.type_deps(t),
                }
            }
            Type::Match(scrutinee, m) => {
                self.type_deps(scrutinee);
                let info = self.w.types.match_info(m).clone();
                self.type_deps(info.bound);
                for case in info.cases.iter() {
                    self.type_deps(case.pattern);
                    self.type_deps(case.body);
                }
            }
        }
    }

    /// What the traversal under way has met: the tree's memo, or with a type parameter bound
    /// the info's.
    fn seen(&mut self) -> &mut Seen {
        if self.bound.is_empty() {
            &mut self.tree_seen
        } else {
            &mut self.info_seen
        }
    }

    /// Binds `ps` for the traversal of an info, a lambda or an alias's right side; the first
    /// binding starts the info's memo afresh.
    fn bind(&mut self, ps: &[TParamId]) -> usize {
        let depth = self.bound.len();
        if depth == 0 && !ps.is_empty() {
            self.info_seen = Seen::default();
        }
        self.bound.extend_from_slice(ps);
        depth
    }

    /// A class scalac's types name that no file holds (`Any`, `Nothing`): its name alone.
    fn named_builtin(&mut self, name: &str) {
        self.use_name(name.to_string());
    }

    /// The class `c` as a type names it: the class and its prefix (the object it is a member of,
    /// or the enclosing class's `this`).
    fn named_class(&mut self, c: ClassId) {
        if self.seen().classes.insert(c, ()).is_some() {
            return;
        }
        if self.w.syms.class(c).kind == ClassKind::Opaque {
            // An opaque type is an alias of the class or the package it is in.
            let (name, owner, file) = {
                let info = self.w.syms.class(c);
                (info.name, info.owner, info.file)
            };
            self.use_name(self.w.name_str(name));
            match owner {
                Owner::Class(o) => self.class_dep(o, Context::MemberRef),
                Owner::Package(p) => {
                    let t = Target::Source(self.package_object_name(file, p), self.unit(file));
                    if self.w.program_file(file) {
                        self.package_object_dep(t);
                    }
                }
                Owner::Local => {}
            }
            // Its info outside its scope: bounds, `Nothing` below and what it is written under
            // (`Any` unwritten) above.
            let within = match owner {
                Owner::Class(o) => self.lexically_within(o),
                Owner::Package(p) => self.owner_class.is_none() && self.owner_pkg == Some(p) && self.owner == self.package_object_name(file, p),
                Owner::Local => true,
            };
            if !within {
                self.named_builtin("Nothing");
                match self.w.syms.class(c).parents.first().copied() {
                    Some(bound) => self.type_deps(bound),
                    None => self.named_builtin("Any"),
                }
            }
            self.class_prefix(c);
            return;
        }
        self.class_ref(c);
        self.class_prefix(c);
    }

    fn class_prefix(&mut self, c: ClassId) {
        match self.w.syms.class(c).owner {
            Owner::Class(o) if self.w.syms.class(o).kind == ClassKind::Object => {
                // `O.C`: the object `O`, a module val whose info is its class.
                if let Some(m) = self.w.syms.class(o).module_sym {
                    if self.seen().syms.insert(m, ()).is_none() {
                        self.member_ref(m);
                    }
                }
                self.named_class(o);
                // Inside `O` it is `O.this.C`.
                if self.lexically_within(o) {
                    self.opaque_refinement(o);
                }
            }
            Owner::Class(o) => {
                // `O.this.C`: the enclosing class's `this`, whose underlying type is its own.
                let self_type = self.w.syms.this_type(o);
                self.type_deps(self_type);
            }
            _ => {}
        }
    }

    /// A type parameter as a type names it: a class's, with its bounds and the class's `this`;
    /// a method's own in the tree of its definition likewise; a method's bound by the signature
    /// being traversed only by its bounds.
    /// The class a type parameter is of: the map made at the start, then the classes completed
    /// since.
    fn tparam_owner(&mut self, p: TParamId) -> Option<ClassId> {
        if let Some(&c) = self.tparam_class.get(&p) {
            return c;
        }
        let found = self.w.syms.classes.iter().position(|info| info.tparams.contains(&p)).map(|i| ClassId(i as u32));
        self.tparam_class.insert(p, found);
        found
    }

    fn named_tparam(&mut self, p: TParamId) {
        if self.seen().tparams.insert(p, ()).is_some() {
            return;
        }
        let _ = self.tparam_owner(p);
        let (lo, hi, name) = {
            let info = self.w.syms.tparam(p);
            (info.lower, info.upper, info.name)
        };
        // One bound by the type being traversed had its bounds traversed with it, as the
        // prefix sees them.
        if self.bound.contains(&p) {
            return;
        }
        self.use_name(self.w.name_str(name));
        if let Some(c) = self.tparam_owner(p) {
            self.class_dep(c, Context::MemberRef);
        }
        self.type_deps(lo);
        self.type_deps(hi);
        if let Some(c) = self.tparam_owner(p) {
            let self_type = self.w.syms.this_type(c);
            self.type_deps(self_type);
        }
    }

    /// A term a type names (`x.type`, `p.x.type`): the term, its info and the prefix.
    fn named_term(&mut self, s: SymId, prefix: Option<TypeId>) {
        if self.seen().syms.insert(s, ()).is_none() {
            self.member_ref(s);
            self.info(s);
        }
        if let Some(p) = prefix {
            self.type_deps(p);
        } else {
            self.term_prefix(s);
        }
    }

    /// The prefix of a term a type or a tree names without one: the object or the package
    /// object it is a member of.
    fn term_prefix(&mut self, s: SymId) {
        let info = self.w.syms.sym(s);
        match info.owner {
            Owner::Class(o) if self.w.syms.class(o).kind == ClassKind::Object => {
                if let Some(m) = self.w.syms.class(o).module_sym {
                    if m != s && self.seen().syms.insert(m, ()).is_none() {
                        self.member_ref(m);
                    }
                }
                self.named_class(o);
            }
            Owner::Class(o) => {
                let self_type = self.w.syms.this_type(o);
                self.type_deps(self_type);
            }
            Owner::Package(p) => {
                // `<file>$package.m`: the typer writes the module val of the member's
                // `<file>$package` from anywhere outside it. An object is the package's own.
                let file = info.file;
                if matches!(info.kind, SymKind::Object(_)) {
                    return;
                }
                if let Some(name) = self.package_member_module_name(PackageMember::Sym(s), file, p) {
                    self.use_name(name);
                    if let Some(t) = self.package_member_target(PackageMember::Sym(s), file, p) {
                        self.package_object_dep(t);
                    }
                }
            }
            Owner::Local => {}
        }
    }

    /// An alias as a type names it: the alias, what it stands for and its prefix.
    fn named_alias(&mut self, a: AliasId) {
        if self.seen().aliases.insert(a, ()).is_some() {
            return;
        }
        let (name, owner, file, rhs, bounds, tparams) = {
            let info = self.w.syms.alias(a);
            (info.name, info.owner, info.file, info.rhs, info.bounds, info.tparams.clone())
        };
        self.use_name(self.w.name_str(name));
        match owner {
            Owner::Class(c) => self.class_dep(c, Context::MemberRef),
            Owner::Package(p) => {
                // Its prefix, the `<file>$package` module val.
                if !self.via_package_path {
                    if let Some(name) = self.package_member_module_name(PackageMember::Alias(a), file, p) {
                        self.use_name(name);
                    }
                }
                if let Some(t) = self.package_member_target(PackageMember::Alias(a), file, p) {
                    self.package_object_dep(t);
                }
            }
            Owner::Local => {}
        }
        // The info of an alias with parameters is a type lambda, which names them by position.
        let depth = self.bind(&tparams);
        for &p in &tparams {
            let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
            self.type_deps(lo);
            self.type_deps(hi);
        }
        match bounds {
            Some((lo, hi)) => {
                self.type_deps(lo);
                self.type_deps(hi);
            }
            None => self.type_deps(rhs),
        }
        self.bound.truncate(depth);
        if let Owner::Class(c) = owner {
            self.named_class(c);
        }
    }

    /// The alias `prefix.name` names: a member of the prefix's class, or the original of a
    /// forwarder the class exports it by.
    fn member_alias(&mut self, prefix: TypeId, name: Name) -> Option<AliasId> {
        let widened = match self.w.types.get(prefix) {
            Type::Term(s) => self.w.syms.sym(s).sig.as_ref().map(|sig| sig.ret),
            _ => Some(prefix),
        }?;
        let c = match self.w.types.get(widened) {
            Type::Class(c, _) | Type::This(c) => c,
            _ => return None,
        };
        if let Some(&a) = self.w.syms.class(c).type_aliases.get(&name) {
            return Some(a);
        }
        if !self.w.syms.class(c).has_exports {
            return None;
        }
        match self.w.exports_of(c)?.types.get(&name) {
            Some(&super::resolve::TypeRef::Alias(a)) => Some(a),
            _ => None,
        }
    }

    /// A term's info: its signature's types, its own type parameters named by position.
    fn info(&mut self, s: SymId) {
        self.info_seen_from(s, None);
    }

    /// A term's info as the prefix `prefix` sees it: the type parameters of the class that
    /// declares it replaced by the prefix's arguments for them.
    fn info_seen_from(&mut self, s: SymId, prefix: Option<TypeId>) {
        let sig = match self.w.syms.sym(s).sig.clone() {
            Some(sig) => sig,
            // A member the collection names that no typing completed (a library's `apply` a
            // case class's construction stands for): its signature read now.
            None if !matches!(self.w.syms.sym(s).kind, SymKind::Overloaded(_)) => self.w.sig_arc(s),
            None => return,
        };
        let subst = match (prefix, self.w.syms.sym(s).owner) {
            (Some(p), Owner::Class(o)) => self.subst_from(p, o),
            _ => Vec::new(),
        };
        let seen = |c: &mut Self, t: TypeId| if subst.is_empty() { t } else { c.w.types.subst(t, &subst) };
        let depth = self.bind(&sig.tparams);
        for &p in &sig.tparams {
            let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
            let (lo, hi) = (seen(self, lo), seen(self, hi));
            self.type_deps(lo);
            self.type_deps(hi);
        }
        let terms = self.bound_terms.len();
        for clause in &sig.clauses {
            for param in &clause.params {
                let t = seen(self, param.ty);
                self.param_type(t, param.repeated);
                self.bound_terms.push((param.sym, t));
            }
        }
        let ret = seen(self, sig.ret);
        self.type_deps(ret);
        self.bound_terms.truncate(terms);
        self.bound.truncate(depth);
        // The aliases the signature's written types name, which its types hold expanded.
        self.replay_signature(s);
    }

    /// The type parameters of `owner` with the arguments the base type of `prefix` gives them.
    fn subst_from(&mut self, prefix: TypeId, owner: ClassId) -> Subst {
        let prefix = if self.w.types.has_vars(prefix) { self.w.zonk(prefix) } else { prefix };
        let prefix = match self.w.types.get(prefix) {
            Type::This(c) => self.w.syms.this_type(c),
            _ => prefix,
        };
        let Some(bt) = self.w.base_type(prefix, owner) else { return Vec::new() };
        let Type::Class(_, args) = self.w.types.get(bt) else { return Vec::new() };
        let args = self.w.types.items(args).to_vec();
        let tparams = self.w.syms.class(owner).tparams.clone();
        if args.len() != tparams.len() {
            return Vec::new();
        }
        tparams.into_iter().zip(args).collect()
    }

    // ---- the trees of definitions ----

    /// A class's template: its type parameters, constructor, parents, self type and members.
    fn template(&mut self, c: ClassId) {
        let mods = self.w.syms.class(c).mods;
        let (tparams, parents, ctor, kind, declared_self, own_members, aliases) = {
            let info = self.w.syms.class(c);
            (
                info.tparams[info.outer_tparams as usize..].to_vec(),
                info.parents.clone(),
                info.ctor.clone(),
                info.kind,
                info.declared_self,
                info.member_order.clone(),
                info.type_aliases.values().copied().collect::<Vec<_>>(),
            )
        };
        for p in tparams {
            self.tparam_def(p);
        }
        // The primary constructor: its parameters' types and its result, `Unit`.
        for clause in &ctor {
            for param in &clause.params {
                if param.repeated {
                    self.repeated_param_def(param.ty);
                } else {
                    self.tree_type(param.ty);
                }
            }
        }
        let unit = self.w.b.t_unit;
        self.tree_type(unit);
        self.parents(c, kind, mods, &parents);
        if kind != ClassKind::Object {
            if let Some(t) = declared_self {
                self.tree_type(t);
            }
        }
        for a in aliases {
            if self.w.syms.alias(a).def.is_some() && self.w.syms.alias(a).owner == Owner::Class(c) {
                self.alias_def(a);
            }
        }
        let opaques: Vec<ClassId> = self.w.syms.class(c).nested.values().copied().filter(|&k| self.w.syms.class(k).kind == ClassKind::Opaque).collect();
        for k in opaques {
            self.opaque_def(k);
        }
        for s in own_members {
            if self.w.syms.sym(s).def.is_some() && !self.is_module_val(s) {
                self.member(s);
                self.inline_definition(s);
            }
        }
        if mods & crate::ast::mods::CASE != 0 && kind == ClassKind::Class || kind == ClassKind::EnumCase && self.w.syms.class(c).singleton.is_none() {
            self.case_class_members(c);
            if kind == ClassKind::EnumCase {
                // `def ordinal: Int = n`.
                let int = self.w.b.t_int;
                self.tree_type(int);
            }
        } else if self.w.syms.class(c).companion.map_or(true, |k| self.w.syms.class(k).def.is_none())
            && (self.w.syms.class(c).value_class || kind == ClassKind::Class && ctor.iter().any(|cl| cl.params.iter().any(|p| p.has_default)))
        {
            // The companion scalac's desugaring makes, `object C extends AnyRef`: a value
            // class's, and a class's whose constructor's defaults are its members.
            self.made_companion_template();
        }
        if self.w.syms.class(c).has_exports {
            self.export_forwarders(c);
        }
        if kind == ClassKind::Enum && !self.is_local(c) {
            self.enum_desugaring(c);
        }
        // The bodies the typer kept.
        let tc = self.tclass_of.get(&c).copied();
        if let Some(i) = tc {
            self.this.push(c);
            self.class_bodies(i);
            self.this.pop();
        }
    }

    /// What scalac's desugaring writes for a case class and teq's typer synthesizes without a
    /// tree: `copy` and its default getters, the `_N` accessors, and in the companion `apply`,
    /// `unapply` and, where the companion is made, `toString` and its template.
    fn case_class_members(&mut self, c: ClassId) {
        let (ctor, companion) = {
            let info = self.w.syms.class(c);
            (info.ctor_syms.clone(), info.companion)
        };
        let this_type = self.w.syms.this_type(c);
        // `copy` and `apply`: `new C[T](params)`.
        self.ctor_call(c, this_type);
        for clause in &ctor {
            for &p in clause {
                let name = self.w.name_str(self.w.syms.sym(p).name);
                self.use_name(name);
            }
        }
        // `copy$default$N` and `_N`: `C.this.p`.
        self.type_deps(this_type);
        let made = companion.map_or(true, |k| self.w.syms.class(k).def.is_none());
        {
            // `unapply(x$1: C): C = x$1`.
            self.use_name("x$1".to_string());
            if made {
                // `object C extends AnyRef`, its `toString: String = "C"`.
                self.made_companion_template();
                let string = self.w.b.t_string;
                self.tree_type(string);
            }
        }
    }

    /// The forwarders scalac's namer writes for a class's `export` clauses, which teq's typer
    /// resolves as aliases and makes no definition of: per exported term `final def m(ps): R =
    /// P.m(ps)` (the member through the path `P`, its info seen from `P`, the parameters' names),
    /// per exported type `final type T = P.T`; the path itself; and for a wildcard a dependency
    /// by inheritance on the class of `P`, which makes new members of `P` reach the class.
    fn export_forwarders(&mut self, c: ClassId) {
        let (file, def) = {
            let info = self.w.syms.class(c);
            (info.file, info.def)
        };
        let Some(d) = def else { return };
        let ast = self.w.ast(file);
        let crate::ast::DefKind::Class(cls) = &ast.def(d).kind else { return };
        let clauses: Vec<(Vec<Name>, crate::ast::ImportSel)> = ast.exports[cls.exports.range()].iter().map(|i| (i.path.clone(), i.sel.clone())).collect();
        let mut objects: Vec<ClassId> = Vec::new();
        let mut packages: Vec<PkgId> = Vec::new();
        for (path, sel) in &clauses {
            let o = match self.w.export_qualifier(c, path) {
                Some(super::exports::ExportQualifier::Object(o)) => o,
                Some(super::exports::ExportQualifier::Package(p)) => {
                    // The path: each package a reference.
                    for &seg in path {
                        self.use_name(self.w.name_str(seg));
                    }
                    if !packages.contains(&p) {
                        packages.push(p);
                    }
                    continue;
                }
                None => continue,
            };
            // The path: the object, each segment a reference.
            if let Some(m) = self.w.syms.class(o).module_sym {
                self.term_node(m, None);
            }
            if matches!(sel, crate::ast::ImportSel::Wildcard) {
                self.class_dep(o, if self.is_local(c) { Context::LocalInheritance } else { Context::Inheritance });
            }
            if !objects.contains(&o) {
                objects.push(o);
            }
        }
        // An export of what the qualifier exports itself: its forwarder of the name, which the
        // forwarder of `c` calls, then the original's info.
        for (path, sel) in &clauses {
            let Some(super::exports::ExportQualifier::Object(o)) = self.w.export_qualifier(c, path) else { continue };
            if !self.w.syms.class(o).has_exports {
                continue;
            }
            let Some(qualified) = self.w.exports_of(o) else { continue };
            let mut names: Vec<Name> = match sel {
                crate::ast::ImportSel::Name(n, _) => vec![*n],
                crate::ast::ImportSel::Wildcard => qualified.terms.keys().chain(qualified.types.keys()).copied().collect(),
                _ => Vec::new(),
            };
            names.sort_by_key(|&n| self.w.name_str(n));
            names.dedup();
            for n in names {
                if !self.exported_by(o, n) {
                    continue;
                }
                if let Some(s) = qualified.terms.get(&n).and_then(|r| r.sym()) {
                    self.use_term_name(self.w.name_str(n));
                    self.class_dep(o, Context::MemberRef);
                    if !matches!(self.w.syms.sym(s).kind, SymKind::Object(_)) {
                        self.info(s);
                        self.forwarder_params(s, None, &Vec::new());
                    }
                }
                if let Some(&r) = qualified.types.get(&n) {
                    self.use_name(self.w.name_str(n));
                    self.class_dep(o, Context::MemberRef);
                    match r {
                        super::resolve::TypeRef::Alias(a) => self.named_alias(a),
                        super::resolve::TypeRef::Class(k) => self.named_class(k),
                        _ => {}
                    }
                }
            }
        }
        let Some(scope) = self.w.exports_of(c) else { return };
        let mut terms: Vec<(String, SymId)> = scope.terms.iter().filter_map(|(&n, r)| Some((self.w.name_str(n), r.sym()?))).collect();
        for (&n, ss) in scope.extensions.iter() {
            for &s in ss {
                terms.push((self.w.name_str(n), s));
            }
        }
        terms.sort();
        terms.dedup();
        let bases: Vec<ClassId> = self.w.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        for (_, s) in terms {
            let owner = match self.w.syms.sym(s).owner {
                Owner::Class(owner) => owner,
                Owner::Package(p) if packages.contains(&p) => {
                    // A package's member: its forwarder's info, the member's.
                    self.member_ref(s);
                    if !matches!(self.w.syms.sym(s).kind, SymKind::Object(_)) {
                        self.info(s);
                        self.forwarder_params(s, None, &Vec::new());
                    }
                    continue;
                }
                _ => continue,
            };
            if owner == c || bases.contains(&owner) {
                continue;
            }
            // The object the member is reached through.
            let via = objects.iter().copied().find(|&o| o == owner || self.w.syms.class(o).base_types.iter().any(|&(b, _)| b == owner));
            let Some(o) = via else { continue };
            let prefix = self.w.syms.this_type(o);
            self.member_ref(s);
            self.info_seen_from(s, Some(prefix));
            let subst = self.subst_from(prefix, owner);
            if !subst.is_empty() {
                // The arguments the object's parents give, as written.
                self.replay(Comp::Class(o));
            }
            self.forwarder_params(s, Some(prefix), &subst);
        }
        let mut types: Vec<(Name, super::resolve::TypeRef)> = scope.types.iter().map(|(&n, &r)| (n, r)).collect();
        types.sort_by_key(|&(n, _)| self.w.name_str(n));
        for (_, r) in types {
            let owner = match r {
                super::resolve::TypeRef::Class(k) => self.w.syms.class(k).owner,
                super::resolve::TypeRef::Alias(a) => self.w.syms.alias(a).owner,
                _ => continue,
            };
            let exported = match owner {
                Owner::Class(owner) => owner != c && !bases.contains(&owner) && objects.contains(&owner),
                Owner::Package(p) => packages.contains(&p),
                Owner::Local => false,
            };
            if !exported {
                continue;
            }
            // A forwarder's prefix is the export's path, a package's own rather than its
            // `<file>$package`.
            self.via_package_path = matches!(owner, Owner::Package(_));
            match r {
                super::resolve::TypeRef::Class(k) => self.named_class(k),
                super::resolve::TypeRef::Alias(a) => self.named_alias(a),
                _ => {}
            }
            self.via_package_path = false;
        }
    }

    /// A forwarder's definition: its parameters, by name and type, and its result, the
    /// member's as the export's path sees them.
    fn forwarder_params(&mut self, s: SymId, prefix: Option<TypeId>, subst: &Subst) {
        let _ = prefix;
        let saved = std::mem::take(&mut self.bound);
        if let Some(sig) = self.w.syms.sym(s).sig.clone() {
            // The forwarder's own type parameters, bounded as the path sees the original's.
            for &p in &sig.tparams {
                if self.seen().tparams.insert(p, ()).is_some() {
                    continue;
                }
                let (lo, hi, name) = {
                    let info = self.w.syms.tparam(p);
                    (info.lower, info.upper, info.name)
                };
                self.use_name(self.w.name_str(name));
                for t in [lo, hi] {
                    let t = if subst.is_empty() { t } else { self.w.types.subst(t, subst) };
                    self.type_deps(t);
                }
            }
            for clause in &sig.clauses {
                for param in &clause.params {
                    let name = self.w.name_str(param.name);
                    self.use_name(name);
                    let t = if subst.is_empty() { param.ty } else { self.w.types.subst(param.ty, subst) };
                    self.param_type(t, param.repeated);
                }
            }
            let ret = if subst.is_empty() { sig.ret } else { self.w.types.subst(sig.ret, subst) };
            self.type_deps(ret);
        }
        self.bound = saved;
        self.replay_signature(s);
    }

    /// What the typer recorded for the signature of a member of the build: the aliases its
    /// declared types name, which a forwarder's types, copied from the member's, name too.
    fn replay_signature(&mut self, s: SymId) {
        self.replay(Comp::Sig(s));
    }

    fn replay(&mut self, comp: Comp) {
        let evs: Vec<Ev> = self.w.deps.as_ref().and_then(|d| d.records.get(&comp)).cloned().unwrap_or_default();
        for ev in evs {
            match ev {
                Ev::Alias(a) => self.named_alias(a),
                Ev::Forwarder(c, n) => self.forwarder_ref(c, n),
                Ev::Pkg(_) | Ev::Name(_) | Ev::ImportPath(_) | Ev::ImportSel(..) | Ev::LocalClass(_) => {}
            }
        }
    }

    /// The template of a companion scalac's desugaring makes: `object C extends AnyRef`, its
    /// constructor.
    fn made_companion_template(&mut self) {
        self.use_name("AnyRef".to_string());
        let object = self.w.b.any_ref;
        let t = self.w.b.t_any_ref;
        self.tree_type(t);
        self.class_dep(object, Context::Inheritance);
        self.ctor_call(object, t);
        let unit = self.w.b.t_unit;
        self.tree_type(unit);
    }

    fn class_bodies(&mut self, i: usize) {
        let (defaults, parent_args, prelude, init, methods, ctors) = {
            let tc = &self.w.prog.classes[i];
            (tc.ctor_defaults.clone(), tc.parent_args, tc.parent_prelude, tc.init.clone(), tc.methods.clone(), tc.ctors.clone())
        };
        for d in defaults.into_iter().flatten() {
            self.expr(d);
        }
        if let Some(args) = parent_args {
            self.list(args);
        }
        self.stmts(prelude);
        for it in init {
            match it {
                // An object's module val, which scalac's collector passes by.
                TInit::Field(s, _) if self.is_module_val(s) || self.w.syms.sym(s).kind == SymKind::Given && self.w.syms.sym(s).impl_class.is_some() => {}
                TInit::Field(_, e) | TInit::Stmt(e) => self.expr(e),
                TInit::Parent(_, pc) => {
                    self.stmts(pc.prelude);
                    self.list(pc.args);
                }
            }
        }
        for f in methods.into_iter().chain(ctors) {
            let s = self.w.prog.funs[f.idx()].sym;
            if self.w.syms.sym(s).def.is_some() {
                self.fun(f);
            }
        }
    }

    /// A template's parents as scalac's typer writes them: a class's or an object's superclass
    /// called (`Object` where none is written), a trait's `Object` as a type where it extends
    /// no class, then the traits; `Product` and `Serializable` after a case class's and a case
    /// object's, `scala.reflect.Enum` after an enum's. Each is a dependency by inheritance.
    fn parents(&mut self, c: ClassId, kind: ClassKind, mods: crate::ast::Mods, parents: &[TypeId]) {
        let inherit = if self.is_local(c) { Context::LocalInheritance } else { Context::Inheritance };
        let first_class = parents.first().and_then(|&p| self.class_of_parent(p)).filter(|&pc| self.w.syms.class(pc).kind != ClassKind::Trait);
        if first_class.is_none() && self.w.syms.class(c).value_class {
            let any_val = self.w.b.any_val;
            let t = self.w.b.t_any_val;
            self.tree_type(t);
            self.class_dep(any_val, inherit);
            self.ctor_call(any_val, t);
        } else if first_class.is_none() {
            let object = self.w.b.any_ref;
            let t = self.w.b.t_any_ref;
            self.tree_type(t);
            self.class_dep(object, inherit);
            if kind != ClassKind::Trait {
                self.ctor_call(object, t);
            }
        }
        for (i, &p) in parents.iter().enumerate() {
            self.tree_type(p);
            if let Some(pc) = self.class_of_parent(p) {
                self.class_dep(pc, inherit);
                if i == 0 && first_class.is_some() && kind != ClassKind::Trait {
                    self.ctor_call(pc, p);
                }
            }
        }
        if mods & crate::ast::mods::CASE != 0 && matches!(kind, ClassKind::Class | ClassKind::Object) {
            // `_root_.scala.Product`, `_root_.scala.Serializable`.
            self.use_name("scala".to_string());
            if let Some(product) = self.w.b.product {
                if !parents.iter().any(|&p| self.class_of_parent(p) == Some(product)) {
                    self.named_class(product);
                    self.class_dep(product, inherit);
                }
            }
            self.scala_serializable(inherit);
        }
        if kind == ClassKind::Enum {
            // A type tree the desugaring writes, no path of names.
            if let Some(e) = self.class_named("scala.reflect.Enum") {
                self.named_class(e);
                self.class_dep(e, inherit);
            }
        }
    }

    /// `scala.Serializable`, the alias of `java.io.Serializable` in `scala`'s package object:
    /// the alias, the package object and the interface.
    fn scala_serializable(&mut self, inherit: Context) {
        self.use_name("Serializable".to_string());
        self.use_name("package".to_string());
        if let Some(entry) = self.library_class_entry("scala/package") {
            self.here().binaries.insert((entry, "scala.package$".to_string(), Context::MemberRef));
        }
        if let Some(entry) = self.jdk_class_entry("java.io", "Serializable") {
            let binary = "java.io.Serializable".to_string();
            self.here().binaries.insert((entry.clone(), binary.clone(), Context::MemberRef));
            self.here().binaries.insert((entry, binary, inherit));
        }
    }

    /// The object of a qualified name: its module val and its class.
    fn object_named(&mut self, dotted: &str) -> Option<(SymId, ClassId)> {
        let (pkg, name) = dotted.rsplit_once('.')?;
        let mut p = ROOT_PKG;
        for seg in pkg.split('.') {
            let n = self.w.interner.intern(seg);
            p = self.w.demand_pkg(p, n)?;
        }
        let n = self.w.interner.intern(name);
        let s = self.w.pkg_term(p, n)?.sym()?;
        match self.w.syms.sym(s).kind {
            SymKind::Object(c) => Some((s, c)),
            _ => None,
        }
    }

    /// The class of a qualified name, read from the class path where the typing did not.
    fn class_named(&mut self, dotted: &str) -> Option<ClassId> {
        let (pkg, name) = dotted.rsplit_once('.')?;
        let mut p = ROOT_PKG;
        for seg in pkg.split('.') {
            let n = self.w.interner.intern(seg);
            p = self.w.demand_pkg(p, n)?;
        }
        let n = self.w.interner.intern(name);
        if let Some(c) = self.w.demand_class(p, n) {
            return Some(c);
        }
        match self.w.pkg_type(p, n)? {
            super::resolve::TypeRef::Class(c) => Some(c),
            _ => None,
        }
    }

    /// The jar or class file that holds the top-level class of `path` (`scala/package`) on the
    /// class path.
    fn library_class_entry(&mut self, path: &str) -> Option<String> {
        let loaded = self.w.loaded.as_ref()?;
        let f = loaded.cp.tasty_named(path).or_else(|| loaded.cp.class_named(path))?;
        Some(self.entry_path(f))
    }

    fn jdk_class_entry(&mut self, pkg: &str, simple: &str) -> Option<String> {
        let mut p = ROOT_PKG;
        for seg in pkg.split('.') {
            let n = self.w.interner.intern(seg);
            p = self.w.demand_pkg(p, n)?;
        }
        Some(match self.w.jdk_runtime_path_of(p, simple) {
            Some(path) => format!("jrt:{}", path),
            None => format!("jrt:/modules/java.base/{}/{}.class", pkg.replace('.', "/"), simple),
        })
    }

    /// `new C(..)`'s constructor, of the class type `t`: the constructor, its info (the
    /// class's type parameters its own, named by position: their bounds, the parameters' types,
    /// `C[..]`), and `t`, the tree's type.
    fn ctor_call(&mut self, c: ClassId, t: TypeId) {
        self.ctor_ref(c);
        self.ctor_info(c);
        self.type_deps(t);
    }

    /// The info of a constructor, and of a case class's `apply` teq makes no symbol of: the
    /// class's type parameters its own, named by position (their bounds, the parameters'
    /// types, `C[..]`).
    fn ctor_info(&mut self, c: ClassId) {
        self.ctor_info_written(c, true);
    }

    fn ctor_info_written(&mut self, c: ClassId, replayed: bool) {
        let (ctor, tparams, outer) = {
            let info = self.w.syms.class(c);
            (info.ctor.clone(), info.tparams.clone(), info.outer_tparams as usize)
        };
        let own = &tparams[outer.min(tparams.len())..];
        let depth = self.bind(own);
        for &p in own {
            let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
            self.type_deps(lo);
            self.type_deps(hi);
        }
        for clause in &ctor {
            for param in &clause.params {
                self.param_type(param.ty, param.repeated);
            }
        }
        let result = self.w.syms.this_type(c);
        self.type_deps(result);
        self.bound.truncate(depth);
        if replayed {
            self.replay(Comp::Class(c));
        }
    }

    fn class_of_parent(&self, t: TypeId) -> Option<ClassId> {
        match self.w.types.get(t) {
            Type::Class(c, _) => Some(c),
            _ => None,
        }
    }

    /// A call of the constructor of `c`: `C.<init>`, its name and `C`. A local class's full name
    /// is its enclosing class's with `_$` before its own name, an anonymous class's `$anon`.
    fn ctor_ref(&mut self, c: ClassId) {
        let full = if self.is_local(c) || self.w.syms.class(c).kind == ClassKind::Anon {
            let own = if self.w.syms.class(c).kind == ClassKind::Anon { "$anon".to_string() } else { self.w.name_str(self.w.syms.class(c).name) };
            match (self.this.last(), &self.package_this) {
                (Some(&k), _) => format!("{}._${}", self.mangled_full_name(k), own),
                (None, Some(p)) => format!("{}._${}", p, own),
                (None, None) => format!("_${}", own),
            }
        } else {
            self.mangled_full_name(c)
        };
        let name = format!("{};init;", full.replace('.', ";"));
        self.use_name(name);
        self.class_dep(c, Context::MemberRef);
    }

    /// `throw e`: scalac's `scala.throw(e)`, `(x: Throwable): Nothing`.
    fn throw_ref(&mut self) {
        self.use_name("throw".to_string());
        if let Some(t) = self.class_named("java.lang.Throwable") {
            self.named_class(t);
        }
        self.named_builtin("Nothing");
    }

    /// What scalac's desugaring writes for an enum `E` and teq's typer synthesizes without a
    /// tree: in `E` the import of its cases; in the companion `$values`, `values` and
    /// `valueOf` where every case is a value, `$new` for the values of no `extends` clause of
    /// an enum without type parameters, `fromOrdinal` where a case is a value. The values are
    /// static, and their enclosing class is the companion's owner: its class, or for a
    /// top-level enum the file's last class.
    fn enum_desugaring(&mut self, e: ClassId) {
        let Some(k) = self.w.syms.class(e).companion else { return };
        let cases = self.w.syms.class(e).children.clone();
        let file = self.w.syms.class(e).file;
        let generic = !self.w.syms.class(e).tparams.is_empty();
        let values: Vec<(ClassId, SymId, bool)> = cases
            .iter()
            .filter_map(|&c| {
                let v = self.w.syms.class(c).singleton?;
                let written = self.w.syms.class(c).def.map_or(false, |d| matches!(&self.w.ast(file).def(d).kind, crate::ast::DefKind::Class(cls) if !cls.parents.is_empty()));
                Some((c, v, !written && !generic))
            })
            .collect();
        let all_values = values.len() == cases.len() && !cases.is_empty();
        let simple = values.iter().any(|&(_, _, simple)| simple);
        // `E[?]` of an enum of type parameters.
        let enum_type = if generic {
            let items = vec![crate::types::WILD; self.w.syms.class(e).tparams.len()];
            self.w.types.class(e, &items)
        } else {
            self.w.syms.this_type(e)
        };
        let int = self.w.b.t_int;
        let string = self.w.b.t_string;
        if self.w.syms.class(k).def.is_none() {
            self.made_companion_template();
        }
        // `import E.{A, B}` in `E`'s body: the companion, and each case's value, or its class
        // and the class's companion.
        let module = self.w.syms.class(k).module_sym;
        if let Some(m) = module {
            self.term_node(m, None);
        }
        for &c in &cases {
            match self.w.syms.class(c).singleton {
                Some(v) => self.member_ref(v),
                None => {
                    self.class_ref(c);
                    if let Some(m) = self.companion_of(c).and_then(|o| self.w.syms.class(o).module_sym) {
                        self.member_ref(m);
                    }
                }
            }
        }
        let own_module = |t: &mut Self| {
            if let Some(m) = module {
                t.term_node(m, None);
            }
        };
        if all_values {
            // `private[this] val $values: Array[E] = Array.apply[E]([this.A, ..: E]*)(using
            // ClassTag.apply[E](classOf[E]))`.
            let array = self.w.b.array;
            self.named_class(array);
            self.type_deps(enum_type);
            if let Some(apply) = self.library_member("scala.Array", true, "apply", |sig| !sig.tparams.is_empty()) {
                self.term_node(apply, None);
            }
            self.use_name("<repeated>".to_string());
            for &(_, v, _) in &values {
                self.member_ref(v);
                self.info(v);
            }
            if let Some(apply) = self.library_member("scala.reflect.ClassTag", true, "apply", |_| true) {
                self.term_node(apply, None);
            }
            // `classOf[E]`, `Predef.classOf[T]: Class[T]`.
            self.use_name("classOf".to_string());
            if let Some((predef, _)) = self.object_named("scala.Predef") {
                self.term_node(predef, None);
            }
            if let Some(class) = self.class_named("java.lang.Class") {
                self.named_class(class);
            }
            // `def values: Array[E] = E.$values.clone()`.
            self.use_name("$values".to_string());
            own_module(self);
            // `clone(): Array[T]` of `Array`.
            self.use_name("clone".to_string());
            self.class_dep(array, Context::MemberRef);
            // `def valueOf($name: String): E = $name match { case "A" => this.A .. case _ =>
            // throw new IllegalArgumentException("enum E has no case with name: " + $name) }`.
            self.type_deps(string);
            self.use_name("$name".to_string());
            self.enum_failure("java.lang.IllegalArgumentException");
        }
        if simple {
            // `private[this] def $new(_$ordinal: Int, $name: String): E = { final class $anon
            // extends E(), _root_.scala.runtime.EnumValue; new $anon(): (E & EnumValue) }`.
            self.type_deps(int);
            self.type_deps(string);
            self.type_deps(enum_type);
            let anon = format!("{};_$$anon;init;", self.mangled_full_name(k).replace('.', ";"));
            self.enum_value_class(e, enum_type, &anon, None);
        }
        if !values.is_empty() {
            // `def fromOrdinal(ordinal: Int): E`, `$values.apply(ordinal)` of an enum of values
            // only, else a match of the values' ordinals; a failure is `NoSuchElementException(
            // "enum E has no case with ordinal: " + ordinal.toString())`.
            self.type_deps(int);
            self.type_deps(enum_type);
            self.use_name("ordinal".to_string());
            if all_values {
                self.use_name("$values".to_string());
                own_module(self);
                // `apply(i: Int): T` of `Array`.
                self.use_name("apply".to_string());
                let array = self.w.b.array;
                self.class_dep(array, Context::MemberRef);
            } else {
                for &(_, v, _) in &values {
                    self.member_ref(v);
                    self.info(v);
                }
            }
            // `toString(): String` of `Int`.
            self.use_name("toString".to_string());
            let int_class = self.w.b.int;
            self.class_dep(int_class, Context::MemberRef);
            self.enum_failure("java.util.NoSuchElementException");
        }
        // The values, static members of the companion.
        let saved = (self.owner.clone(), self.owner_file, self.owner_class, self.owner_pkg, self.package_this.clone());
        let holder = match self.w.syms.class(k).owner {
            Owner::Class(o) => Some(Responsible::Owned(Owned::Class(o))),
            Owner::Package(_) => self.responsible_for_imports(self.unit(file)),
            Owner::Local => None,
        };
        match holder {
            Some(Responsible::Owned(owned)) => self.enter(owned),
            Some(Responsible::Main(f, p, name)) => self.enter_main(f, p, name),
            None => return,
        }
        let anon = format!("{};_$$anon;init;", self.mangled_full_name(k).replace('.', ";"));
        for &(c, v, simple) in &values {
            let _ = v;
            self.type_deps(enum_type);
            if simple {
                // `val A: E = E.$new(n, "A")`.
                own_module(self);
                self.use_name("$new".to_string());
                self.type_deps(int);
                self.type_deps(string);
            } else {
                // `val A: E = { final class $anon extends E[..](args), EnumValue; new $anon():
                // (E[..] & EnumValue) }`.
                self.enum_value_class(e, enum_type, &anon, Some(c));
            }
        }
        let (owner, owner_file, owner_class, owner_pkg, package_this) = saved;
        self.owner = owner;
        self.owner_file = owner_file;
        self.owner_class = owner_class;
        self.owner_pkg = owner_pkg;
        self.package_this = package_this;
    }

    /// The class of an enum's value: `final class $anon extends E[..](args),
    /// _root_.scala.runtime.EnumValue`, then `new $anon(): (E[..] & EnumValue)`; of the case
    /// `case_class`'s parent where it has one, of `E()` for `$new`.
    fn enum_value_class(&mut self, e: ClassId, enum_type: TypeId, anon: &str, case_class: Option<ClassId>) {
        let parent = case_class.and_then(|c| self.w.syms.class(c).parents.first().copied()).unwrap_or(enum_type);
        let parent = match self.w.types.get(parent) {
            // An enum of type parameters: its value extends `E[Nothing]`.
            Type::Class(pc, args) if pc == e && case_class.is_some() && self.w.types.items(args).is_empty() && !self.w.syms.class(e).tparams.is_empty() => {
                let nothing = crate::types::NOTHING;
                let items = vec![nothing; self.w.syms.class(e).tparams.len()];
                self.w.types.class(e, &items)
            }
            _ => parent,
        };
        self.tree_type(parent);
        self.class_dep(e, Context::LocalInheritance);
        self.ctor_call(e, parent);
        if let Some(c) = case_class {
            if let Some(i) = self.tclass_of.get(&c).copied() {
                if let Some(args) = self.w.prog.classes[i].parent_args {
                    self.list(args);
                }
            }
        }
        for seg in ["scala", "runtime"] {
            self.use_name(seg.to_string());
        }
        if let Some(v) = self.class_named("scala.runtime.EnumValue") {
            self.named_class(v);
            self.class_dep(v, Context::LocalInheritance);
        }
        self.use_name(anon.to_string());
        self.type_deps(parent);
    }

    /// `throw new X("enum E has no case with ..: " + ..)` of `valueOf` and `fromOrdinal`.
    fn enum_failure(&mut self, exception: &str) {
        self.throw_ref();
        if let Some(x) = self.class_named(exception) {
            let t = self.w.types.class(x, &[]);
            self.ctor_ref(x);
            self.type_deps(t);
        }
        self.use_name("+".to_string());
        let string = self.w.b.string;
        self.class_dep(string, Context::MemberRef);
        let t = self.w.b.t_string;
        self.type_deps(t);
        self.named_builtin("Any");
    }

    /// A member of a library class, or of its companion object where `module`, of the name,
    /// the alternative `pick` takes of an overloaded one.
    fn library_member(&mut self, class: &str, module: bool, name: &str, pick: impl Fn(&crate::symbols::MethodSig) -> bool) -> Option<SymId> {
        let c = self.class_named(class)?;
        let owner = if !module || self.w.syms.class(c).kind == ClassKind::Object { c } else { self.companion_of(c)? };
        let n = self.w.interner.intern(name);
        let found = match self.w.module_term(owner, n).and_then(|r| r.sym()).or_else(|| self.member_named(owner, n)) {
            Some(s) => s,
            None => {
                let t = self.w.syms.this_type(owner);
                self.w.find_member(t, n)?.0
            }
        };
        let alts: Vec<SymId> = self.w.syms.alternatives(found).map(|a| a.to_vec()).unwrap_or_else(|| vec![found]);
        alts.into_iter().find(|&a| {
            let sig = self.w.sig_arc(a);
            pick(&sig)
        })
    }

    /// `given g[T](using x: X): Y with { .. }` of parameters, the method scalac's desugaring
    /// makes of it beside its class: `given def g[T](using x: X): g[T] = new g[T](using x)`.
    fn given_def(&mut self, sig: &crate::symbols::MethodSig, k: ClassId) {
        for &p in &sig.tparams {
            self.tparam_def(p);
        }
        for clause in &sig.clauses {
            for param in &clause.params {
                self.tree_type(param.ty);
            }
        }
        self.named_class(k);
        for &p in &sig.tparams {
            self.named_tparam(p);
        }
        // The class's header wrote its parents' aliases too, which are no part of this.
        let t = self.w.syms.this_type(k);
        self.ctor_ref(k);
        self.ctor_info_written(k, false);
        self.type_deps(t);
        for clause in &sig.clauses {
            for param in &clause.params {
                self.use_term_name(self.w.name_str(param.name));
            }
        }
    }

    /// A type parameter's definition: its bounds as written, `Nothing` and `Any` where none is.
    fn tparam_def(&mut self, p: TParamId) {
        let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
        self.tree_type(lo);
        self.tree_type(hi);
    }

    /// A type a tree of a definition names: all of it, a method's type parameters by name.
    fn tree_type(&mut self, t: TypeId) {
        let saved = std::mem::take(&mut self.bound);
        self.type_deps(t);
        self.bound = saved;
    }

    /// A member's definition: its type parameters, its parameters' and result's types.
    fn member(&mut self, s: SymId) {
        let Some(sig) = self.w.syms.sym(s).sig.clone() else { return };
        let given_class = self.w.syms.sym(s).impl_class.filter(|k| self.w.syms.sym(s).kind == SymKind::Given && !self.given_objects.contains_key(k));
        if let Some(k) = given_class {
            self.given_def(&sig, k);
            return;
        }
        for &p in &sig.tparams {
            self.tparam_def(p);
        }
        let saved = std::mem::take(&mut self.bound);
        for clause in &sig.clauses {
            for param in &clause.params {
                if param.repeated {
                    self.repeated_param_def(param.ty);
                } else {
                    self.type_deps(param.ty);
                }
            }
        }
        self.bound = saved;
        self.tree_type(sig.ret);
    }

    /// A repeated parameter as its definition's tree has it after the typer: `Seq[T]
    /// @Repeated`, the annotation's constructor called.
    fn repeated_param_def(&mut self, elem: TypeId) {
        if let Some(seq) = self.w.b.seq {
            self.named_class(seq);
        }
        if let Some(r) = self.class_named("scala.annotation.internal.Repeated") {
            self.named_class(r);
            self.ctor_ref(r);
        }
        self.type_deps(elem);
    }

    /// A parameter's type, a repeated one's as scalac writes it: `<repeated>[T]`.
    fn param_type(&mut self, t: TypeId, repeated: bool) {
        if repeated {
            self.use_name("<repeated>".to_string());
        }
        self.type_deps(t);
    }

    /// An opaque type's definition, an alias to scalac: `opaque type T = U`, `opaque type T <: B
    /// = U`.
    fn opaque_def(&mut self, k: ClassId) {
        let (tparams, underlying, parents) = {
            let info = self.w.syms.class(k);
            (info.tparams.clone(), info.underlying, info.parents.clone())
        };
        for p in tparams {
            self.tparam_def(p);
        }
        // A bound written is a `TypeBoundsTree`, whose lower bound is `Nothing`.
        if let Some(&bound) = parents.first() {
            self.named_builtin("Nothing");
            self.tree_type(bound);
        }
        if let Some(u) = underlying {
            self.tree_type(u);
        }
    }

    fn lexically_within(&self, o: ClassId) -> bool {
        let mut at = self.owner_class;
        while let Some(c) = at {
            if c == o {
                return true;
            }
            at = match self.w.syms.class(c).owner {
                Owner::Class(k) => Some(k),
                _ => None,
            };
        }
        false
    }

    /// The self type of a class with opaque type members refines it with their aliases, which
    /// its `this` reveals.
    fn opaque_refinement(&mut self, c: ClassId) {
        let opaques: Vec<ClassId> = self.w.syms.class(c).nested.values().copied().filter(|&k| self.w.syms.class(k).kind == ClassKind::Opaque).collect();
        for k in opaques {
            let (tparams, underlying) = {
                let info = self.w.syms.class(k);
                (info.tparams.clone(), info.underlying)
            };
            let depth = self.bind(&tparams);
            for &p in &tparams {
                let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
                self.type_deps(lo);
                self.type_deps(hi);
            }
            if let Some(u) = underlying {
                self.type_deps(u);
            }
            self.bound.truncate(depth);
            // The aliases its definition wrote, which the typer read through.
            let evs = self.w.deps.as_ref().and_then(|d| d.records.get(&Comp::Class(k))).cloned().unwrap_or_default();
            for ev in evs {
                match ev {
                    Ev::Alias(a) => self.named_alias(a),
                    Ev::Pkg(p) => self.package_ref(p),
                    Ev::Name(n) => self.written_name(n),
                    Ev::ImportPath(_) | Ev::ImportSel(..) | Ev::Forwarder(..) | Ev::LocalClass(_) => {}
                }
            }
        }
    }

    fn alias_def(&mut self, a: AliasId) {
        let (tparams, rhs, bounds) = {
            let info = self.w.syms.alias(a);
            (info.tparams.clone(), info.rhs, info.bounds)
        };
        for p in tparams {
            self.tparam_def(p);
        }
        match bounds {
            Some((lo, hi)) => {
                self.tree_type(lo);
                self.tree_type(hi);
            }
            None => self.tree_type(rhs),
        }
    }

    // ---- the bodies ----

    fn fun(&mut self, f: FunId) {
        let (defaults, body, params) = {
            let tf = &self.w.prog.funs[f.idx()];
            (tf.defaults.clone(), tf.body, tf.params.clone())
        };
        for p in params {
            self.local_def(p);
        }
        for d in defaults.into_iter().flatten() {
            self.expr(d);
        }
        if let Some(b) = body {
            self.expr(b);
        }
    }

    /// An inline method's body as its definition check stored it, which scalac's collector walks
    /// with the rest of its class's tree: what it refers to goes to the method's class, a
    /// macro's splice and its implementation among it, besides the callers' expansions.
    fn inline_definition(&mut self, s: SymId) {
        let Some(d) = self.w.inline_definitions.get(&s).cloned() else { return };
        let Some(body) = d.body else { return };
        for &p in &d.params {
            self.local_def(p);
        }
        let host = match self.w.syms.sym(s).owner {
            Owner::Class(c) => Some(c),
            _ => None,
        };
        self.inline_hosts.extend(host);
        self.in_inline_body = true;
        for &e in d.defaults.iter().flatten() {
            self.expr(e);
        }
        self.expr(body);
        self.in_inline_body = false;
        self.inline_hosts.truncate(self.inline_hosts.len() - host.is_some() as usize);
    }

    /// The name scalac's tree has for a read of `s` where an inline body reads it through an
    /// accessor of the inline method's class (`PrepareInlineable` rewrites the body to
    /// `inline$s`): in the body, and, for a private member, in an expansion of it in another
    /// class, whose name zinc keys apart.
    /// zinc's name hashing leaves a private member out, so the accessor's public name is what a
    /// change of the member's type reaches.
    fn accessor_name(&mut self, s: SymId) -> Option<String> {
        let Owner::Class(o) = self.w.syms.sym(s).owner else { return None };
        if !self.in_inline_body && self.zinc_name(o) == self.owner {
            return None;
        }
        // Outside the body, a protected or qualified-private member may be read directly, and
        // zinc hashes its name: the accessor stands for a private one alone there.
        let t = self.w.syms.sym(s);
        if !self.in_inline_body && (t.mods & crate::ast::mods::PRIVATE == 0 || t.scoped_private) {
            return None;
        }
        if self.accessor_names.is_none() {
            let mut names: FxMap<(ClassId, SymId), String> = FxMap::default();
            for i in 0..self.w.syms.classes.len() {
                let c = ClassId(i as u32);
                let info = self.w.syms.class(c);
                if info.def.is_none() || !info.member_order.iter().any(|&m| self.w.syms.sym(m).mods & crate::ast::mods::INLINE != 0) {
                    continue;
                }
                for a in self.w.inline_accessors(c) {
                    if !a.setter {
                        names.insert((c, a.target), a.name);
                    }
                }
            }
            self.accessor_names = Some(names);
        }
        for i in (0..self.inline_hosts.len()).rev() {
            let host = self.inline_hosts[i];
            if let Some(name) = self.accessor_names.as_ref().and_then(|m| m.get(&(host, s)).cloned()) {
                return Some(name);
            }
            // A class read from products, whose pickle's accessors the class path leaves out:
            // the accessor by its naming, for a member an inline body reads through one.
            if !self.w.program_file(self.w.syms.class(host).file) && self.w.needs_inline_accessor(host, s) {
                return Some(self.w.inline_accessor_name(host, s));
            }
        }
        None
    }

    /// A local's definition: the type it is declared with or the typer inferred.
    fn local_def(&mut self, s: SymId) {
        if let Some(sig) = self.w.syms.sym(s).sig.clone() {
            self.tree_type(sig.ret);
        }
    }

    /// The class of an object a class declares, where `s` is its lazy val.
    fn inner_object_class(&self, s: SymId) -> Option<ClassId> {
        let t = self.w.syms.sym(s).sig.as_ref()?.ret;
        match self.w.types.get(t) {
            Type::Class(k, _) if self.w.syms.class(k).inner_object == Some(s) => Some(k),
            _ => None,
        }
    }

    fn local_module_class(&self, v: SymId, e: TExprId) -> Option<ClassId> {
        match self.w.prog.expr(e) {
            TExpr::New(c, _) if self.w.syms.class(c).local_module == Some(v) => Some(c),
            _ => None,
        }
    }

    fn list(&mut self, l: ListRef) {
        let items: Vec<TExprId> = self.w.prog.expr_list(l).to_vec();
        for e in items {
            self.expr(e);
        }
    }

    fn stmts(&mut self, l: ListRef) {
        let items: Vec<TStmt> = self.w.prog.stmt_list(l).to_vec();
        for s in items {
            match s {
                TStmt::Expr(e) => self.expr(e),
                // A local object's lazy val, which scalac's collector passes by: its class alone.
                TStmt::Val(v, e) if self.local_module_class(v, e).is_some() => {
                    if let Some(c) = self.local_module_class(v, e) {
                        self.walk_class(c);
                    }
                }
                TStmt::Val(v, e) => {
                    self.local_def(v);
                    self.expr(e);
                }
                TStmt::Fun(f) => {
                    let s = self.w.prog.funs[f.idx()].sym;
                    self.member(s);
                    self.fun(f);
                }
                TStmt::Pat(p, e) => {
                    self.pat(p);
                    self.expr(e);
                    self.pattern_definition(p);
                }
            }
        }
    }

    /// `val C(a, b) = e` of several variables: scalac's desugaring matches into a tuple of them
    /// (`TupleN.apply`) and selects each with `_1`, `_2`, ...
    fn pattern_definition(&mut self, p: TPatId) {
        let mut binders = Vec::new();
        self.binders_of(p, &mut binders);
        let n = binders.len();
        if n < 2 {
            return;
        }
        if let Some(tuple) = self.class_named(&format!("scala.Tuple{}", n)) {
            self.named_class(tuple);
            self.companion_apply(tuple);
        }
        for i in 1..=n {
            self.use_name(format!("_{}", i));
        }
    }

    fn binders_of(&self, p: TPatId, out: &mut Vec<SymId>) {
        match self.w.prog.pats[p.idx()] {
            TPat::Bind(s, inner) => {
                out.push(s);
                if let Some(i) = inner {
                    self.binders_of(i, out);
                }
            }
            TPat::Test(_, _, inner) | TPat::Unapply(_, _, inner) => self.binders_of(inner, out),
            TPat::Class(_, _, _, subs) | TPat::Alt(subs) => {
                for &s in &self.w.prog.pat_lists[subs.range()] {
                    self.binders_of(s, out);
                }
            }
            TPat::Seq(items, rest) => {
                for &s in &self.w.prog.pat_lists[items.range()] {
                    self.binders_of(s, out);
                }
                if let Some(r) = rest {
                    self.binders_of(r, out);
                }
            }
            TPat::Wildcard | TPat::Equals(..) => {}
        }
    }

    fn cases(&mut self, l: ListRef) {
        let items: Vec<TCase> = self.w.prog.case_list(l).to_vec();
        for c in items {
            self.pat(c.pat);
            if let Some(g) = c.guard {
                self.expr(g);
            }
            self.expr(c.body);
        }
    }

    fn pat(&mut self, p: TPatId) {
        if self.walked_pats.insert(p, ()).is_some() {
            return;
        }
        match self.w.prog.pats[p.idx()] {
            TPat::Wildcard => {}
            TPat::Bind(s, inner) => {
                self.local_def(s);
                if let Some(i) = inner {
                    self.pat(i);
                }
            }
            TPat::Test(_, t, inner) => {
                self.tree_type(t);
                self.pat(inner);
            }
            TPat::Equals(e, _) => self.expr(e),
            TPat::Class(c, t, _, subs) => {
                self.tree_type(t);
                // `C(..)` is `C.unapply` of the companion, its fields selected as `_1`, `_2`, ..
                let unapply = self.w.syms.class(c).companion.and_then(|k| self.w.syms.class(k).members.get(&crate::names::UNAPPLY).copied());
                match unapply {
                    Some(u) if !matches!(self.w.syms.sym(u).kind, SymKind::Overloaded(_)) => self.term_node(u, None),
                    _ => {
                        self.use_name("unapply".to_string());
                        self.companion_ref(c);
                    }
                }
                let subs: Vec<TPatId> = self.w.prog.pat_lists[subs.range()].to_vec();
                for s in subs {
                    self.pat(s);
                }
            }
            TPat::Alt(subs) => {
                let subs: Vec<TPatId> = self.w.prog.pat_lists[subs.range()].to_vec();
                for s in subs {
                    self.pat(s);
                }
            }
            TPat::Seq(items, rest) => {
                let items: Vec<TPatId> = self.w.prog.pat_lists[items.range()].to_vec();
                for s in items {
                    self.pat(s);
                }
                if let Some(r) = rest {
                    self.pat(r);
                }
            }
            TPat::Unapply(s, call, inner) => {
                self.local_def(s);
                self.expr(call);
                self.pat(inner);
            }
        }
    }

    /// The type of the node as the typer recorded it.
    fn type_of(&mut self, e: TExprId) -> Option<TypeId> {
        self.w.prog.type_of(e)
    }

    fn expr(&mut self, e: TExprId) {
        let hosts: Vec<ClassId> = match self.w.prog.capture.as_deref().and_then(|c| c.inline_calls.get(&e)) {
            Some(calls) => calls
                .iter()
                .filter_map(|c| match self.w.syms.sym(c.callee).owner {
                    Owner::Class(o) => Some(o),
                    _ => None,
                })
                .collect(),
            None => Vec::new(),
        };
        if hosts.is_empty() {
            return self.expr_walk(e);
        }
        let depth = self.inline_hosts.len();
        self.inline_hosts.extend(hosts);
        self.expr_walk(e);
        self.inline_hosts.truncate(depth);
    }

    fn expr_walk(&mut self, e: TExprId) {
        if self.walked.insert(e, ()).is_some() {
            return;
        }
        let capture = self.w.prog.capture.as_deref();
        let wraps: Vec<Wrap> = capture.and_then(|c| c.wraps.get(&e)).cloned().unwrap_or_default();
        let form = capture.and_then(|c| c.forms.get(&e)).copied();
        let targs = capture.and_then(|c| c.targs.get(&e)).copied();
        let inline_calls = capture.and_then(|c| c.inline_calls.get(&e)).cloned();
        for w in wraps {
            match w {
                Wrap::Cast(t) => {
                    self.named_builtin_member("asInstanceOf");
                    if matches!(self.w.prog.expr(e), TExpr::Null) {
                        self.named_builtin("Null");
                    }
                    self.tree_type(t);
                }
                Wrap::Ascribed(t) | Wrap::Splice(t) => self.tree_type(t),
                Wrap::Unchecked | Wrap::Named(_) | Wrap::Member(..) => {}
            }
        }
        if let Some(list) = targs {
            for &t in self.w.types.items(list).to_vec().iter() {
                self.tree_type(t);
            }
        }
        if let Some(calls) = inline_calls {
            // The bindings of the receiver and arguments, which scalac's expansion substitutes
            // where they are paths: no reference of theirs names them.
            if calls.iter().any(|c| c.binds) {
                if let TExpr::Block(stmts, _) = self.w.prog.expr(e) {
                    for st in self.w.prog.stmt_list(stmts).to_vec() {
                        if let TStmt::Val(v, _) = st {
                            self.proxies.insert(v, ());
                        }
                    }
                }
            }
            for call in calls {
                self.member_ref(call.callee);
                self.info(call.callee);
                self.term_prefix(call.callee);
                if let Some(r) = call.recv {
                    self.expr(r);
                }
                for a in call.args {
                    self.expr(a);
                }
                for &t in self.w.types.items(call.targs).to_vec().iter() {
                    self.tree_type(t);
                }
            }
        }
        let node = self.w.deps.as_ref().and_then(|d| d.nodes.get(&e)).copied();
        match node {
            Some(Node::AliasVal(s)) | Some(Node::Constant(s)) => {
                self.term_node(s, None);
            }
            Some(Node::Apply(c)) => {
                self.companion_apply(c);
                if let Some(new) = self.new_under(e) {
                    self.applied.insert(new, ());
                }
            }
            Some(Node::Export(c, alias, recv)) => {
                self.expr(recv);
                // `recv.alias`, the forwarder of `c`: its name, `c`, and the original's info.
                self.use_name(self.w.name_str(alias));
                self.class_dep(c, Context::MemberRef);
                if let Some((call, s)) = self.call_under(e) {
                    self.forwarded.insert(call, ());
                    // The forwarder's info is the original's seen from the export's path.
                    let path = self.w.exports_of(c).and_then(|scope| scope.terms.get(&alias).copied());
                    let prefix = match path {
                        Some(super::resolve::TermRef::ModuleMember(o, _)) => Some(self.w.syms.this_type(o)),
                        _ => None,
                    };
                    self.info_seen_from(s, prefix);
                }
            }
            None => {}
        }
        if let Some(Form::Cast(_)) = form {
            match self.w.prog.expr(e) {
                // `null.asInstanceOf[T]`, the zero of `T`.
                TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Unit | TExpr::Null => {
                    self.named_builtin_member("asInstanceOf");
                    self.named_builtin("Null");
                }
                TExpr::Js(..) => self.named_builtin_member("asInstanceOf"),
                _ => {}
            }
        }
        if let Some(Form::EnumMember(name)) = form {
            self.enum_member(e, name);
            return;
        }
        if let (Some(Form::Op(crate::names::ORDINAL)), TExpr::Js(..)) = (form, self.w.prog.expr(e)) {
            // `e.ordinal`, of `scala.reflect.Enum`.
            self.use_name("ordinal".to_string());
            if let Some(c) = self.class_named("scala.reflect.Enum") {
                self.class_dep(c, Context::MemberRef);
            }
            let int = self.w.b.t_int;
            self.type_deps(int);
        }
        match form {
            Some(Form::Test(t)) | Some(Form::Cast(t)) | Some(Form::Evidence(t)) | Some(Form::JsObject(t)) => self.tree_type(t),
            Some(Form::Repeated(t)) => self.tree_type(t),
            Some(Form::Member(s)) => {
                self.member_ref(s);
                self.term_prefix(s);
            }
            _ => {}
        }
        match self.w.prog.expr(e) {
            TExpr::Int(_) | TExpr::Long(_) | TExpr::Double(_) | TExpr::Bool(_) | TExpr::Char(_) | TExpr::Str(_) | TExpr::Unit | TExpr::Null => {}
            TExpr::This | TExpr::Super(_) => {}
            TExpr::Local(s) => {
                if !self.synthetic_local(s) && !self.proxies.contains_key(&s) {
                    self.member_ref(s);
                }
                self.local_def(s);
            }
            TExpr::Static(_) if self.forwarded.contains_key(&e) => {}
            TExpr::Static(s) => self.term_node(s, None),
            TExpr::Module(_) if matches!(node, Some(Node::AliasVal(_))) => {}
            TExpr::Module(c) => {
                if let Some(m) = self.w.syms.class(c).module_sym {
                    self.term_node(m, None);
                } else {
                    self.named_class(c);
                }
            }
            TExpr::Field(r, s) => {
                if !self.forwarded.contains_key(&e) {
                    self.term_node(s, Some(r));
                    self.expr(r);
                }
            }
            TExpr::CallStatic(s, args) => {
                if !self.forwarded.contains_key(&e) {
                    self.term_node(s, None);
                }
                self.defaults(Callee::Method(s), args);
                self.list(args);
            }
            TExpr::CallMethod(r, s, args) => {
                // An assignment through an abstract var, which scalac's tree keeps as the var's
                // selection assigned.
                let assigned = super::setters::abstract_var_of_setter(&self.w.syms, self.w.interner, s);
                if !self.forwarded.contains_key(&e) {
                    self.term_node(assigned.unwrap_or(s), Some(r));
                    self.expr(r);
                }
                if assigned.is_none() {
                    self.defaults(Callee::Method(s), args);
                }
                self.list(args);
            }
            TExpr::CallClosure(f, args) => {
                // `f(args)` of a function value: `f.apply(args)`.
                if let Some(t) = self.receiver_type(f) {
                    let t = if self.w.types.has_vars(t) { self.w.zonk(t) } else { t };
                    if let Type::Class(c, _) = self.w.types.get(t) {
                        let apply = self.member_named(c, crate::names::APPLY).or_else(|| self.w.find_member(t, crate::names::APPLY).map(|(s, _)| s));
                        if let Some(apply) = apply {
                            self.member_ref(apply);
                            self.info_seen_from(apply, Some(t));
                        }
                    }
                    self.type_deps(t);
                }
                self.expr(f);
                self.list(args);
            }
            TExpr::New(_, args) if matches!(form, Some(Form::GivenCall(_))) => {
                if let Some(Form::GivenCall(g)) = form {
                    self.term_node(g, None);
                }
                self.list(args);
            }
            TExpr::New(c, args) if self.w.sam_classes.contains_key(&c) => {
                // A closure converted to a class with one abstract method: `Closure` of its type,
                // a dependency on that class by inheritance, local to the closure; its method is
                // the closure's body.
                let parents = self.w.syms.class(c).parents.clone();
                for p in parents {
                    self.tree_type(p);
                    if let Some(pc) = self.class_of_parent(p) {
                        self.class_dep(pc, Context::LocalInheritance);
                    }
                }
                if self.walked_classes.insert(c, ()).is_none() {
                    let members = self.w.syms.class(c).member_order.clone();
                    for s in members {
                        if let Some(sig) = self.w.syms.sym(s).sig.clone() {
                            for clause in &sig.clauses {
                                for param in &clause.params {
                                    self.type_deps(param.ty);
                                }
                            }
                            self.type_deps(sig.ret);
                        }
                    }
                    // The closure's body, which the class's method holds.
                    if let Some(i) = self.tclass_of.get(&c).copied() {
                        for f in self.w.prog.classes[i].methods.clone() {
                            self.fun(f);
                        }
                    }
                }
                self.list(args);
            }
            TExpr::New(c, args) if self.is_module(c) => {
                // An object's instance, which its module val makes: the object's class is walked
                // as a class of its own.
                self.list(args);
            }
            TExpr::New(c, args) if self.copies.contains_key(&e) => {
                // A case class's `copy`: `recv.copy(args)`, the receiver's fields for the
                // arguments not given its `copy$default$N` getters.
                let _ = c;
                let items: Vec<TExprId> = self.w.prog.expr_list(args).to_vec();
                for (i, a) in items.iter().enumerate() {
                    if let TExpr::Field(r, _) = self.w.prog.expr(*a) {
                        if matches!(self.w.prog.expr(r), TExpr::Local(t) if self.synthetic_local(t)) {
                            self.use_name(format!("copy$default${}", i + 1));
                            continue;
                        }
                    }
                    self.expr(*a);
                }
            }
            TExpr::New(c, args) => {
                if self.applied.contains_key(&e) {
                    self.defaults(Callee::Apply(c), args);
                } else {
                    let t = self.type_of(e).unwrap_or_else(|| self.w.syms.this_type(c));
                    self.tree_type(t);
                    self.ctor_call(c, t);
                    self.defaults(Callee::Ctor(c), args);
                }
                self.walk_class(c);
                self.list(args);
            }
            TExpr::NewVia(s, args) => {
                if let Owner::Class(c) = self.w.syms.sym(s).owner {
                    if let Some(t) = self.type_of(e) {
                        self.tree_type(t);
                    }
                    self.ctor_ref(c);
                }
                self.list(args);
            }
            TExpr::Lambda(params, body) => {
                let ps: Vec<SymId> = self.w.prog.sym_list(params).to_vec();
                for p in ps {
                    self.local_def(p);
                }
                if let Some(t) = self.type_of(body) {
                    self.tree_type(t);
                }
                self.sam(e);
                self.expr(body);
            }
            TExpr::If(a, b, c) => {
                self.expr(a);
                self.expr(b);
                if let Some(c) = c {
                    self.expr(c);
                }
            }
            TExpr::While(a, b) | TExpr::Assign(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            TExpr::Prim(op, a, b) => {
                let (name, left, right) = match form {
                    Some(Form::Op(n)) => (self.w.name_str(n), a, b),
                    Some(Form::SwappedOp(n)) => (self.w.name_str(n), b, a),
                    _ => (prim_name(op).to_string(), a, b),
                };
                let result = self.type_of(e);
                self.operator(&name, left, Some(right), result);
                self.expr(a);
                self.expr(b);
            }
            TExpr::Unary(op, a) => {
                match form {
                    Some(Form::Promotion) => {}
                    Some(Form::Widening) => self.widening(a, e),
                    Some(Form::Cast(_)) => self.named_builtin_member("asInstanceOf"),
                    _ => {
                        if let Some(name) = unary_name(op) {
                            let result = self.type_of(e);
                            self.operator(name, a, None, result);
                        }
                    }
                }
                self.expr(a);
            }
            TExpr::StrConcat(parts) => {
                match form {
                    Some(Form::Interp { kind, parts: written }) => self.interpolation(kind, written),
                    _ => self.concatenation(parts),
                }
                self.list(parts);
            }
            TExpr::ToStr(a, conv) => {
                if !conv.is_rendering() {
                    self.operator("toString", a, None, Some(self.w.b.t_string));
                }
                self.expr(a);
            }
            TExpr::Block(stmts, value) => {
                if matches!(form, Some(Form::CaseCopy)) {
                    self.case_copy(stmts, value);
                } else {
                    self.stmts(stmts);
                    self.expr(value);
                }
            }
            TExpr::Match(selector, cases) => {
                if let Some(t) = self.type_of(selector) {
                    self.patmat(t);
                }
                self.expr(selector);
                self.cases(cases);
            }
            TExpr::Index(a, _) | TExpr::JsSelect(a, _) | TExpr::Spread(a) | TExpr::Return(a) | TExpr::Splice(a) => self.expr(a),
            TExpr::Throw(a, _) => {
                self.throw_ref();
                self.expr(a);
            }
            TExpr::TypeTest(a, _) => self.expr(a),
            TExpr::Js(template, l) => {
                let sym = self.w.prog.template_syms.get(&template).copied();
                if let Some(s) = sym {
                    let items: Vec<TExprId> = self.w.prog.expr_list(l).to_vec();
                    let recv = (!self.w.syms.sym(s).is_extension && matches!(self.w.syms.sym(s).owner, Owner::Class(_))).then(|| items.first().copied()).flatten();
                    self.term_node(s, recv);
                }
                self.list(l);
            }
            TExpr::SeqLit(l) | TExpr::ArrayLit(l) | TExpr::ObjLit(l) => self.list(l),
            TExpr::ClassOf(c) => self.named_class(c),
            TExpr::JsImport(_) | TExpr::JsGlobal(..) => {}
            TExpr::Try(i) => {
                let t = self.w.prog.tries[i as usize].clone();
                self.expr(t.body);
                self.cases(t.cases);
                if let Some(f) = t.finalizer {
                    self.expr(f);
                }
            }
        }
    }

    /// `E.values`, `E.valueOf(name)`, `E.fromOrdinal(i)` of the members scalac's desugaring
    /// gives an enum's companion: the member, its info and the companion.
    fn enum_member(&mut self, e: TExprId, name: Name) {
        let Some(t) = self.type_of(e) else { return };
        let enum_type = match self.w.types.get(t) {
            Type::Class(c, args) if c == self.w.b.array => self.w.types.items(args).first().copied(),
            _ => Some(t),
        };
        let Some(Type::Class(c, _)) = enum_type.map(|t| self.w.types.get(t)) else { return };
        self.use_name(self.w.name_str(name));
        self.companion_ref(c);
        self.named_class(c);
        if name == crate::names::VALUES {
            let array = self.w.b.array;
            self.named_class(array);
        } else if let TExpr::Js(_, l) = self.w.prog.expr(e) {
            let param = if name == crate::names::VALUE_OF { self.w.b.t_string } else { self.w.b.t_int };
            self.type_deps(param);
            if let Some(&key) = self.w.prog.expr_list(l).get(1) {
                self.expr(key);
            }
        }
    }

    /// A reference to the term `s` through the receiver `recv` (or its owner's path): the term,
    /// its info as the reference's type has it, and the prefix.
    fn term_node(&mut self, s: SymId, recv: Option<TExprId>) {
        if let Some(r) = recv {
            if self.forwarded.contains_key(&r) {
                return;
            }
        }
        self.member_ref(s);
        let prefix = match recv {
            Some(r) => self.receiver_type(r),
            None => self.static_prefix(s),
        };
        self.info_seen_from(s, prefix);
        match (recv, prefix) {
            (Some(_), Some(t)) => self.type_deps(t),
            _ => self.term_prefix(s),
        }
    }

    /// The prefix a member is reached through without a receiver: its object's type.
    fn static_prefix(&mut self, s: SymId) -> Option<TypeId> {
        let Owner::Class(o) = self.w.syms.sym(s).owner else { return None };
        (self.w.syms.class(o).kind == ClassKind::Object).then(|| self.w.syms.this_type(o))
    }

    /// The type of a receiver: `C.this` of the class whose body is walked for `this`.
    fn receiver_type(&mut self, r: TExprId) -> Option<TypeId> {
        match self.w.prog.expr(r) {
            TExpr::This | TExpr::Super(_) => {
                let c = *self.this.last()?;
                Some(self.w.syms.this_type(c))
            }
            TExpr::Module(c) => self.type_of(r).or_else(|| Some(self.w.syms.this_type(c))),
            // A local's type, where the node's own is not kept.
            TExpr::Local(s) => self.type_of(r).or_else(|| self.w.syms.sym(s).sig.as_ref().map(|sig| sig.ret)),
            _ => self.type_of(r),
        }
    }

    /// The type of an operand as scalac selects the operator's overload for it: under a
    /// promotion the operand's own.
    fn operand_type(&mut self, e: TExprId) -> Option<TypeId> {
        let form = self.w.prog.capture.as_deref().and_then(|c| c.forms.get(&e)).copied();
        match (self.w.prog.expr(e), form) {
            (TExpr::Unary(_, inner), Some(Form::Promotion)) => self.operand_type(inner),
            _ => self.receiver_type(e),
        }
    }

    /// `left.name(right)` of a builtin operator: the name, a dependency on the class of the
    /// left operand (none for `Any`'s), and the types of the call.
    fn operator(&mut self, name: &str, left: TExprId, right: Option<TExprId>, result: Option<TypeId>) {
        self.use_name(name.to_string());
        let Some(lt) = self.operand_type(left) else { return };
        let lt = if self.w.types.has_vars(lt) { self.w.zonk(lt) } else { lt };
        match self.w.types.get(lt) {
            Type::Class(c, _) if self.w.syms.class(c).kind == ClassKind::Builtin => self.class_dep(c, Context::MemberRef),
            _ => {}
        }
        self.type_deps(lt);
        if let Some(r) = right.and_then(|r| self.operand_type(r)) {
            self.type_deps(r);
        }
        if let Some(t) = result {
            self.type_deps(t);
        }
    }

    /// A member of `Any` a node stands for (`asInstanceOf`): its name alone, `Any` holding it.
    fn named_builtin_member(&mut self, name: &str) {
        self.use_name(name.to_string());
        self.use_name("Any".to_string());
    }

    /// `Int.int2long(i)`: the numeric widening's conversion on the companion of the operand's
    /// class.
    fn widening(&mut self, operand: TExprId, widened: TExprId) {
        let (Some(from), Some(to)) = (self.receiver_type(operand), self.type_of(widened)) else { return };
        let (Type::Class(f, _), Type::Class(t, _)) = (self.w.types.get(from), self.w.types.get(to)) else { return };
        let (fname, tname) = (self.w.name_str(self.w.syms.class(f).name), self.w.name_str(self.w.syms.class(t).name));
        self.use_name(format!("{}2{}", fname.to_lowercase(), tname.to_lowercase()));
        self.use_name(fname);
        self.type_deps(from);
        self.type_deps(to);
    }

    /// A concatenation `a + b + ..`: each `+` on the left operand's class, `String.+(Any)` once a
    /// string heads the chain.
    fn concatenation(&mut self, parts: ListRef) {
        let items: Vec<TExprId> = self.w.prog.expr_list(parts).to_vec();
        let Some(&first) = items.first() else { return };
        let string = self.w.b.t_string;
        let head = items.first().and_then(|&h| self.rendered(h));
        let mut left_is_string = head.map_or(true, |t| t == string);
        for i in 1..items.len() {
            let left = if i == 1 { first } else { items[i - 1] };
            let _ = left;
            self.use_name("+".to_string());
            if left_is_string {
                self.class_dep(self.w.b.string, Context::MemberRef);
                self.type_deps(string);
                self.named_builtin("Any");
            } else if let Some(t) = head {
                if let Type::Class(c, _) = self.w.types.get(t) {
                    self.class_dep(c, Context::MemberRef);
                }
                self.type_deps(t);
                self.type_deps(string);
            }
            left_is_string = true;
        }
    }

    /// The type of a concatenation's operand before its rendering.
    fn rendered(&mut self, e: TExprId) -> Option<TypeId> {
        match self.w.prog.expr(e) {
            TExpr::ToStr(inner, conv) if conv.is_rendering() => self.receiver_type(inner),
            _ => self.receiver_type(e),
        }
    }

    /// `StringContext(parts*).kind(args*)`.
    fn interpolation(&mut self, kind: Name, parts: TList) {
        // `_root_.scala.StringContext.apply(parts*).kind(args*)`.
        self.use_name("scala".to_string());
        if let Some(sc) = self.class_named("scala.StringContext") {
            self.named_class(sc);
            self.companion_ref(sc);
            self.companion_apply(sc);
            // The interpolator method (`StringContext` also has an object `s` for patterns).
            let method = self.w.syms.class(sc).member_order.iter().copied().find(|&m| {
                let info = self.w.syms.sym(m);
                info.name == kind && info.kind == SymKind::Def
            });
            match method {
                Some(m) => {
                    self.member_ref(m);
                    self.info(m);
                }
                None => self.use_name(self.w.name_str(kind)),
            }
        }
        for &t in self.w.types.items(parts).to_vec().iter() {
            self.type_deps(t);
        }
    }

    /// The member `name` of `c`, an overloaded one's first alternative.
    fn member_named(&self, c: ClassId, name: Name) -> Option<SymId> {
        let s = self.w.syms.class(c).members.get(&name).copied()?;
        match self.w.syms.sym(s).kind {
            SymKind::Overloaded(_) => self.w.syms.alternatives(s).and_then(|a| a.first().copied()),
            _ => Some(s),
        }
    }

    /// `C.apply` of a case class `C`, which the typer wrote as the constructor's call: `apply`
    /// of the companion, through the companion.
    /// The companion object of `c`: the one teq's symbols link, or for a class read from the
    /// class path the object of its name beside it.
    fn companion_of(&mut self, c: ClassId) -> Option<ClassId> {
        if let Some(k) = self.w.syms.class(c).companion {
            return Some(k);
        }
        let (name, owner) = {
            let info = self.w.syms.class(c);
            (info.name, info.owner)
        };
        let r = match owner {
            Owner::Package(p) => self.w.pkg_term(p, name),
            Owner::Class(o) => self.w.module_term(o, name),
            Owner::Local => None,
        }?;
        match self.w.syms.sym(r.sym()?).kind {
            SymKind::Object(k) => Some(k),
            _ => None,
        }
    }

    fn companion_apply(&mut self, c: ClassId) {
        let companion = self.companion_of(c);
        let found = companion.and_then(|k| self.w.module_term(k, crate::names::APPLY)).and_then(|r| r.sym());
        // Of an `apply` the companion overloads, the one of the constructor's parameters.
        let arity = self.w.syms.class(c).ctor.first().map_or(0, |cl| cl.params.len());
        let apply = found.and_then(|a| match self.w.syms.alternatives(a).map(|x| x.to_vec()) {
            Some(alts) => alts.iter().copied().find(|&x| self.w.syms.sym(x).def.is_none()).or_else(|| {
                alts.iter().copied().find(|&x| self.w.sig_arc(x).clauses.first().map_or(0, |cl| cl.params.len()) == arity)
            }),
            None => Some(a),
        });
        match apply {
            Some(a) => self.term_node(a, None),
            None => {
                // A case class's `apply` the typer synthesizes: the constructor's info.
                self.use_term_name("apply".to_string());
                self.companion_ref(c);
                self.ctor_info(c);
            }
        }
    }

    /// The companion object of `c` as a tree names it: the object and its class.
    fn companion_ref(&mut self, c: ClassId) {
        let companion = self.companion_of(c);
        match companion.and_then(|k| self.w.syms.class(k).module_sym) {
            Some(m) => self.term_node(m, None),
            None => {
                // A companion teq's typer makes none of: scalac's object `C`, whose class is `C$`.
                let name = self.w.name_str(self.w.syms.class(c).name);
                self.use_name(name);
                match self.target(c) {
                    Target::Source(name, file) => {
                        if file != self.owner_file {
                            self.here().classes.insert((name, Context::MemberRef));
                        }
                    }
                    Target::Binary(entry, binary) => {
                        self.here().binaries.insert((entry, format!("{}$", binary), Context::MemberRef));
                    }
                    Target::Nowhere => {}
                }
            }
        }
    }

    /// `recv.copy(..)` of a case class, which the typer wrote as a block binding the receiver
    /// and a `new`: `copy` of the receiver's class and the receiver itself.
    fn case_copy(&mut self, stmts: ListRef, value: TExprId) {
        let items: Vec<TStmt> = self.w.prog.stmt_list(stmts).to_vec();
        let mut recv_type = None;
        for st in &items {
            match *st {
                TStmt::Val(v, init) if self.synthetic_local(v) => {
                    recv_type = self.w.syms.sym(v).sig.as_ref().map(|s| s.ret);
                    self.expr(init);
                }
                TStmt::Val(v, init) => {
                    self.local_def(v);
                    self.expr(init);
                }
                TStmt::Expr(x) => self.expr(x),
                TStmt::Fun(f) => self.fun(f),
                TStmt::Pat(p, x) => {
                    self.pat(p);
                    self.expr(x);
                }
            }
        }
        if let Some(new) = self.new_under(value) {
            if let TExpr::New(c, _) = self.w.prog.expr(new) {
                let copy = self.w.syms.class(c).members.get(&crate::names::COPY).copied();
                let prefix = recv_type.unwrap_or_else(|| self.w.syms.this_type(c));
                match copy {
                    Some(s) if !matches!(self.w.syms.sym(s).kind, SymKind::Overloaded(_)) && self.w.syms.sym(s).sig.is_some() => {
                        self.member_ref(s);
                        self.info_seen_from(s, Some(prefix));
                    }
                    _ => {
                        // `copy(ps): C` with the constructor's parameters, seen from the receiver.
                        self.use_name("copy".to_string());
                        self.class_dep(c, Context::MemberRef);
                        let subst = self.subst_from(prefix, c);
                        let ctor = self.w.syms.class(c).ctor.clone();
                        for clause in &ctor {
                            for param in &clause.params {
                                let t = if subst.is_empty() { param.ty } else { self.w.types.subst(param.ty, &subst) };
                                self.param_type(t, param.repeated);
                            }
                        }
                    }
                }
                self.type_deps(prefix);
                self.copies.insert(new, ());
            }
        }
        self.expr(value);
    }

    /// The call or selection of a member a node is, or the value of a block it is.
    fn call_under(&self, e: TExprId) -> Option<(TExprId, SymId)> {
        let mut e = e;
        for _ in 0..8 {
            match self.w.prog.expr(e) {
                TExpr::CallMethod(_, s, _) | TExpr::CallStatic(s, _) | TExpr::Field(_, s) | TExpr::Static(s) => return Some((e, s)),
                TExpr::Block(_, v) => e = v,
                _ => return None,
            }
        }
        None
    }

    /// The `New` a node is, or the value of a block it is.
    fn new_under(&self, e: TExprId) -> Option<TExprId> {
        let mut e = e;
        for _ in 0..8 {
            match self.w.prog.expr(e) {
                TExpr::New(..) => return Some(e),
                TExpr::Block(_, v) => e = v,
                _ => return None,
            }
        }
        None
    }

    /// The arguments a call leaves to the parameters' defaults: the default getters scalac
    /// calls for them, `m$default$N` of the method's class, a constructor's and a case class
    /// `apply`'s `$lessinit$greater$default$N` of the companion.
    fn defaults(&mut self, callee: Callee, args: ListRef) {
        let items: Vec<TExprId> = self.w.prog.expr_list(args).to_vec();
        let Some(capture) = self.w.prog.capture.as_deref() else { return };
        let defaulted: Vec<usize> = items.iter().enumerate().filter(|(_, e)| matches!(capture.forms.get(e), Some(Form::Default))).map(|(i, _)| i + 1).collect();
        if defaulted.is_empty() {
            return;
        }
        for n in defaulted {
            match callee {
                Callee::Method(s) => {
                    let name = format!("{}$default${}", crate::jvm::names::encode(self.w.name_ref(self.w.syms.sym(s).name)), n);
                    self.use_name(name);
                    match self.w.syms.sym(s).owner {
                        Owner::Class(o) => self.class_dep(o, Context::MemberRef),
                        Owner::Package(p) => {
                            let file = self.w.syms.sym(s).file;
                            if let Some(t) = self.package_member_target(PackageMember::Sym(s), file, p) {
                                self.package_object_dep(t);
                            }
                        }
                        Owner::Local => {}
                    }
                }
                Callee::Ctor(c) | Callee::Apply(c) => {
                    self.use_name(format!("$lessinit$greater$default${}", n));
                    self.companion_ref(c);
                }
            }
        }
    }

    /// A closure whose type is a class with one abstract method other than a function: a
    /// dependency on that class by inheritance, local to the closure.
    fn sam(&mut self, lambda: TExprId) {
        let Some(t) = self.type_of(lambda) else { return };
        let Type::Class(c, _) = self.w.types.get(t) else { return };
        let is_function = self.w.b.functions.contains(&Some(c)) || self.w.b.context_functions.contains(&Some(c));
        if !is_function {
            self.class_dep(c, Context::LocalInheritance);
        }
    }

    /// A match's selector: each sealed class its type names, used with the `PatMatTarget` scope.
    fn patmat(&mut self, t: TypeId) {
        let mut seen: FxMap<TypeId, ()> = FxMap::default();
        self.sealed_in(t, &mut seen);
    }

    fn sealed_in(&mut self, t: TypeId, seen: &mut FxMap<TypeId, ()>) {
        if seen.insert(t, ()).is_some() {
            return;
        }
        let t = if self.w.types.has_vars(t) { self.w.zonk(t) } else { t };
        match self.w.types.get(t) {
            Type::Class(c, args) => {
                let info = self.w.syms.class(c);
                if info.mods & crate::ast::mods::SEALED != 0 || info.kind == ClassKind::Enum {
                    let name = self.w.name_str(info.name);
                    self.use_sealed(name);
                }
                for &a in self.w.types.items(args).to_vec().iter() {
                    self.sealed_in(a, seen);
                }
            }
            Type::Union(a, b) | Type::Inter(a, b) => {
                self.sealed_in(a, seen);
                self.sealed_in(b, seen);
            }
            // What scalac's traverser follows to a named type's info: a type parameter's and an
            // abstract member's bounds, a path's underlying type, an alias's right side.
            Type::Param(p) => {
                let (lo, hi) = (self.w.syms.tparam(p).lower, self.w.syms.tparam(p).upper);
                self.sealed_in(lo, seen);
                self.sealed_in(hi, seen);
            }
            Type::Member(..) | Type::AppMember(..) | Type::Decl(_) => {
                let (lo, hi) = self.w.member_bounds(t);
                self.sealed_in(lo, seen);
                self.sealed_in(hi, seen);
            }
            Type::This(_) | Type::Term(_) | Type::Select(..) => {
                let under = self.w.path_underlying(t);
                self.sealed_in(under, seen);
            }
            Type::Alias(..) => {
                let under = self.w.deref(t);
                if under != t {
                    self.sealed_in(under, seen);
                }
            }
            _ => {}
        }
    }

    /// A local or anonymous class the body makes: its template, under the body's class.
    fn walk_class(&mut self, c: ClassId) {
        if !self.is_local(c) && self.w.syms.class(c).kind != ClassKind::Anon {
            return;
        }
        if self.walked_classes.insert(c, ()).is_some() {
            return;
        }
        let (parents, own_members, kind, mods, nested) = {
            let info = self.w.syms.class(c);
            let mut nested: Vec<ClassId> = info.nested.values().copied().collect();
            nested.sort();
            (info.parents.clone(), info.member_order.clone(), info.kind, info.mods, nested)
        };
        self.parents(c, kind, mods, &parents);
        let mut nested = nested;
        for s in own_members {
            // An object it declares: its class, its lazy val passed by as scalac's module val.
            if let Some(k) = self.inner_object_class(s) {
                nested.push(k);
                continue;
            }
            // The accessor of the outer instance, which scalac adds after the collector.
            if self.is_module_val(s) || self.w.name_ref(self.w.syms.sym(s).name).starts_with("$outer") {
                continue;
            }
            self.member(s);
        }
        // The classes and objects it declares, which nothing need instantiate either.
        for k in nested {
            if !matches!(self.w.syms.class(k).kind, ClassKind::Opaque | ClassKind::Builtin) {
                self.walk_class(k);
            }
        }
        if let Some(i) = self.tclass_of.get(&c).copied() {
            self.this.push(c);
            self.class_bodies(i);
            self.this.pop();
        }
    }
}
