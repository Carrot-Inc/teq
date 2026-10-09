use crate::arena::Arena;
use crate::ast::{mods, DefId, Mods};
use crate::intern::{FxMap, Name};
use crate::arena::{CellRef, Cells};
use crate::source::{FileId, Span};
use crate::types::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    Package(PkgId),
    Class(ClassId),
    /// A local definition inside a block.
    Local,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClassKind {
    Class,
    Trait,
    Object,
    Enum,
    EnumCase,
    Opaque,
    Builtin,
    /// The anonymous class backing a structural `given ... with`.
    GivenImpl,
    /// The class of a `new T { ... }` expression, owned by the scope where it stands.
    Anon,
}

/// How a class relates to JavaScript (Scala.js facade syntax).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JsKind {
    /// An ordinary Scala class.
    Scala,
    /// `@js.native`: a type that JavaScript defines. Nothing is emitted for it; its members are
    /// plain properties and methods of the receiver.
    Native,
    /// A class or trait with a native ancestor (`extends js.Object`): its instances are plain JS
    /// objects whose properties carry the verbatim member names.
    Object,
}

/// Where a native class, object, def or val lives in JavaScript.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JsBinding {
    /// `@JSImport("module", "name")`, an index into `Program::js_imports`.
    Import(u32),
    /// `@JSGlobal("name")`: a global identifier.
    Global(Name),
    /// `@JSGlobalScope`: an object whose members are global identifiers.
    GlobalScope,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Completion {
    NotStarted,
    InProgress,
    Done,
}

#[derive(Clone, Debug)]
pub struct ParamSig {
    pub name: Name,
    pub ty: TypeId,
    pub by_name: bool,
    pub repeated: bool,
    pub has_default: bool,
    pub sym: SymId,
}

#[derive(Clone, Debug, Default)]
pub struct ClauseSig {
    pub params: Vec<ParamSig>,
    pub is_using: bool,
    pub is_implicit: bool,
}

#[derive(Clone, Debug)]
pub struct MethodSig {
    pub tparams: Vec<TParamId>,
    pub clauses: Vec<ClauseSig>,
    pub ret: TypeId,
}

impl MethodSig {
    pub fn value(ty: TypeId) -> MethodSig {
        MethodSig { tparams: Vec::new(), clauses: Vec::new(), ret: ty }
    }

    /// The clauses of a right-associative extension method, the first `ext_clauses` of them the
    /// extension's, in the order dotty's `Desugar.extMethod` declares them (`rightAssocParams`),
    /// by their index here: the extension's using clauses before its receiver, the method's first
    /// clause, the receiver's and the extension's using clauses after it, the method's others.
    /// None where the method's first clause is no single parameter or a using clause: its
    /// clauses keep their order.
    pub fn right_assoc_order(&self, ext_clauses: usize) -> Option<Vec<usize>> {
        let own = self.clauses.get(ext_clauses)?;
        if own.is_using || own.params.len() != 1 {
            return None;
        }
        let receiver = (0..ext_clauses).find(|&i| !self.clauses[i].is_using)?;
        let mut order: Vec<usize> = (0..self.clauses.len()).collect();
        let first = order.remove(ext_clauses);
        order.insert(receiver, first);
        Some(order)
    }
}

/// What a recursive reference reads for a definition whose result type is being inferred, once
/// the error is reported.
pub static ERROR_SIG: MethodSig = MethodSig { tparams: Vec::new(), clauses: Vec::new(), ret: ERROR };

#[derive(Clone)]
pub struct ClassInfo {
    pub name: Name,
    pub kind: ClassKind,
    pub mods: Mods,
    pub owner: Owner,
    pub file: FileId,
    pub def: Option<DefId>,
    pub span: Span,
    /// For an anonymous class made while an inline method expanded: the file of the outermost
    /// call site, whose module holds the class, since the site's typing makes it and names it.
    pub made_at: Option<FileId>,
    pub tparams: Vec<TParamId>,
    pub parents: Vec<TypeId>,
    /// The class this one extends, directly or through a trait that extends a class.
    pub superclass: Option<ClassId>,
    /// The classes that have this one as their superclass.
    pub subclasses: Vec<ClassId>,
    pub members: FxMap<Name, SymId>,
    pub member_order: Vec<SymId>,
    pub nested: FxMap<Name, ClassId>,
    pub type_aliases: FxMap<Name, AliasId>,
    pub companion: Option<ClassId>,
    pub module_sym: Option<SymId>,
    pub children: Vec<ClassId>,
    pub ctor: Vec<ClauseSig>,
    /// Constructor parameter symbols per clause, created by the namer.
    pub ctor_syms: Vec<Vec<SymId>>,
    /// All ancestors (self first), each as a type in terms of this class's type parameters.
    pub base_types: Vec<(ClassId, TypeId)>,
    pub ordinal: u32,
    pub underlying: Option<TypeId>,
    pub extensions: Vec<SymId>,
    pub givens: Vec<SymId>,
    /// For enum cases without parameters: the value symbol in the enum companion.
    pub singleton: Option<SymId>,
    /// For an object defined in a block: the local lazy val that holds its one instance per
    /// run of the block; the class itself is an ordinary lifted class.
    pub local_module: Option<SymId>,
    /// For an object nested in a class or trait, from source or from a jar: the lazy val of the
    /// outer that holds its one instance per enclosing instance, as scalac's module val `val X:
    /// X.type = new X.type()`; the class itself is an ordinary inner class taking the outer
    /// instance first.
    pub inner_object: Option<SymId>,
    /// For enums with constructor parameters or initialisers: every case runs them through
    /// `$init`, and the values are created when the companion is initialised.
    pub stateful: bool,
    /// Whether the body has `export` clauses.
    pub has_exports: bool,
    /// The name of the body's `self =>` alias, or `EMPTY`.
    pub self_alias: Name,
    pub has_imports: bool,
    /// Whether the body has expression statements, or for a trait vals with an initialiser.
    pub has_statements: bool,
    /// A `@javaDefined` stand-in of the standard library that a definition of the same name in
    /// package `scala` took the place of; it is left out of the program.
    pub replaced: bool,
    pub js: JsKind,
    pub js_binding: Option<JsBinding>,
    /// Whether the class or one of its ancestors overloads a name.
    pub has_overloads: bool,
    /// Whether the output names of the members were settled, for a class compiled from a jar.
    pub named_for_output: bool,
    /// `extends AnyVal`: an ordinary class in the output, structurally equal and hashed.
    pub value_class: bool,
    /// The secondary constructors (`def this(...)`), in order, and the symbol that stands for
    /// the primary one next to them, made once the class has any.
    pub ctors: Vec<SymId>,
    pub primary_ctor: Option<SymId>,
    /// The `T` of a `self: T =>` declaration, in terms of the class's type parameters, and
    /// what `this` is inside the body then: the class's own type with `T`.
    pub declared_self: Option<TypeId>,
    pub this_type: Option<TypeId>,
    /// Whether an ancestor declares a type member, which a lookup in the body then walks to.
    pub inherits_types: bool,
    /// How many of `tparams` are the type parameters of the enclosing classes, which a class
    /// nested in a generic class of a jar takes first, from its prefix.
    pub outer_tparams: u8,
}

impl ClassInfo {
    /// The type parameters the class declares, after those it takes from enclosing classes.
    #[inline]
    pub fn own_tparams(&self) -> &[TParamId] {
        &self.tparams[self.outer_tparams as usize..]
    }

    /// The file whose module the class belongs to.
    pub fn module_file(&self) -> FileId {
        self.made_at.unwrap_or(self.file)
    }

    /// The name a case class or a case object prints itself by (its `productPrefix`): its own,
    /// where the copy of one an inline body defines goes by its name and its expansion's site
    /// (`C$<site>`, `Worker::local_class_name`).
    pub fn product_name<'a>(&self, name: &'a str) -> &'a str {
        if self.made_at.is_some() && self.owner == Owner::Local && self.kind != ClassKind::Anon && self.mods & crate::ast::mods::CASE != 0 {
            return name.split('$').next().unwrap_or(name);
        }
        name
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymKind {
    Val,
    Var,
    Def,
    Param,
    Object(ClassId),
    EnumValue(ClassId),
    Given,
    /// The entry of a name with several definitions in one scope: the alternatives are in
    /// `Symbols::overloads`. No definition and no expression refers to it; lookups resolve it.
    Overloaded(u32),
}

#[derive(Clone)]
pub struct SymInfo {
    pub name: Name,
    pub kind: SymKind,
    pub mods: Mods,
    pub owner: Owner,
    pub file: FileId,
    pub def: Option<DefId>,
    pub span: Span,
    pub sig: Option<Arc<MethodSig>>,
    pub is_extension: bool,
    pub ext_tparams: u8,
    pub ext_clauses: u8,
    /// JS expansion template from `@js("...")`.
    pub intrinsic: Option<Arc<str>>,
    /// Index into `Program::js_imports` for `@jsImport` and `@JSImport` definitions.
    pub js_import: Option<u32>,
    /// `@JSGlobal("name")` on a def or val: the global identifier it stands for.
    pub js_global: Option<Name>,
    /// `@JSName("name")`: the JavaScript property name of a member of a JS type.
    pub js_name: Option<Name>,
    /// `@JSBracketAccess`: `apply` reads `recv[key]` and `update` writes it.
    pub js_bracket: bool,
    /// `@JSName(js.Symbol.iterator)` from a jar: `js_name` is the well-known symbol's name, and
    /// the property is `recv[Symbol.iterator]`.
    pub js_symbol: bool,
    /// Set for a val that implements a parameterless def of a parent trait.
    pub needs_accessor: bool,
    /// Set for a var that implements an abstract setter (an abstract var's, or a declared
    /// `def x_=`): its JavaScript class writes the setter method beside the getter.
    pub needs_setter: bool,
    /// `private[scope]`: unlike a plain private member it is inherited and can be overridden.
    pub scoped_private: bool,
    /// Set for a val with an initialiser that a constructor parameter of a subclass overrides:
    /// the parameter is set first and keeps the field.
    pub overridden_by_param: bool,
    pub is_main: bool,
    /// For givens backed by a class (`given ... with`).
    pub impl_class: Option<ClassId>,
    pub by_name: bool,
    /// `@javaDefined`: callable with or without an empty argument list, as scalac allows for
    /// methods defined in Java.
    pub java_defined: bool,
    /// `@jvmEvidence`: the definition's `ClassTag` context bounds are evidence the JVM alone
    /// evaluates. Where it is erased the signature has no parameter for it, and the bits name
    /// the type parameters whose tag a call still has to find (`Worker::erased_evidence`).
    pub jvm_evidence: bool,
    pub erased_tags: u32,
    /// Set for the entry of a class member whose name an ancestor has a method for. Whether it
    /// overrides what it meets there or is overloaded with it is settled by the first lookup,
    /// until which `merge_pending` stays set.
    pub meets_inherited: bool,
    pub merge_pending: bool,
    /// Set for an overload set that a merge with the inherited alternatives replaced in its
    /// class (`Worker::merge_inherited`): it keeps its alternatives for a reader that took it
    /// before (a published record is not changed under its readers), and
    /// the passes over the sets pass it by.
    pub superseded: bool,
    /// Set for a method that an overloaded name stands for, in its own class or in a subclass.
    pub alternative: bool,
    /// Whether the method has a name of its own in the output, which `Symbols::dispatch_names`
    /// holds: an alternative of an overloaded name.
    pub dispatch: Dispatch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dispatch {
    Unknown,
    InProgress,
    Plain,
    Named,
}

#[derive(Clone)]
pub struct TParamInfo {
    pub name: Name,
    pub variance: i8,
    /// How many type arguments the parameter takes; 0 for a type.
    pub arity: u8,
    pub upper: TypeId,
    pub lower: TypeId,
    /// The variances a higher-kinded parameter declares for its own parameters (`CC[X, +Y]`);
    /// empty where they are all invariant.
    pub hk_variances: Vec<i8>,
}

#[derive(Clone)]
pub struct AliasInfo {
    pub name: Name,
    pub owner: Owner,
    pub file: FileId,
    /// Absent for an alias read from a library, which completes through `Typer::loaded`.
    pub def: Option<DefId>,
    pub tparams: Vec<TParamId>,
    pub rhs: TypeId,
    /// The bounds of an abstract type member (`type T >: L <: U`), which has no `rhs`; over
    /// the parameters as lambdas for a member that takes arguments.
    pub bounds: Option<(TypeId, TypeId)>,
}

impl AliasInfo {
    #[inline]
    pub fn is_abstract(&self) -> bool {
        self.bounds.is_some()
    }
}

#[derive(Default, Clone)]
pub struct PkgEntry {
    pub term: Option<SymId>,
    pub class: Option<ClassId>,
    pub alias: Option<AliasId>,
    pub pkg: Option<PkgId>,
    pub extensions: Vec<SymId>,
}

#[derive(Clone)]
pub struct PkgInfo {
    pub name: Name,
    pub parent: Option<PkgId>,
    pub entries: FxMap<Name, PkgEntry>,
    pub givens: Vec<SymId>,
    /// Files of this package that have top-level `export` clauses.
    pub export_files: Vec<FileId>,
    /// Another module's `<file>$package` objects of this package whose pickles hold export
    /// forwarders: their tables are the package's, as the program's top-level clauses are.
    pub export_objects: Vec<ClassId>,
    /// The `object package` of a `package object p extends ...`: what the package's own
    /// entries lack is looked up among its members, the inherited ones included.
    pub package_object: Option<ClassId>,
}

impl PkgInfo {
    /// Whether top-level export clauses add names to the package, the program's or another
    /// module's.
    pub fn has_exports(&self) -> bool {
        !self.export_files.is_empty() || !self.export_objects.is_empty()
    }
}

pub const ROOT_PKG: PkgId = PkgId(0);

pub struct Symbols {
    pub classes: Arena<ClassInfo>,
    pub syms: Arena<SymInfo>,
    pub tparams: Arena<TParamInfo>,
    pub aliases: Arena<AliasInfo>,
    pub pkgs: Arena<PkgInfo>,
    /// Each `SymKind::Overloaded` entry with its alternatives, in the order they were met.
    pub overloads: Arena<(SymId, Vec<SymId>)>,
    /// The names in the output of the methods whose `dispatch` is `Named`.
    pub dispatch_names: crate::arena::Layered<SymId, Name>,
    /// Per file its place in the canonical order: a program file's id, and for the pseudo files
    /// of library bodies, which are made in the order they are demanded, the order of their
    /// keys (`Worker::rank_files`). Empty until the typer is done: what orders the output by file
    /// reads the rank, not the id.
    pub file_ranks: Vec<u32>,
    /// The definitions converted from products placed in their sources, by
    /// pseudo file and definition: each its source, an index into `product_sources`, and the byte
    /// offset of its start. Set with the ranks, from the loader's table.
    pub product_places: crate::intern::FxMap<(FileId, crate::ast::DefId), (u32, u32)>,
    /// Per source of a product's definitions, its rank among the files, the token its producer
    /// recorded and its key.
    pub product_sources: Vec<(u32, u64, String)>,
    /// The pseudo files of products, each with its owning source.
    pub product_files: crate::intern::FxMap<FileId, u32>,
    /// The methods of products whose expansions were replayed, each with its source.
    pub product_callee_sources: crate::intern::FxMap<SymId, u32>,
    /// The members of products' classes that the whole program's classes have not
    /// (`Loaded::product_synthetics`).
    pub product_synthetics: crate::intern::FxMap<SymId, ()>,
    /// The classes of products no conversion places, each with its source and offset
    /// (`Loaded::product_class_places`).
    pub product_class_places: crate::intern::FxMap<ClassId, (u32, u32)>,
    /// The completion cells of the classes, symbols and aliases, and the body cells of the
    /// symbols, outside the records (`Cells`).
    pub class_cells: Cells,
    pub sym_cells: Cells,
    pub body_cells: Cells,
    pub alias_cells: Cells,
    /// The class checks' cells: claimed by the worker checking a class, `Done` once its `TClass`
    /// is appended (`Worker::check_class`).
    pub check_cells: Cells,
}

/// A symbol's record with its cells: the record by `Deref`, the signature cell `state` and the
/// body cell `body` as handles.
#[derive(Clone, Copy)]
pub struct SymRef<'a> {
    pub info: &'a SymInfo,
    cells: &'a Cells,
    bodies: &'a Cells,
    id: u32,
}

impl<'a> std::ops::Deref for SymRef<'a> {
    type Target = SymInfo;
    #[inline]
    fn deref(&self) -> &SymInfo {
        self.info
    }
}

impl<'a> SymRef<'a> {
    #[inline]
    pub fn state(&self) -> CellRef<'a> {
        self.cells.cell(self.id)
    }
    #[inline]
    pub fn body(&self) -> CellRef<'a> {
        self.bodies.cell(self.id)
    }
}

pub struct SymMut<'a> {
    pub info: &'a mut SymInfo,
    cells: &'a Cells,
    id: u32,
}

impl<'a> std::ops::Deref for SymMut<'a> {
    type Target = SymInfo;
    #[inline]
    fn deref(&self) -> &SymInfo {
        self.info
    }
}

impl<'a> std::ops::DerefMut for SymMut<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut SymInfo {
        self.info
    }
}

impl<'a> SymMut<'a> {
    #[inline]
    pub fn state(&self) -> CellRef<'a> {
        self.cells.cell(self.id)
    }
}

/// A class's record with its completion cell.
#[derive(Clone, Copy)]
pub struct ClassRef<'a> {
    pub info: &'a ClassInfo,
    cells: &'a Cells,
    id: u32,
}

impl<'a> std::ops::Deref for ClassRef<'a> {
    type Target = ClassInfo;
    #[inline]
    fn deref(&self) -> &ClassInfo {
        self.info
    }
}

impl<'a> ClassRef<'a> {
    #[inline]
    pub fn state(&self) -> CellRef<'a> {
        self.cells.cell(self.id)
    }
}

pub struct ClassMut<'a> {
    pub info: &'a mut ClassInfo,
    cells: &'a Cells,
    id: u32,
}

impl<'a> std::ops::Deref for ClassMut<'a> {
    type Target = ClassInfo;
    #[inline]
    fn deref(&self) -> &ClassInfo {
        self.info
    }
}

impl<'a> std::ops::DerefMut for ClassMut<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut ClassInfo {
        self.info
    }
}

impl<'a> ClassMut<'a> {
    #[inline]
    pub fn state(&self) -> CellRef<'a> {
        self.cells.cell(self.id)
    }
}

/// An alias's record with its completion cell.
#[derive(Clone, Copy)]
pub struct AliasRef<'a> {
    pub info: &'a AliasInfo,
    cells: &'a Cells,
    id: u32,
}

impl<'a> std::ops::Deref for AliasRef<'a> {
    type Target = AliasInfo;
    #[inline]
    fn deref(&self) -> &AliasInfo {
        self.info
    }
}

impl<'a> AliasRef<'a> {
    #[inline]
    pub fn state(&self) -> CellRef<'a> {
        self.cells.cell(self.id)
    }
}

pub struct AliasMut<'a> {
    pub info: &'a mut AliasInfo,
    cells: &'a Cells,
    id: u32,
}

impl<'a> std::ops::Deref for AliasMut<'a> {
    type Target = AliasInfo;
    #[inline]
    fn deref(&self) -> &AliasInfo {
        self.info
    }
}

impl<'a> std::ops::DerefMut for AliasMut<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut AliasInfo {
        self.info
    }
}

impl<'a> AliasMut<'a> {
    #[inline]
    pub fn state(&self) -> CellRef<'a> {
        self.cells.cell(self.id)
    }
}

impl Symbols {
    pub fn new() -> Symbols {
        let root = PkgInfo {
            name: crate::names::EMPTY,
            parent: None,
            entries: FxMap::default(),
            givens: Vec::new(),
            export_files: Vec::new(),
            export_objects: Vec::new(),
            package_object: None,
        };
        Symbols {
            classes: Arena::new(),
            syms: Arena::new(),
            tparams: Arena::new(),
            aliases: Arena::new(),
            pkgs: Arena::from_vec(vec![root]),
            overloads: Arena::new(),
            dispatch_names: Default::default(),
            file_ranks: Vec::new(),
            product_places: Default::default(),
            product_sources: Vec::new(),
            product_files: Default::default(),
            product_callee_sources: Default::default(),
            product_synthetics: Default::default(),
            product_class_places: Default::default(),
            class_cells: Cells::recycled(),
            sym_cells: Cells::recycled(),
            body_cells: Cells::recycled(),
            alias_cells: Cells::recycled(),
            check_cells: Cells::recycled(),
        }
    }

    /// Whether class `c` is one of `files`' own: it stands in one, or an expansion at a site in
    /// one made it (`made_at`); what typing the file again makes anew.
    pub fn class_of_files(&self, c: ClassId, files: &[FileId]) -> bool {
        let info = self.class(c);
        files.contains(&info.file) || info.made_at.map_or(false, |f| files.contains(&f))
    }

    /// Another worker's tables over the same shared regions.
    pub fn attach(&self, worker: usize) -> Symbols {
        Symbols {
            classes: Arena::attach(self.classes.shared_arc(), worker),
            syms: Arena::attach(self.syms.shared_arc(), worker),
            tparams: Arena::attach(self.tparams.shared_arc(), worker),
            aliases: Arena::attach(self.aliases.shared_arc(), worker),
            pkgs: Arena::attach(self.pkgs.shared_arc(), worker),
            overloads: Arena::attach(self.overloads.shared_arc(), worker),
            dispatch_names: self.dispatch_names.attach(),
            file_ranks: Vec::new(),
            product_places: Default::default(),
            product_sources: Vec::new(),
            product_files: Default::default(),
            product_callee_sources: Default::default(),
            product_synthetics: Default::default(),
            product_class_places: Default::default(),
            class_cells: self.class_cells.attach(worker),
            sym_cells: self.sym_cells.attach(worker),
            body_cells: self.body_cells.attach(worker),
            alias_cells: self.alias_cells.attach(worker),
            check_cells: self.check_cells.attach(worker),
        }
    }

    /// The workers' own records' cells at their merged ids (`Cells::carry`): worker `k`'s new
    /// ids by own index in `classes[k]`, `syms[k]` and `aliases[k]`.
    pub fn carry_cells(&self, classes: &[Vec<u32>], syms: &[Vec<u32>], aliases: &[Vec<u32>], crew: &crate::crew::Crew) {
        crew.each(classes.len() * 5, &|i| {
            let k = i / 5;
            match i % 5 {
                0 => self.class_cells.carry(k, &classes[k]),
                1 => self.check_cells.carry(k, &classes[k]),
                2 => self.sym_cells.carry(k, &syms[k]),
                3 => self.body_cells.carry(k, &syms[k]),
                _ => self.alias_cells.carry(k, &aliases[k]),
            }
        });
    }

    /// The place of a file in the canonical order (`file_ranks`).
    #[inline]
    pub fn file_rank(&self, f: FileId) -> u32 {
        self.file_ranks.get(f.0 as usize).copied().unwrap_or(f.0)
    }

    /// Where a definition of `file` stands in the canonical order: its file's rank and `start`,
    /// or for one converted from a product its source's rank and its place there.
    #[inline]
    pub fn place(&self, file: FileId, def: Option<crate::ast::DefId>, start: u32) -> (u32, u32) {
        match self.product_place(file, def) {
            Some((source, offset)) => (self.product_sources[source as usize].0, offset),
            None => (self.file_rank(file), start),
        }
    }

    /// The source and offset of a definition converted from a product.
    #[inline]
    pub fn product_place(&self, file: FileId, def: Option<crate::ast::DefId>) -> Option<(u32, u32)> {
        if self.product_places.is_empty() {
            return None;
        }
        self.product_places.get(&(file, def?)).copied()
    }
    /// The package of the object `c` where it is the companion of an opaque type held, as
    /// another module's pickle nests both and scalac places them, by its file's `$package` object:
    /// the package teq's own build makes a class of it in.
    pub fn opaque_companion_package(&self, c: ClassId, interner: &crate::intern::Interner) -> Option<PkgId> {
        let info = self.class(c);
        let Owner::Class(o) = info.owner else { return None };
        let holder = self.class(o);
        let Owner::Package(p) = holder.owner else { return None };
        let companion = info.companion.filter(|&k| self.class(k).kind == ClassKind::Opaque && (self.class(k).owner == Owner::Class(o) || self.class(k).owner == Owner::Package(p)));
        (info.kind == ClassKind::Object && holder.kind == ClassKind::Object && interner.get(holder.name).ends_with("$package") && companion.is_some()).then_some(p)
    }


    /// The tag a name made from a position takes for a definition converted from a product,
    /// the token its producer recorded, and the offset: in the place of the pseudo file's tag
    /// and its line's offset.
    pub fn product_position(&self, file: FileId, def: Option<crate::ast::DefId>) -> Option<(u64, u32)> {
        self.product_place(file, def).map(|(source, offset)| (self.product_sources[source as usize].1, offset))
    }

    /// The key of the source a product's pseudo file holds definitions of.
    pub fn product_file_key(&self, file: FileId) -> Option<&str> {
        let &source = self.product_files.get(&file)?;
        Some(self.product_sources[source as usize].2.as_str())
    }

    #[inline]
    pub fn class(&self, c: ClassId) -> ClassRef<'_> {
        let info = self.classes.get(c.0);
        #[cfg(debug_assertions)]
        for t in info.parents.iter().chain(info.base_types.iter().map(|(_, t)| t)).chain(info.underlying.iter()).chain(info.declared_self.iter()).chain(info.this_type.iter()) {
            crate::types::view::origin("a class's info", *t);
        }
        ClassRef { info, cells: &self.class_cells, id: c.0 }
    }
    /// The class's record once its completion is done, `None` before: the cell read before the
    /// record, so that a completion another thread published, its record's version and then its
    /// cell, each with release ordering, is read whole. A record taken before its cell may be the
    /// version from before the completion.
    #[inline]
    pub fn class_done(&self, c: ClassId) -> Option<ClassRef<'_>> {
        if self.class_cells.get(c.0) != Completion::Done {
            return None;
        }
        Some(self.class(c))
    }
    /// The class's record as it is published, for its ids alone (a metadata-only read: the
    /// classes of its base types, its members), which no view translates.
    #[inline]
    pub fn class_raw(&self, c: ClassId) -> &ClassInfo {
        self.classes.raw(c.0)
    }
    #[inline]
    pub fn class_mut(&mut self, c: ClassId) -> ClassMut<'_> {
        ClassMut { info: self.classes.get_mut(c.0), cells: &self.class_cells, id: c.0 }
    }
    #[inline]
    pub fn sym(&self, s: SymId) -> SymRef<'_> {
        SymRef { info: self.syms.get(s.0), cells: &self.sym_cells, bodies: &self.body_cells, id: s.0 }
    }
    #[inline]
    pub fn sym_mut(&mut self, s: SymId) -> SymMut<'_> {
        SymMut { info: self.syms.get_mut(s.0), cells: &self.sym_cells, id: s.0 }
    }
    /// The alias's record once its completion is done, `None` before, the cell read first
    /// (`class_done`).
    #[inline]
    pub fn alias_done(&self, a: AliasId) -> Option<AliasRef<'_>> {
        if self.alias_cells.get(a.0) != Completion::Done {
            return None;
        }
        Some(self.alias(a))
    }
    #[inline]
    pub fn alias(&self, a: AliasId) -> AliasRef<'_> {
        let info = self.aliases.get(a.0);
        #[cfg(debug_assertions)]
        for t in [info.rhs].into_iter().chain(info.bounds.into_iter().flat_map(|(lo, hi)| [lo, hi])) {
            crate::types::view::origin("an alias's right-hand side or bounds", t);
        }
        AliasRef { info, cells: &self.alias_cells, id: a.0 }
    }
    #[inline]
    pub fn alias_mut(&mut self, a: AliasId) -> AliasMut<'_> {
        AliasMut { info: self.aliases.get_mut(a.0), cells: &self.alias_cells, id: a.0 }
    }
    /// The signature `Worker::sig_of` completed, borrowed from this table alone: for a reader
    /// that goes on reading the worker's other fields meanwhile.
    pub fn sig(&self, s: SymId) -> &MethodSig {
        let info = self.sym(s);
        #[cfg(debug_assertions)]
        if let Some(sig) = &info.info.sig {
            for t in sig.clauses.iter().flat_map(|c| c.params.iter().map(|p| p.ty)).chain([sig.ret]) {
                crate::types::view::origin("a signature read raw (Symbols::sig)", t);
            }
        }
        match &info.info.sig {
            Some(sig) if info.state() != Completion::InProgress || sig.ret != ERROR => sig,
            _ => &ERROR_SIG,
        }
    }
    /// A given, or a Scala 2 implicit definition that the given search treats as one.
    #[inline]
    pub fn is_given(&self, s: SymId) -> bool {
        let info = &self.syms[s.idx()];
        info.kind == SymKind::Given || info.mods & (mods::IMPLICIT | mods::GIVEN) != 0
    }
    /// A Scala 3 given: one of the source, or a local or jar definition with the `given` flag.
    #[inline]
    pub fn is_scala3_given(&self, s: SymId) -> bool {
        let info = &self.syms[s.idx()];
        info.kind == SymKind::Given || info.mods & mods::GIVEN != 0
    }
    /// A Scala 2 implicit definition.
    #[inline]
    pub fn is_scala2_implicit(&self, s: SymId) -> bool {
        self.syms[s.idx()].mods & mods::IMPLICIT != 0
    }
    #[inline]
    pub fn tparam(&self, p: TParamId) -> &TParamInfo {
        let info = self.tparams.get(p.0);
        #[cfg(debug_assertions)]
        for t in [info.upper, info.lower] {
            crate::types::view::origin("a type parameter's bounds", t);
        }
        info
    }
    #[inline]
    pub fn pkg(&self, p: PkgId) -> &PkgInfo {
        self.pkgs.get(p.0)
    }

    pub fn sub_pkg(&mut self, parent: PkgId, name: Name) -> PkgId {
        if let Some(p) = self.pkgs[parent.idx()].entries.get(&name).and_then(|e| e.pkg) {
            return p;
        }
        let id = PkgId(self.pkgs.push(PkgInfo {
            name,
            parent: Some(parent),
            entries: FxMap::default(),
            givens: Vec::new(),
            export_files: Vec::new(),
            export_objects: Vec::new(),
            package_object: None,
        }));
        self.pkgs[parent.idx()].entries.entry(name).or_default().pkg = Some(id);
        id
    }

    pub fn new_class(
        &mut self,
        name: Name,
        kind: ClassKind,
        mods: Mods,
        owner: Owner,
        file: FileId,
        def: Option<DefId>,
        span: Span,
    ) -> ClassId {
        ClassId(self.classes.push(ClassInfo {
            name,
            kind,
            mods,
            owner,
            file,
            def,
            span,
            tparams: Vec::new(),
            parents: Vec::new(),
            superclass: None,
            subclasses: Vec::new(),
            members: FxMap::default(),
            member_order: Vec::new(),
            nested: FxMap::default(),
            type_aliases: FxMap::default(),
            companion: None,
            module_sym: None,
            children: Vec::new(),
            ctor: Vec::new(),
            ctor_syms: Vec::new(),
            base_types: Vec::new(),
            ordinal: 0,
            underlying: None,
            extensions: Vec::new(),
            givens: Vec::new(),
            singleton: None,
            local_module: None,
            inner_object: None,
            stateful: false,
            has_exports: false,
            self_alias: crate::names::EMPTY,
            has_imports: false,
            has_statements: false,
            replaced: false,
            js: JsKind::Scala,
            js_binding: None,
            has_overloads: false,
            named_for_output: false,
            made_at: None,
            value_class: false,
            ctors: Vec::new(),
            primary_ctor: None,
            declared_self: None,
            this_type: None,
            inherits_types: false,
            outer_tparams: 0,
        }))
    }

    pub fn new_sym(
        &mut self,
        name: Name,
        kind: SymKind,
        mods: Mods,
        owner: Owner,
        file: FileId,
        def: Option<DefId>,
        span: Span,
    ) -> SymId {
        SymId(self.syms.push(SymInfo {
            name,
            kind,
            mods,
            owner,
            file,
            def,
            span,
            sig: None,
            is_extension: false,
            ext_tparams: 0,
            ext_clauses: 0,
            intrinsic: None,
            js_import: None,
            js_global: None,
            js_name: None,
            js_bracket: false,
            js_symbol: false,
            needs_accessor: false,
            needs_setter: false,
            scoped_private: false,
            overridden_by_param: false,
            is_main: false,
            impl_class: None,
            by_name: false,
            java_defined: false,
            jvm_evidence: false,
            erased_tags: 0,
            meets_inherited: false,
            merge_pending: false,
            superseded: false,
            alternative: false,
            dispatch: Dispatch::Unknown,
        }))
    }

    /// The name under which the output defines and calls `s`: its own, unless it is an
    /// alternative of an overloaded method.
    #[inline]
    pub fn dispatch_name(&self, s: SymId) -> Name {
        let info = &self.syms[s.idx()];
        match info.dispatch {
            Dispatch::Named => *self.dispatch_names.get(&s).expect("a named method has its name"),
            _ => info.name,
        }
    }

    /// The alternatives `s` stands for when it is the entry of an overloaded name.
    #[inline]
    pub fn alternatives(&self, s: SymId) -> Option<&[SymId]> {
        match self.syms[s.idx()].kind {
            SymKind::Overloaded(i) => Some(&self.overloads[i as usize].1),
            _ => None,
        }
    }

    /// An entry that stands for `alts`, registered nowhere.
    pub fn new_overloaded(&mut self, like: SymId, owner: Owner, alts: Vec<SymId>) -> SymId {
        let (name, file, span) = {
            let s = self.sym(like);
            (s.name, s.file, s.span)
        };
        let index = self.overloads.push((SymId(0), Vec::new()));
        let set = self.new_sym(name, SymKind::Overloaded(index), 0, owner, file, None, span);
        if let Owner::Class(c) = owner {
            self.classes[c.idx()].has_overloads = true;
        }
        for &a in &alts {
            if !self.sym(a).alternative {
                self.sym_mut(a).alternative = true;
            }
        }
        *self.overloads.get_mut(index) = (set, alts);
        let mut info = self.sym_mut(set);
        info.sig = Some(Arc::new(MethodSig::value(ERROR)));
        info.state().done_fresh();
        set
    }

    pub fn new_tparam(&mut self, name: Name, variance: i8) -> TParamId {
        TParamId(self.tparams.push(TParamInfo { name, variance, arity: 0, upper: ANY, lower: NOTHING, hk_variances: Vec::new() }))
    }

    /// Whether `s` is a member of a JS type, whose members are plain JS properties and methods.
    pub fn js_member(&self, s: SymId) -> bool {
        match self.sym(s).owner {
            Owner::Class(c) => self.class(c).js != JsKind::Scala,
            _ => false,
        }
    }

    /// The JavaScript name of a member of a JS type: the `@JSName` or the source name.
    pub fn js_member_name(&self, s: SymId) -> Name {
        let info = self.sym(s);
        info.js_name.unwrap_or(info.name)
    }

    /// The type of `this` inside `c`: its own type, and the self type it declares.
    #[inline]
    pub fn this_type(&self, c: ClassId) -> TypeId {
        let info = self.class(c);
        info.this_type.unwrap_or_else(|| info.base_types.first().map_or(ERROR, |&(_, t)| t))
    }

    pub fn add_member(&mut self, c: ClassId, s: SymId) {
        let name = self.syms[s.idx()].name;
        let info = &mut self.classes[c.idx()];
        info.members.insert(name, s);
        info.member_order.push(s);
    }

    /// Whether a private member `name` of `c` would share its run-time name with a member of a
    /// class or trait above `c` or of a class below it: all of them belong to the one instance,
    /// so a backend names such a member apart.
    pub fn private_name_clashes(&self, c: ClassId, name: Name) -> bool {
        let info = self.class(c);
        if info.base_types.len() <= 1 && info.subclasses.is_empty() {
            return false;
        }
        let defines = |k: ClassId| k != c && self.class(k).members.contains_key(&name);
        if info.base_types.iter().any(|&(b, _)| defines(b)) {
            return true;
        }
        let mut below: Vec<ClassId> = info.subclasses.clone();
        while let Some(d) = below.pop() {
            let sub = self.class(d);
            if sub.base_types.iter().any(|&(b, _)| defines(b)) {
                return true;
            }
            below.extend_from_slice(&sub.subclasses);
        }
        false
    }

    /// A def that nothing can override, as scalac sees it: final, private, local, top-level or a
    /// member of a class that cannot be extended. Its self tail calls may become a loop.
    pub fn is_effectively_final(&self, s: SymId) -> bool {
        let info = self.sym(s);
        if info.mods & (mods::FINAL | mods::PRIVATE) != 0 {
            return true;
        }
        match info.owner {
            Owner::Local | Owner::Package(_) => true,
            Owner::Class(c) => {
                let class = self.class(c);
                class.mods & mods::FINAL != 0
                    || matches!(
                        class.kind,
                        ClassKind::Object | ClassKind::GivenImpl | ClassKind::Anon | ClassKind::EnumCase
                    )
            }
        }
    }
}

impl crate::arena::Record for SymInfo {}

impl crate::arena::Record for ClassInfo {}

impl crate::arena::Record for AliasInfo {}

impl crate::arena::Record for TParamInfo {}

/// The types a record of the symbols' tables holds, visited and mapped in one order: what a
/// worker's view translates (`Symbols::view_records`), a publication exports and the checks
/// look at.
pub trait RecordTypes: Clone {
    /// What the record is, as the checks name it.
    const WHAT: &'static str;
    fn each_type(&self, f: &mut dyn FnMut(TypeId));
    fn map_types(&mut self, f: &mut dyn FnMut(TypeId) -> TypeId);

    /// What a copy of the record holds on the heap, its vectors' and tables' allocations, which a
    /// view's copy costs beside the record itself (estimated from their capacities).
    fn heap_bytes(&self) -> usize;

    /// A copy's bytes: the record and its heap.
    fn copy_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.heap_bytes()
    }

    /// The record in the view of the worker on this thread: `None` where each of its types is
    /// the worker's canonical one already (the record is read as it is), else the copy with
    /// its types imported. The types handed over are checked in the assertion-enabled builds,
    /// once per version (`RecordView`).
    fn in_worker_view(&self) -> Option<Self> {
        TypeStore::worker_import(|v| self.translated(&mut |t| v.import(t), &mut |t| v.handed(Self::WHAT, t))).flatten()
    }

    /// A peer's record in the view of the loader's lock holder on this thread, the base: `None`
    /// where each of its types is the base's, else the copy with its types exported.
    fn in_holder_view(&self) -> Option<Self> {
        TypeStore::holder_export(|v| self.translated(&mut |t| v.export(t), &mut |t| v.handed(Self::WHAT, t))).flatten()
    }

    /// The record with each type through `f`, `None` where none moves; each type handed over
    /// through `handed`.
    fn translated(&self, f: &mut dyn FnMut(TypeId) -> TypeId, handed: &mut dyn FnMut(TypeId)) -> Option<Self> {
        let mut moved = false;
        self.each_type(&mut |t| moved |= f(t) != t);
        let copy = moved.then(|| {
            let mut copy = self.clone();
            copy.map_types(f);
            copy
        });
        copy.as_ref().unwrap_or(self).each_type(handed);
        copy
    }
}

fn sig_each_type(sig: &MethodSig, f: &mut dyn FnMut(TypeId)) {
    for c in &sig.clauses {
        for p in &c.params {
            f(p.ty);
        }
    }
    f(sig.ret);
}

/// The signature with each type through `f`, the same `Arc` where none moves.
pub fn sig_mapped(sig: &Arc<MethodSig>, f: &mut dyn FnMut(TypeId) -> TypeId) -> Arc<MethodSig> {
    let mut copy: Option<MethodSig> = None;
    for (i, c) in sig.clauses.iter().enumerate() {
        for (j, p) in c.params.iter().enumerate() {
            let t = f(p.ty);
            if t != p.ty {
                copy.get_or_insert_with(|| (**sig).clone()).clauses[i].params[j].ty = t;
            }
        }
    }
    let ret = f(sig.ret);
    if ret != sig.ret {
        copy.get_or_insert_with(|| (**sig).clone()).ret = ret;
    }
    copy.map_or_else(|| sig.clone(), Arc::new)
}

fn sig_bytes(sig: &MethodSig) -> usize {
    std::mem::size_of::<MethodSig>() + sig.tparams.capacity() * 4 + sig.clauses.capacity() * std::mem::size_of::<ClauseSig>() + sig.clauses.iter().map(|c| c.params.capacity() * std::mem::size_of::<ParamSig>()).sum::<usize>()
}

/// A hash map's allocation from its capacity: its entries and a control byte each.
fn map_bytes<K, V>(m: &FxMap<K, V>) -> usize {
    m.capacity() * (std::mem::size_of::<(K, V)>() + 1)
}

impl RecordTypes for SymInfo {
    const WHAT: &'static str = "a signature (Symbols::sym)";
    fn heap_bytes(&self) -> usize {
        self.sig.as_deref().map_or(0, sig_bytes)
    }
    fn each_type(&self, f: &mut dyn FnMut(TypeId)) {
        if let Some(sig) = &self.sig {
            sig_each_type(sig, f);
        }
    }
    fn map_types(&mut self, f: &mut dyn FnMut(TypeId) -> TypeId) {
        if let Some(sig) = &self.sig {
            self.sig = Some(sig_mapped(sig, f));
        }
    }
}

impl RecordTypes for ClassInfo {
    const WHAT: &'static str = "a class's info (Symbols::class)";
    fn heap_bytes(&self) -> usize {
        let ids = self.tparams.capacity() + self.subclasses.capacity() + self.member_order.capacity() + self.children.capacity() + self.extensions.capacity() + self.givens.capacity() + self.ctors.capacity();
        ids * 4
            + self.parents.capacity() * 4
            + self.base_types.capacity() * 8
            + self.ctor.capacity() * std::mem::size_of::<ClauseSig>()
            + self.ctor.iter().map(|c| c.params.capacity() * std::mem::size_of::<ParamSig>()).sum::<usize>()
            + self.ctor_syms.capacity() * std::mem::size_of::<Vec<SymId>>()
            + self.ctor_syms.iter().map(|v| v.capacity() * 4).sum::<usize>()
            + map_bytes(&self.members)
            + map_bytes(&self.nested)
            + map_bytes(&self.type_aliases)
    }
    fn each_type(&self, f: &mut dyn FnMut(TypeId)) {
        self.parents.iter().for_each(|&t| f(t));
        self.base_types.iter().for_each(|&(_, t)| f(t));
        self.underlying.iter().chain(self.declared_self.iter()).chain(self.this_type.iter()).for_each(|&t| f(t));
        self.ctor.iter().flat_map(|c| c.params.iter()).for_each(|p| f(p.ty));
    }
    fn map_types(&mut self, f: &mut dyn FnMut(TypeId) -> TypeId) {
        self.parents.iter_mut().for_each(|t| *t = f(*t));
        self.base_types.iter_mut().for_each(|(_, t)| *t = f(*t));
        self.underlying.iter_mut().chain(self.declared_self.iter_mut()).chain(self.this_type.iter_mut()).for_each(|t| *t = f(*t));
        self.ctor.iter_mut().flat_map(|c| c.params.iter_mut()).for_each(|p| p.ty = f(p.ty));
    }
}

impl RecordTypes for TParamInfo {
    const WHAT: &'static str = "a type parameter's bounds (Symbols::tparam)";
    fn heap_bytes(&self) -> usize {
        self.hk_variances.capacity()
    }
    fn each_type(&self, f: &mut dyn FnMut(TypeId)) {
        f(self.upper);
        f(self.lower);
    }
    fn map_types(&mut self, f: &mut dyn FnMut(TypeId) -> TypeId) {
        self.upper = f(self.upper);
        self.lower = f(self.lower);
    }
}

impl RecordTypes for AliasInfo {
    const WHAT: &'static str = "an alias's right-hand side or bounds (Symbols::alias)";
    fn heap_bytes(&self) -> usize {
        self.tparams.capacity() * 4
    }
    fn each_type(&self, f: &mut dyn FnMut(TypeId)) {
        f(self.rhs);
        if let Some((lo, hi)) = self.bounds {
            f(lo);
            f(hi);
        }
    }
    fn map_types(&mut self, f: &mut dyn FnMut(TypeId) -> TypeId) {
        self.rhs = f(self.rhs);
        if let Some((lo, hi)) = self.bounds {
            self.bounds = Some((f(lo), f(hi)));
        }
    }
}

impl Symbols {
    /// Reads the shared records through the worker's views from here, or no more: a signature, a class's
    /// info, a type parameter's bounds and an alias's record read by the worker outside the
    /// loader's lock with its types in the worker's view, each version translated once
    /// (`arena::RecordView`). Off, the copies go.
    pub fn view_records(&mut self, on: bool) {
        fn view<T: RecordTypes>(on: bool) -> Option<crate::arena::RecordViewFns<T>> {
            on.then_some(crate::arena::RecordViewFns { in_view: T::in_worker_view, in_base: T::in_holder_view, bytes: T::copy_bytes })
        }
        self.syms.set_view(view::<SymInfo>(on));
        self.classes.set_view(view::<ClassInfo>(on));
        self.tparams.set_view(view::<TParamInfo>(on));
        self.aliases.set_view(view::<AliasInfo>(on));
    }

    /// Every record this worker made so far has escaped it (`Arena::escape_own`).
    #[cfg(debug_assertions)]
    pub fn escape_own(&self) {
        self.syms.escape_own();
        self.classes.escape_own();
        self.tparams.escape_own();
        self.aliases.escape_own();
        self.pkgs.escape_own();
        self.overloads.escape_own();
    }

    /// Frees the copies of the versions the views read before their last (`Arena::sweep_view`).
    pub fn sweep_views(&mut self) {
        self.syms.sweep_view();
        self.classes.sweep_view();
        self.tparams.sweep_view();
        self.aliases.sweep_view();
    }

    /// Each type of the records the loader's lock holder is about to publish (`Arena::unpublished`)
    /// with what the record is: the checks' look at a publication before it becomes every
    /// worker's.
    #[cfg(debug_assertions)]
    pub fn each_unpublished_type(&self, f: &mut dyn FnMut(&'static str, TypeId)) {
        fn each<T: RecordTypes>(a: &Arena<T>, f: &mut dyn FnMut(&'static str, TypeId)) {
            for r in a.unpublished() {
                r.each_type(&mut |t| f(T::WHAT, t));
            }
        }
        each(&self.syms, f);
        each(&self.classes, f);
        each(&self.tparams, f);
        each(&self.aliases, f);
    }

    /// Per record kind (signatures, classes, type parameters, aliases), what the views hold and
    /// did.
    pub fn view_counts(&self) -> [crate::arena::ViewCounts; 4] {
        [self.syms.view_counts(), self.classes.view_counts(), self.tparams.view_counts(), self.aliases.view_counts()]
    }
}
impl crate::arena::Record for PkgInfo {}
impl crate::arena::Record for SymId {}
impl crate::arena::Record for PkgId {}
impl crate::arena::Record for Vec<ClassId> {}
impl crate::arena::Record for (SymId, Vec<SymId>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ReentrantLock;

    /// A build past its signature phase at one worker through the fork, the overlays' shared
    /// namespace: the symbols' tables forked with one class `c`
    /// from before the fork, the store forked with this thread its worker, the views on.
    struct Forked {
        store: Box<TypeStore>,
        syms: Symbols,
        lock: ReentrantLock,
        c: ClassId,
        p: TParamId,
        s: SymId,
        a: AliasId,
    }

    impl Forked {
        fn new() -> Forked {
            let store = Box::new(TypeStore::new());
            store.class(ClassId(1), &[]);
            store.set_exclusive(false);
            let mut syms = Symbols::new();
            let c = syms.new_class(Name(1), ClassKind::Class, 0, Owner::Package(ROOT_PKG), FileId(0), None, Span::default());
            let p = syms.new_tparam(Name(2), 0);
            let s = syms.new_sym(Name(5), SymKind::Def, 0, Owner::Package(ROOT_PKG), FileId(0), None, Span::default());
            let a = AliasId(syms.aliases.push(AliasInfo { name: Name(6), owner: Owner::Package(ROOT_PKG), file: FileId(0), def: None, tparams: Vec::new(), rhs: ERROR, bounds: None }));
            syms.syms.fork();
            syms.classes.fork();
            syms.tparams.fork();
            syms.aliases.fork();
            assert!(store.fork_overlays(1));
            store.enter_overlay(0);
            syms.view_records(true);
            Forked { store, syms, lock: ReentrantLock::new(), c, p, s, a }
        }

        /// Takes the loader's lock, at depth one what `Worker::lock_taken` does of the tables:
        /// what the hold pushes is staged in the shared region.
        fn take(&mut self) {
            self.lock.lock();
            if crate::shared::lock_depth() == 1 {
                self.syms.sweep_views();
                let s = &mut self.syms;
                s.syms.lock_taken();
                s.classes.lock_taken();
                s.tparams.lock_taken();
                s.aliases.lock_taken();
                for a in [&mut s.syms.alloc_shared, &mut s.classes.alloc_shared, &mut s.tparams.alloc_shared, &mut s.aliases.alloc_shared] {
                    *a = true;
                }
            }
        }

        /// Releases it, at depth one publishing what the hold staged and changed.
        fn release(&mut self) {
            if crate::shared::lock_depth() == 1 {
                let s = &mut self.syms;
                for a in [&mut s.syms.alloc_shared, &mut s.classes.alloc_shared, &mut s.tparams.alloc_shared, &mut s.aliases.alloc_shared] {
                    *a = false;
                }
                s.syms.lock_released();
                s.classes.lock_released();
                s.tparams.lock_released();
                s.aliases.lock_released();
            }
            self.lock.unlock();
        }

        /// A class type of `k` in this thread's namespace: the worker's overlay outside the
        /// lock, the base under it.
        fn ty(&self, k: u32) -> TypeId {
            self.store.class(ClassId(k), &[])
        }
    }

    impl Drop for Forked {
        fn drop(&mut self) {
            // A test that panics under the lock leaves it held: the lock goes unheld.
            while crate::shared::lock_depth() > 0 {
                self.lock.unlock();
            }
            self.syms.view_records(false);
            self.store.leave_overlay();
        }
    }

    /// The worker made `C9` before the holder made the base's: a record the holder publishes
    /// with the base's reads as the worker's own for the worker and as the base's for the holder,
    /// and the metadata-only read takes it as it is.
    #[test]
    fn a_worker_reads_its_own_equivalent_and_the_holder_the_bases() {
        let mut f = Forked::new();
        let own = f.ty(9);
        f.take();
        let base = f.ty(9);
        assert_ne!(own, base, "the holder's view is the base alone");
        f.syms.class_mut(f.c).parents = vec![base];
        f.release();
        assert_eq!(f.syms.class(f.c).parents, vec![own]);
        assert_eq!(f.syms.class_raw(f.c).parents, vec![base]);
        f.take();
        assert_eq!(f.syms.class(f.c).parents, vec![base]);
        f.release();
    }

    /// A base type the worker holds no equivalent of is its canonical one: read as it is, the
    /// record not copied.
    #[test]
    fn a_record_whose_types_are_canonical_is_read_as_it_is() {
        let mut f = Forked::new();
        f.take();
        let base = f.ty(9);
        f.syms.class_mut(f.c).parents = vec![base];
        f.release();
        let read: *const ClassInfo = &*f.syms.class(f.c);
        assert_eq!(f.syms.class(f.c).parents, vec![base]);
        assert!(std::ptr::eq(read, f.syms.class_raw(f.c)), "a copy of a record nothing moves in");
        assert_eq!(f.syms.classes.view_counts().copies, 0);
    }

    /// An unshared path reads as the worker's one representative of its origin.
    #[test]
    fn an_unshared_path_reads_as_the_workers_representative() {
        let mut f = Forked::new();
        let own = f.store.term_unshared(SymId(3));
        let p = f.p;
        f.take();
        let exported = f.store.export(own);
        assert_ne!(exported, own);
        f.syms.tparams.get_mut(p.0).upper = exported;
        f.release();
        assert_eq!(f.syms.tparam(p).upper, own);
    }

    /// Within one hold the holder reads its working copy as it changes (a read, a change, a read),
    /// never a view's cache; the worker's read after the release is the new version's.
    #[test]
    fn a_read_within_the_hold_sees_the_change() {
        let mut f = Forked::new();
        let own = f.ty(9);
        assert!(f.syms.class(f.c).parents.is_empty());
        f.take();
        assert!(f.syms.class(f.c).parents.is_empty());
        let base = f.ty(9);
        f.syms.class_mut(f.c).parents = vec![base];
        assert_eq!(f.syms.class(f.c).parents, vec![base]);
        let this = f.ty(10);
        f.syms.class_mut(f.c).this_type = Some(this);
        assert_eq!(f.syms.class(f.c).this_type, Some(this));
        f.release();
        assert_eq!(f.syms.class(f.c).parents, vec![own]);
        assert_eq!(f.syms.class(f.c).this_type, Some(this));
    }

    /// A completion that completes another within it (the lock taken again, a record staged)
    /// reads the outer record half made, its parents in place and its self type not yet, as
    /// `complete_class_def` leaves it; after the outermost release the worker reads both whole.
    #[test]
    fn a_recursive_completion_reads_the_record_half_made() {
        let mut f = Forked::new();
        let own = f.ty(9);
        f.take();
        let base = f.ty(9);
        f.syms.class_mut(f.c).parents = vec![base];
        f.take();
        let d = f.syms.new_class(Name(4), ClassKind::Class, 0, Owner::Package(ROOT_PKG), FileId(0), None, Span::default());
        assert!(d.0 < crate::arena::LOCAL_BASE, "a record the hold pushes is the shared region's");
        assert_eq!(f.syms.class(f.c).parents, vec![base]);
        assert_eq!(f.syms.class(f.c).this_type, None);
        f.syms.class_mut(d).parents = vec![base];
        f.release();
        f.syms.class_mut(f.c).this_type = Some(base);
        f.release();
        assert_eq!(f.syms.class(f.c).parents, vec![own]);
        assert_eq!(f.syms.class(f.c).this_type, Some(own));
        assert_eq!(f.syms.class(d).parents, vec![own]);
    }

    /// Each publication is a version of its own: the worker reads the latest, translated once
    /// per version and not at every read, the copies of the versions before freed once no read
    /// can hold them.
    #[test]
    fn reads_follow_successive_publications_and_translate_each_once() {
        let mut f = Forked::new();
        let own = [f.ty(9), f.ty(10)];
        let mut base = Vec::new();
        for (k, &own) in own.iter().enumerate() {
            f.take();
            base.push(f.ty(9 + k as u32));
            f.syms.class_mut(f.c).parents = base.clone();
            f.release();
            assert_eq!(f.syms.class(f.c).parents[k], own);
            assert_eq!(f.syms.class(f.c).parents[k], own);
        }
        let counts = f.syms.classes.view_counts();
        assert_eq!((counts.records, counts.copies, counts.retired, counts.translated), (1, 1, 1, 2));
        f.take();
        f.release();
        assert_eq!(f.syms.classes.view_counts().retired, 0);
    }

    /// A signature, an alias's record and a constructor's parameters are read in the view as a
    /// class's parents are.
    #[test]
    fn signatures_aliases_and_constructors_read_in_the_view() {
        let mut f = Forked::new();
        let own = f.ty(9);
        let (s, a) = (f.s, f.a);
        f.take();
        let base = f.ty(9);
        f.syms.sym_mut(s).sig = Some(Arc::new(MethodSig::value(base)));
        let alias = f.syms.aliases.get_mut(a.0);
        alias.rhs = base;
        alias.bounds = Some((NOTHING, base));
        let param = ParamSig { name: Name(7), ty: base, by_name: false, repeated: false, has_default: false, sym: s };
        f.syms.class_mut(f.c).ctor = vec![ClauseSig { params: vec![param], is_using: false, is_implicit: false }];
        f.release();
        assert_eq!(f.syms.sym(s).sig.as_ref().unwrap().ret, own);
        assert_eq!(f.syms.sig(s).ret, own);
        assert_eq!((f.syms.alias(a).rhs, f.syms.alias(a).bounds), (own, Some((NOTHING, own))));
        assert_eq!(f.syms.class(f.c).ctor[0].params[0].ty, own);
    }

    /// The version the fork left of a shared record is read as it is, its types every worker's,
    /// with no translation and nothing kept.
    #[test]
    fn the_version_the_fork_left_is_read_as_it_is() {
        let f = Forked::new();
        let read: *const ClassInfo = &*f.syms.class(f.c);
        assert!(std::ptr::eq(read, f.syms.class_raw(f.c)));
        assert_eq!(f.syms.classes.view_counts().translated, 0);
        assert_eq!(f.syms.classes.view_counts().records, 0);
    }

    /// A worker's own record is its own: read as it is, whatever its types.
    #[test]
    fn a_workers_own_record_is_not_viewed() {
        let mut f = Forked::new();
        let own = f.ty(9);
        let d = f.syms.new_class(Name(4), ClassKind::Anon, 0, Owner::Local, FileId(0), None, Span::default());
        assert!(d.0 >= crate::arena::LOCAL_BASE);
        f.syms.class_mut(d).parents = vec![own];
        assert_eq!(f.syms.class(d).parents, vec![own]);
        assert_eq!(f.syms.classes.view_counts().translated, 0);
    }

    /// Two workers over one fork,
    /// each on a thread of its own: worker 1, on a thread it leaves before worker 0 reads, makes a
    /// type parameter bounded by a type of its overlay and, when `escape`, lets it escape (what the
    /// release of its hold of the loader's lock does, `escape_own`); worker 0, this thread, reads
    /// the record by its id with the views on.
    #[cfg(debug_assertions)]
    struct Peers {
        store: Box<TypeStore>,
        syms: Symbols,
        peer: Symbols,
        lock: ReentrantLock,
        p: TParamId,
        bound: TypeId,
    }

    #[cfg(debug_assertions)]
    impl Peers {
        fn new(escape: bool) -> Peers {
            let store = Box::new(TypeStore::new());
            store.class(ClassId(1), &[]);
            store.set_exclusive(false);
            let mut syms = Symbols::new();
            syms.syms.fork();
            syms.classes.fork();
            syms.tparams.fork();
            syms.aliases.fork();
            syms.pkgs.fork();
            syms.overloads.fork();
            assert!(store.fork_overlays(2));
            let mut peer = syms.attach(1);
            let (p, bound) = std::thread::scope(|s| {
                let (store, peer) = (&*store, &mut peer);
                s.spawn(move || {
                    store.enter_overlay(1);
                    let bound = store.class(ClassId(9), &[]);
                    let p = peer.new_tparam(Name(3), 0);
                    peer.tparams[p.idx()].upper = bound;
                    if escape {
                        peer.escape_own();
                    }
                    store.leave_overlay();
                    (p, bound)
                })
                .join()
                .unwrap()
            });
            store.enter_overlay(0);
            syms.view_records(true);
            Peers { store, syms, peer, lock: ReentrantLock::new(), p, bound }
        }
    }

    #[cfg(debug_assertions)]
    impl Drop for Peers {
        fn drop(&mut self) {
            while crate::shared::lock_depth() > 0 {
                self.lock.unlock();
            }
            self.syms.view_records(false);
            self.store.leave_overlay();
        }
    }

    #[cfg(debug_assertions)]
    fn panic_text(r: std::thread::Result<impl Sized>) -> String {
        match r {
            Ok(_) => String::new(),
            Err(e) => e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(),
        }
    }

    /// The enforcement at two threads, the refusal: a peer's record read raw hands worker 0 a
    /// type of worker 1's overlay, which its first read through the store refuses.
    #[cfg(debug_assertions)]
    #[test]
    fn a_peers_record_read_raw_is_refused() {
        let f = Peers::new(true);
        let raw = f.syms.tparams.raw(f.p.0).upper;
        assert_eq!(raw, f.bound);
        let refused = panic_text(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.store.get(raw))));
        assert!(refused.contains("from a peer's overlay"), "the peer's type read raw was not refused: {:?}", refused);
    }

    /// The enforcement at two threads, the acceptance: the same record read through worker 0's
    /// view hands over its bound imported, worker 0's own type of that structure, which the store
    /// reads without a refusal; the record is translated at its first read and kept by its id.
    #[cfg(debug_assertions)]
    #[test]
    fn a_peers_record_read_through_the_view_is_imported() {
        let f = Peers::new(true);
        let upper = f.syms.tparam(f.p).upper;
        assert_ne!(upper, f.bound);
        assert_eq!(crate::arena::worker_of(upper.0), Some(0));
        assert!(matches!(f.store.get(upper), crate::types::Type::Class(ClassId(9), _)));
        assert_eq!(f.syms.tparam(f.p).upper, upper);
        let counts = f.syms.tparams.view_counts();
        assert_eq!((counts.peer_reads, counts.peer_translated, counts.peer_copies), (2, 1, 1));
    }

    /// The loader's lock holder reads a peer's record through its view too, the base's: the
    /// bound exported.
    #[cfg(debug_assertions)]
    #[test]
    fn a_peers_record_read_by_the_holder_is_exported() {
        let mut f = Peers::new(true);
        f.lock.lock();
        f.syms.tparams.lock_taken();
        let upper = f.syms.tparam(f.p).upper;
        assert!(upper.0 < crate::arena::LOCAL_BASE, "the holder's read of a peer's bound is the base's");
        assert!(matches!(f.store.get(upper), crate::types::Type::Class(ClassId(9), _)));
        f.syms.tparams.lock_released();
        f.lock.unlock();
    }

    /// A peer reads a record only once it has escaped its owner: an id that reaches the reader
    /// without the release that publishes it is refused.
    #[cfg(debug_assertions)]
    #[test]
    fn a_peers_record_read_before_it_escaped_is_refused() {
        let f = Peers::new(false);
        let refused = panic_text(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.syms.tparam(f.p).upper)));
        assert!(refused.contains("before it escaped"), "the unpublished record's read was not refused: {:?}", refused);
    }

    /// Once a peer has read a record, its owner never writes it: the peers' views keep what they
    /// read by the id.
    #[cfg(debug_assertions)]
    #[test]
    fn an_owners_write_after_a_peers_read_is_refused() {
        let mut f = Peers::new(true);
        let _ = f.syms.tparam(f.p).upper;
        let p = f.p;
        let refused = panic_text(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.peer.tparams[p.idx()].upper = crate::tir::NO_TYPE)));
        assert!(refused.contains("after a peer read it"), "the owner's write after a peer's read was not refused: {:?}", refused);
    }

    /// Every mutable accessor of an owner's records refuses a record a peer has read, not the
    /// indexed write alone: a write through `iter_mut`, `rfind_mut` or `last_mut` would change
    /// the record under the peer's translation of it.
    #[cfg(debug_assertions)]
    #[test]
    fn an_owners_write_through_its_mutable_accessors_after_a_peers_read_is_refused() {
        let mut f = Peers::new(true);
        let _ = f.syms.tparam(f.p).upper;
        for (how, write) in [
            ("iter_mut", (|p: &mut Symbols| p.tparams.iter_mut().for_each(|t| t.upper = crate::tir::NO_TYPE)) as fn(&mut Symbols)),
            ("rfind_mut", |p: &mut Symbols| p.tparams.rfind_mut(|_| true).into_iter().for_each(|t| t.upper = crate::tir::NO_TYPE)),
            ("last_mut", |p: &mut Symbols| p.tparams.last_mut().into_iter().for_each(|t| t.upper = crate::tir::NO_TYPE)),
        ] {
            let refused = panic_text(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| write(&mut f.peer))));
            assert!(refused.contains("after a peer read it"), "the owner's write through {} after a peer's read was not refused: {:?}", how, refused);
        }
        assert_eq!(f.peer.tparams[f.p.idx()].upper, f.bound);
    }

    /// The release's check refuses a record the holder is about to publish with a type of the
    /// worker's overlay, which a site forgot to export.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "published into a class's info")]
    fn an_unexported_type_in_a_publication_is_refused() {
        let mut f = Forked::new();
        let own = f.ty(9);
        f.take();
        f.syms.class_mut(f.c).parents = vec![own];
        let store: &TypeStore = &f.store;
        f.syms.each_unpublished_type(&mut |what, t| crate::types::view::published(store, what, t));
    }
}
