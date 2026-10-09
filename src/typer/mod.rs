mod accessors;
mod anon;
mod apply;
mod arity;
pub mod bundle;
pub mod capture;
mod capture_census;
pub mod check;
mod complete;
mod conversions;
pub mod deps;
mod depfun;
mod deferred;
mod derive;
mod exceptions;
pub mod exports;
pub mod export_plan;
mod expr;
mod implicits;
pub mod index;
pub mod incremental;
mod inline;
mod inline_definition;
mod substitution;
mod interop;
pub mod loader;
mod matchtypes;
mod measured;
mod merge;
pub mod thread;
mod members;
mod namedtuple;
mod namer;
mod overload;
mod pat;
pub mod prep;
pub mod prims;
pub mod profile;
mod quoted;
mod resolve;
pub use apply::{ArgList, ArgSrc};
pub use expr::WITHHELD_TEMPLATE;
pub use inline::MAX_INLINES;
pub use measured::reach_mode_error;
pub use resolve::{TermRef, TypeRef};
mod reflective;
mod restrict;
pub mod setters;
mod show;
mod signature;
pub(crate) mod site;
mod space;
mod state;
pub mod stdlib;
mod subtype;
mod tailrec;
pub mod unused;

use crate::ast::{Ast, Asts, ListRef};
use crate::intern::{FxMap, Interner, Name};
use crate::source::{Diagnostics, FileId, Sources, Span};
use crate::symbols::*;
use crate::tir::{FunId, InlineDefinition, JsImport, Program, TExprId, TPatId, TStmt};
use crate::types::*;

#[derive(Clone)]
pub struct Builtins {
    pub int: ClassId,
    pub long: ClassId,
    pub double: ClassId,
    pub byte: ClassId,
    pub short: ClassId,
    pub float: ClassId,
    pub boolean: ClassId,
    pub string: ClassId,
    pub char: ClassId,
    pub unit: ClassId,
    pub array: ClassId,
    /// The type of `null`, below every reference type, and `AnyRef`, above them.
    pub null: ClassId,
    pub any_ref: ClassId,
    /// The top of the value types: the primitives and the value classes.
    pub any_val: ClassId,
    pub t_int: TypeId,
    pub t_long: TypeId,
    pub t_double: TypeId,
    pub t_byte: TypeId,
    pub t_short: TypeId,
    pub t_float: TypeId,
    pub t_boolean: TypeId,
    pub t_string: TypeId,
    pub t_char: TypeId,
    pub t_unit: TypeId,
    pub t_null: TypeId,
    pub t_any_ref: TypeId,
    pub t_any_val: TypeId,
    pub functions: Vec<Option<ClassId>>,
    pub context_functions: Vec<Option<ClassId>>,
    /// `=> A` as a function type's parameter (`ByName`), made on first use.
    pub by_name: Option<ClassId>,
    /// `A*` as reflection shows a repeated parameter's type, scalac's `<repeated>[A]`, made on
    /// first use.
    pub repeated: Option<ClassId>,
    pub tuples: Vec<Option<ClassId>>,
    pub seq: Option<ClassId>,
    pub string_context: Option<ClassId>,
    pub partial_function: Option<ClassId>,
    pub conversion: Option<ClassId>,
    /// `<:<` and `=:=`, whose values the compiler supplies and which are functions at run time.
    pub sub_evidence: Option<ClassId>,
    pub eq_evidence: Option<ClassId>,
    /// The std types with `CanEqual` instances of their own, and the type class itself.
    pub option: Option<ClassId>,
    /// `scala.NamedTuple.NamedTuple`, the opaque type behind `(name: T, ...)`.
    pub named_tuple: Option<ClassId>,
    pub either: Option<ClassId>,
    pub set: Option<ClassId>,
    pub map: Option<ClassId>,
    pub can_equal: Option<ClassId>,
    pub value_of: Option<ClassId>,
    /// `scala.Selectable`, whose subclasses take structural calls.
    pub selectable: Option<ClassId>,
    /// `java.lang.Number`, when a library defines it: scalac lets a numeric primitive compare
    /// with its subclasses.
    pub number: Option<ClassId>,
    /// `scala.scalajs.js.Dynamic`, when the Scala.js library is compiled in: its unknown members
    /// are emitted verbatim.
    pub js_dynamic: Option<ClassId>,
    /// `java.lang.Throwable`, what `throw` takes and `catch` matches on, and
    /// `js.JavaScriptException`, which stands for a caught value that is no `Throwable`.
    pub throwable: Option<ClassId>,
    pub js_exception: Option<ClassId>,
    /// `*:`, the tuple cons: `h *: t` with a known tail is the tuple one longer, with an
    /// unknown one it stays an application of this trait.
    pub cons_tuple: Option<ClassId>,
    /// `Product`, which case classes, case objects, enum cases and tuples are without extending
    /// it, and the tuple traits.
    pub product: Option<ClassId>,
    /// `Product` as a type; `ERROR` without the standard library.
    pub t_product: TypeId,
    pub t_singleton: TypeId,
    pub t_equals: TypeId,
    /// `scala.reflect.Enum`, which enum classes and cases are without extending it.
    pub reflect_enum: Option<ClassId>,
    pub t_enum: TypeId,
    pub tuple_trait: Option<ClassId>,
    pub non_empty_tuple: Option<ClassId>,
    pub empty_tuple: Option<ClassId>,
    /// `head`, `tail`, `size`, `apply`, `++`, `zip`, `toList`: the members of every tuple
    /// (`arity.rs`).
    pub tuple_members: Vec<Name>,
    /// A bit per `tuple_members` name, by its number modulo 256: a name of no bit is none of
    /// them, which every other member call asks (`Builtins::may_be_tuple_member`).
    pub tuple_member_bits: [u64; 4],
    /// The members of scala-library's `Predef`, which the lean std defines in package `scala`.
    pub predef_names: std::collections::HashSet<Name, crate::intern::FxBuild>,
    /// The concrete members of `Product` with the std functions that stand in for them on a
    /// receiver without the trait's body (`derive::product_helper`).
    pub product_helpers: Vec<(Name, SymId)>,
    pub scala_pkg: PkgId,
    /// The numeric rank of the first type ids (`prims::R_BYTE` to `R_DOUBLE`), which the
    /// primitive types are among; `NO_RANK` for the others.
    pub num_rank: [u8; NUM_RANK_TABLE],
}

impl Builtins {
    /// Whether `name` may be one of `tuple_members`: false for most names, at a bit's cost.
    #[inline]
    pub fn may_be_tuple_member(&self, name: Name) -> bool {
        self.tuple_member_bits[(name.0 / 64 % 4) as usize] & 1 << (name.0 % 64) != 0
    }
}

pub const NUM_RANK_TABLE: usize = 32;
pub const NO_RANK: u8 = u8::MAX;
pub const MARK_UNCHECKED: u8 = 1;
pub const MARK_CAST: u8 = 2;
/// `(e: Boolean)`, an ascription to a type that is no literal type: no constant as the
/// condition of an `if` to scalac, whatever `e` is, while an operation over it sees the
/// constant; what fills a hole or stands for an inline parameter there is a constant again.
pub const MARK_ASCRIBED: u8 = 4;
/// A call the typer wrote as its receiver or as a value (`x.asInstanceOf[T]`, `s.toString` of
/// a string): no constant to scalac, wherever it stands.
pub const MARK_CALL: u8 = 8;
/// A cast or an ascription gave the node another type: it no longer stands as written
/// for what it named (a parameter of an inline body, `Worker::proxy_declared`).
pub const MARK_RETYPED: u8 = 16;
/// The closure of an ascription to a context function type (`(1: (Int ?=> Int))`): dotty's tree
/// is the ascription around it, no contextual closure (`isContextualClosure`), which a value
/// expected of another type applies to the givens in scope as it applies any other.
pub const MARK_ASCRIBED_CLOSURE: u8 = 32;

#[derive(Clone)]
pub enum Frame {
    Locals {
        names: Vec<(Name, SymId)>,
        tparams: Vec<(Name, TParamId)>,
        givens: Vec<SymId>,
        /// The classes defined in the block.
        classes: Vec<(Name, ClassId)>,
        /// The type aliases defined in the block.
        aliases: Vec<(Name, AliasId)>,
    },
    Class(ClassId),
}

impl Frame {
    pub fn locals() -> Frame {
        Frame::Locals { names: Vec::new(), tparams: Vec::new(), givens: Vec::new(), classes: Vec::new(), aliases: Vec::new() }
    }
}

#[derive(Clone)]
pub struct Env {
    pub file: FileId,
    pub frames: Vec<Frame>,
    /// Imports of the enclosing blocks and bodies, outermost scope first. Within a scope the
    /// wildcards come first, in source order: lookups go backwards, so a named import wins over
    /// a wildcard and a later import is met before an earlier one.
    pub imports: Vec<ResolvedImport>,
}

impl Env {
    /// Adds an import to the scope that starts at `scope`, keeping the scope's wildcards first.
    pub fn push_import(&mut self, scope: usize, mut imp: ResolvedImport) {
        imp.depth = self.frames.len() as u32;
        match imp.name {
            Some(_) => self.imports.push(imp),
            None => {
                let wildcards = self.imports[scope..].iter().take_while(|i| i.name.is_none()).count();
                self.imports.insert(scope + wildcards, imp);
            }
        }
    }
}

/// A selection or an operator of a library body being typed: the selection's node, whose tree's
/// point is its name's, and the application it heads, whose point is the call's (an inline
/// expansion's diagnostic moves to the call, as scalac's does).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BodyNode {
    pub file: FileId,
    pub selection: crate::ast::ExprId,
    pub call: crate::ast::ExprId,
}

/// The extension method whose body is being typed.
#[derive(Clone, Copy)]
pub struct ExtScope {
    pub owner: Owner,
    pub file: FileId,
    pub group: u32,
    pub receiver: SymId,
}

/// A worker's type variables, by index (`TVarId::index`); a variable of another worker (its
/// tag differs, and it is not one of the signature phase's, which every worker starts with)
/// reads as open and unbounded, and a change to it is dropped: another worker's variable
/// that reached this one through a published type is dead, as it would be in one worker's
/// table once its application ended. After the merge the merging worker holds every other
/// worker's table as well (`absorb`), so that a type a body recorded (an expression's, for the
/// JVM backend) reads its variables' instances as one worker's table would.
pub struct TVars {
    own: Vec<TVarInfo>,
    tag: u32,
    /// The id of the variable at index 0 of this worker's own: the tag in its place.
    base: u32,
    /// The signature phase's variables, tagged 0, which every worker's table starts with.
    prefix: u32,
    dummy: TVarInfo,
    /// The other workers' tables by tag, after the merge, whole as `absorb` takes them until the
    /// merge keeps what its records reach of them (`keep_others`).
    others: Vec<Vec<TVarInfo>>,
    kept: Option<FxMap<u32, TVarInfo>>,
}

impl TVars {
    pub fn new() -> TVars {
        TVars { own: Vec::new(), tag: 0, base: 0, prefix: 0, dummy: TVarInfo::default(), others: Vec::new(), kept: None }
    }

    /// Worker `tag`'s table over the signature phase's `prefix` variables of `main`.
    pub fn attach(main: &TVars, tag: u32, prefix: u32) -> TVars {
        TVars { own: main.own[..prefix as usize].to_vec(), tag, base: TVarId::tagged(tag, 0).0, prefix, dummy: TVarInfo::default(), others: Vec::new(), kept: None }
    }

    /// The instance of `v`, when it has one: this worker's variable or, after the merge,
    /// another worker's.
    pub fn inst(&self, v: TVarId) -> Option<TypeId> {
        self[v].inst
    }

    /// Takes over worker `other`'s table at the merge.
    pub fn absorb(&mut self, other: TVars) {
        let tag = other.tag as usize;
        if self.others.len() <= tag {
            self.others.resize_with(tag + 1, Vec::new);
        }
        self.others[tag] = other.own;
    }

    /// Every variable's instance and bounds, this worker's and the others' it took over (or
    /// kept of them).
    pub fn each_type(&self, mut f: impl FnMut(TypeId)) -> usize {
        let mut n = 0;
        let others: Box<dyn Iterator<Item = &TVarInfo>> = match &self.kept {
            Some(kept) => Box::new(kept.values()),
            None => Box::new(self.others.iter().flatten()),
        };
        for v in self.own.iter().chain(others) {
            n += 1;
            for &t in v.inst.iter().chain(&v.lower).chain(&v.upper) {
                f(t);
            }
        }
        n
    }

    /// Whether `v` is this worker's own or the signature phase's, which its table holds.
    pub fn is_own(&self, v: TVarId) -> bool {
        self.slot(v).is_some()
    }

    /// Another worker's variable as `absorb` took its table over, before `keep_others`.
    pub fn other_info(&self, v: TVarId) -> Option<&TVarInfo> {
        self.others.get(v.tag() as usize).and_then(|t| t.get(v.index()))
    }

    /// The variables of the other workers' tables `absorb` took over.
    pub fn others_len(&self) -> usize {
        self.others.iter().map(|t| t.len()).sum()
    }

    /// The other workers' variables the merge kept (`keep_others`).
    pub fn kept_len(&self) -> usize {
        self.kept.as_ref().map_or(0, |k| k.len())
    }

    /// Of the other workers' tables, the variables the merge's records reach (`kept`, with their
    /// types under the new ids), the rest dropped: another worker's variable no kept record
    /// reaches reads open and unbounded, as during the bodies.
    pub fn keep_others(&mut self, kept: Vec<(TVarId, TVarInfo)>) {
        // Millions of entries at sixteen workers, freed off the merge's path.
        let others = std::mem::take(&mut self.others);
        if others.iter().map(Vec::len).sum::<usize>() > 1 << 16 {
            crate::alloc::spawn(move || drop(others));
        }
        self.kept = Some(kept.into_iter().map(|(v, info)| (v.0, info)).collect());
    }

    #[cold]
    #[inline(never)]
    fn other(&self, v: TVarId) -> &TVarInfo {
        match &self.kept {
            Some(kept) => kept.get(&v.0).unwrap_or(&self.dummy),
            None => self.others.get(v.tag() as usize).and_then(|t| t.get(v.index())).unwrap_or(&self.dummy),
        }
    }

    #[cold]
    #[inline(never)]
    fn other_mut(&mut self, v: TVarId) -> &mut TVarInfo {
        let found = match &mut self.kept {
            Some(kept) => kept.get_mut(&v.0),
            None => self.others.get_mut(v.tag() as usize).and_then(|t| t.get_mut(v.index())),
        };
        match found {
            Some(info) => info,
            None => {
                self.dummy = TVarInfo::default();
                &mut self.dummy
            }
        }
    }

    /// Where the variable stands in this worker's table: its index when it is this worker's
    /// (one subtraction and the comparison that bounds the index), or the signature phase's.
    #[inline(always)]
    /// Whether `v` is one of this worker's variables made after the first `len`.
    pub(super) fn made_since(&self, v: TVarId, len: usize) -> bool {
        let i = v.0.wrapping_sub(self.base) as usize;
        i >= len && i < self.own.len()
    }

    fn slot(&self, v: TVarId) -> Option<usize> {
        let i = v.0.wrapping_sub(self.base) as usize;
        if i < self.own.len() {
            Some(i)
        } else if v.0 < self.prefix {
            Some(v.0 as usize)
        } else {
            None
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.own.len()
    }

    /// The id of the variable at `index` of this worker's table.
    #[inline]
    pub fn id(&self, index: usize) -> TVarId {
        TVarId::tagged(self.tag, index)
    }

    pub fn push(&mut self, info: TVarInfo) {
        self.own.push(info);
    }

    /// Drops the variables made since the table held `len`, which nothing refers to any more: a
    /// completion's probes, rolled back (`Worker::in_query`). A variable made later takes the
    /// index, and the types made over the dropped one are its types.
    pub fn truncate(&mut self, len: usize) {
        self.own.truncate(len);
    }

    pub fn last_mut(&mut self) -> Option<&mut TVarInfo> {
        self.own.last_mut()
    }
}

impl std::ops::Index<TVarId> for TVars {
    type Output = TVarInfo;
    #[inline(always)]
    fn index(&self, v: TVarId) -> &TVarInfo {
        match self.slot(v) {
            Some(i) => unsafe { self.own.get_unchecked(i) },
            None => self.other(v),
        }
    }
}

impl std::ops::IndexMut<TVarId> for TVars {
    #[inline(always)]
    fn index_mut(&mut self, v: TVarId) -> &mut TVarInfo {
        match self.slot(v) {
            Some(i) => unsafe { self.own.get_unchecked_mut(i) },
            None => self.other_mut(v),
        }
    }
}

impl std::ops::Index<usize> for TVars {
    type Output = TVarInfo;
    #[inline]
    fn index(&self, i: usize) -> &TVarInfo {
        &self.own[i]
    }
}

impl std::ops::IndexMut<usize> for TVars {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut TVarInfo {
        &mut self.own[i]
    }
}

#[derive(Clone, Default)]
pub struct TVarInfo {
    pub inst: Option<TypeId>,
    pub lower: Vec<TypeId>,
    pub upper: Vec<TypeId>,
    /// The index of the first variable of the application that was being typed when this one
    /// was made (older variables belong to enclosing applications) in the low 24 bits, which an
    /// index takes at most (`TVarId::TAG_SHIFT`), and the top 8 bits of `shown`'s 24 above them:
    /// the record stays 72 bytes.
    base_shown: u32,
    pub solving: bool,
    /// Stands for the join of the branches typed so far (`Worker::branch_guide`).
    pub guide: bool,
    /// The type parameter the variable was made for, shown while it is open.
    pub name: Option<Name>,
    shown_low: u16,
}

impl TVarInfo {
    pub fn open(base: u32, shown: u32) -> TVarInfo {
        debug_assert!(base >> TVarId::TAG_SHIFT == 0 && shown >> TVarId::TAG_SHIFT == 0);
        TVarInfo { base_shown: base | (shown >> 16) << TVarId::TAG_SHIFT, shown_low: shown as u16, ..Default::default() }
    }

    pub fn base(&self) -> u32 {
        self.base_shown & ((1 << TVarId::TAG_SHIFT) - 1)
    }

    /// The variable's place among the ones the typing that made it made (`Worker::var_frames`),
    /// plus one, which a message shows it by: a function of that body's, class's or item's own
    /// order, whichever worker typed it and whatever it typed before. 0 where none is kept.
    pub fn shown(&self) -> u32 {
        (self.base_shown >> TVarId::TAG_SHIFT) << 16 | self.shown_low as u32
    }
}

const _: () = assert!(std::mem::size_of::<TVarInfo>() == 72);

pub enum Undo {
    Inst(TVarId),
    Lower(TVarId),
    Upper(TVarId),
}

#[derive(Clone, Copy)]
pub enum Redo {
    Inst(TVarId, TypeId),
    Lower(TVarId, TypeId),
    Upper(TVarId, TypeId),
}

#[derive(Clone, Copy)]
pub struct ResolvedImport {
    pub name: Option<Name>,
    pub target: ImportTarget,
    /// What a wildcard leaves out (`{a as _, b as c, *}`), in `Worker::import_hidden`.
    pub hidden: ListRef,
    /// How many frames enclose the import, 0 at the top of a file. Its givens are as near as
    /// the givens of the innermost of those frames.
    pub depth: u32,
    /// Where an explicit import of `Predef` stands, which takes Predef's root import away from
    /// the code after it.
    pub unimports_predef: Option<u32>,
    /// The selector the import came of and a `given T`'s type, which a name resolved through it
    /// marks used (`unused.rs`), in `Worker::import_sels`; `SelRef::NONE` outside the check and
    /// for what no selector of the program wrote. Kept apart, so that the record the lookups copy
    /// stays as small as it was.
    pub sel: unused::SelRef,
}

/// A stable val an import selects on, an index into `Worker::import_values`: the val and the
/// object it was reached through, which an inherited val (`O.v` of a trait) is read on, or the
/// enclosing class whose instance it is read on (`C.this.v`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ValueImport(pub u32);

impl crate::arena::Record for Option<std::sync::Arc<Vec<ResolvedImport>>> {}

#[derive(Clone, Copy)]
pub enum ImportTarget {
    PkgMember(PkgId, Name),
    PkgAll(PkgId),
    ClassMember(ClassId, Name),
    ClassAll(ClassId),
    /// `import p.given` and `import Obj.given`: the givens and nothing else, where a wildcard
    /// brings in everything but the givens.
    PkgGivens(PkgId),
    ClassGivens(ClassId),
    /// A stable val of a package or an object, the prefix of the import (`import dom.window.x`);
    /// named selectors turn it into `ValueMember`.
    ValueAll(ValueImport),
    ValueMember(ValueImport, Name),
    /// `import v.given` of such a val.
    ValueGivens(ValueImport),
    /// `import Predef.{x as _}`: binds nothing, and takes Predef's root import away.
    UnimportPredef,
    /// An import whose path did not resolve: it binds nothing, and a name it could bind is
    /// unknown rather than missing (`Worker::unknown_through_import`).
    Unresolved,
}

/// The memos a worker keeps apart per view (`Worker::swap_view_memos`).
#[derive(Default)]
pub struct ViewMemos {
    matches: matchtypes::MatchMemos,
    given_fast: FxMap<(TypeId, FileId, u64), (std::sync::Arc<implicits::GivenResult>, Box<[u32]>)>,
}

pub struct Worker<'a> {
    pub asts: &'a Asts,
    pub interner: &'a Interner,
    pub syms: Symbols,
    /// The type store, one for every worker (`types.rs`: its readers take no lock).
    pub types: std::sync::Arc<TypeStore>,
    pub diags: Diagnostics,
    pub prog: Program,
    pub b: Builtins,
    pub env: Env,
    pub def_syms: crate::arena::FileMaps<SymId>,
    pub def_classes: crate::arena::FileMaps<ClassId>,
    pub def_aliases: crate::arena::FileMaps<AliasId>,
    pub file_pkgs: crate::arena::FileVec<PkgId>,
    pub file_imports: crate::arena::FileVec<Option<std::sync::Arc<Vec<ResolvedImport>>>>,
    /// The names the wildcard imports leave out, by the ranges the imports hold: shared, since
    /// a pseudo file's imports are every worker's.
    pub import_hidden: std::sync::Arc<crate::shared::SlabVec<Name>>,
    /// The selectors of the imports the unused-import check resolved, each with a `given T`'s
    /// type, by `ResolvedImport::sel`: shared as `import_hidden` is, appended under the loader's
    /// lock.
    pub import_sels: std::sync::Arc<crate::shared::SlabVec<(unused::Sel, Option<TypeId>)>>,
    /// The imports in the body of a class, each with the offset at which it stands.
    pub class_imports: crate::arena::Layered<ClassId, std::sync::Arc<[(u32, ResolvedImport)]>>,
    pub ext_scope: Option<ExtScope>,
    pub tvars: TVars,
    /// The first variable of the innermost method application being typed.
    pub app_base: u32,
    pub trail: Vec<Undo>,
    /// The attempts under way (`state.rs`).
    pub attempts: state::Attempts,
    /// The argument caches of the applications several attempts apply (`state::ArgCache`).
    attempt_caches: Vec<state::ArgCache>,
    pub fun_of_sym: crate::arena::Layered<SymId, FunId>,
    /// The string concatenations that are interpolations, which scalac does not fold.
    pub interpolations: FxMap<TExprId, ()>,
    /// The members whose bodies are being typed, innermost last: a macro that asks for the
    /// tree of the method it is called in gets the signature, not the body typed again.
    pub bodies_in_progress: Vec<SymId>,
    /// The typings under way that number their variables apart (`var_frame_begin`): per frame,
    /// the variables' count at its start and how many the frames nested in it made since.
    pub var_frames: Vec<(u32, u32)>,
    pub val_init: crate::arena::Layered<SymId, TExprId>,
    pub class_done: crate::arena::Layered<ClassId, ()>,
    pub implicit_depth: u32,
    /// The depth of the given search under way that is a search alone (`resolve_given_search_only`):
    /// the synthesis a failed search falls back to is left out at that depth.
    pub search_only_depth: Option<u32>,
    /// Candidates a top-level given search may still try, scalac's implicit search limit.
    pub implicit_budget: u32,
    /// The first ambiguity met by the given search that is under way. A search that fails
    /// because of it reports this instead of a missing instance.
    pub given_ambiguity: Option<String>,
    /// The candidates of the search under way whose inline expansion failed, with the first
    /// error each: discarded quietly like scalac's, named when nothing else is found.
    pub failed_givens: Vec<(SymId, String)>,
    /// The parameterized givens being instantiated, innermost last.
    pub given_stack: Vec<implicits::OpenGiven>,
    /// Whether the given search under way is for a by-name using parameter.
    pub search_byname: bool,
    /// Set while a definition's annotation is typed for its pickle.
    pub typing_annotation: bool,
    /// Set while the arguments of a constructor in an annotation are typed: scalac lifts none of
    /// them (`Applications.isAnnotConstr`), where it lifts an ordinary call's.
    pub annotation_args: bool,
    /// The last target measured for the divergence check: the type, its size, its classes and
    /// its shape with open variables blanked out.
    pub measured_target: Option<(TypeId, u32, u64, TypeId)>,
    /// The alias given whose right-hand side is being typed, which its own search leaves out.
    pub defining: Option<SymId>,
    /// The conversions of a package or class, indexed by member name and result class on the
    /// first conversion lookup in that scope.
    pub conversion_indexes: FxMap<conversions::ScopeKey, std::sync::Arc<conversions::ConversionIndex>>,
    /// Set for the second round of an overload resolution, in which an argument that a
    /// conversion takes to the parameter type is compatible with it.
    pub view_compat: bool,
    /// The next argument typed is a varargs spread (`f(xs*)`): an array typed for it is marked
    /// as spread as it is written (`Program::spread_bits`). Taken by the argument it is for.
    pub spread_arg: bool,
    /// Under a member's application (`apply::apply_member_or_extension`, `logging` deep): each
    /// type mismatch reported, by its diagnostic's index and span, with the type it required.
    pub mismatches: Vec<(usize, Span, TypeId)>,
    pub logging: u32,
    /// How many method applications are under way (`apply::apply_method_in`), and the depth at
    /// which a member's retry on the qualifier runs (`apply::retry_on_qualifier`): the application
    /// a try of it enters, one deeper, is the retry's own, which stopping at its first plain clause
    /// leaves the lists after it to the next try; one nested in its arguments is not.
    pub app_depth: u32,
    pub retry_depth: Option<u32>,
    /// The arguments a member's retry on its qualifier takes as its test typed them
    /// (`apply::retry_on_qualifier`, dotty's `FunProto` cache): adapted as dotty's `adapt` adapts
    /// a typed tree, never wrapped in a context function, a closure converted to a SAM.
    pub retry_typed: FxMap<TExprId, ()>,
    /// What the member's application under way in `apply::apply_member_or_extension` typed of
    /// its arguments, for its retry; a spare one is kept in `arg_caches`.
    pub arg_cache: Option<Box<apply::ArgCache>>,
    pub arg_caches: Vec<Box<apply::ArgCache>>,
    /// The first elements of the tuples an infix operation's operand was flattened from
    /// (`a op (b, c)`, dotty's `ApplyKind.InfixTuple`), by file.
    pub infix_tuples: FxMap<(crate::source::FileId, crate::ast::ExprId), ()>,
    /// Under a member's retry, set where `try_extensions` applied a lexical extension it
    /// selected (dotty's `tryExtension`), whose failure to apply opens no conversion search.
    pub lexical_selected: Option<bool>,
    /// Set where `pick_extension` took the first of several overloads the arguments did not
    /// decide between, dotty's failed overload resolution (`lexical_among_imports`).
    pub ext_undecided: bool,
    /// Set (`Some(false)`) by a member's application around the typing of a function literal
    /// it was given (`apply::type_arg_recorded`): `type_lambda` takes it, and where the formal
    /// gives a parameter no type it reports the missing type and types none of the literal,
    /// leaving `Some(true)`, so that the retry's typing is the literal's first.
    pub untyped_lambda: Option<bool>,
    /// Set for the member selection on a converted receiver, which no second conversion may
    /// serve.
    pub no_receiver_conversion: bool,
    /// The member a library body's selection declares, for the selection of
    /// that name on that typed receiver: it is the member the call takes, not the name's
    /// overloads.
    pub declared_call: Option<(TExprId, Name, crate::ast::DeclRef)>,
    /// What the last such selection took, for the reader's listing.
    pub declared_took: Option<loader::declared::Took>,
    /// The declaration of the library body's operator `type_infix` is about to type.
    pub infix_declared: Option<crate::ast::DeclRef>,
    /// `F.map[A, B](fa)(f)`: the type arguments of a direct extension call name the
    /// extension's own type parameters too.
    pub direct_ext_targs: bool,
    /// The type arguments a macro gave `Select.overloaded`, for the application of the member
    /// it names in the place of written ones.
    pub macro_targs: Option<(Name, Vec<TypeId>)>,
    /// The inferred type argument list of a library body that an application is typed without,
    /// by its start: the pickled arguments did not check.
    pub dropped_type_list: Option<u32>,
    /// The target of an assignment operator being typed (`q.y op= r`), and the receiver its
    /// selection typed, for a setter call (`assign_op_through_setter`).
    pub assign_op_target: Option<crate::ast::ExprId>,
    pub assign_op_receiver: Option<(TExprId, TypeId)>,
    /// Set while the receiver of a member selection is typed, until the application it is
    /// takes it (an inline expansion under it clears it): a bare reference to a polymorphic method leaves the type arguments its result
    /// does not fix open, and says so in `receiver_left_open`.
    pub open_receiver: bool,
    pub receiver_left_open: bool,
    /// The span of an application or selection whose member is selected: the type arguments
    /// that nothing constrains and its result holds invariantly stay open for what follows, as
    /// scalac does not interpolate them (`Response(status).withEntity(b)(using enc)` for a
    /// `Response[F[_]]`).
    pub open_applied_receiver: Option<crate::source::Span>,
    /// The applications with inferred type arguments whose pickled arguments did not check,
    /// by the type expected of them, with whether those were dropped: typed again against
    /// the same type inside an enclosing retry, each takes the same way at once.
    pub inferred_outcomes: FxMap<(FileId, crate::ast::ExprId, Option<TypeId>), bool>,
    /// The same for the initialisers of vals with inferred types, with whether the val took the
    /// initialiser's own type.
    pub inferred_val_outcomes: FxMap<(FileId, crate::ast::ExprId, Option<TypeId>), bool>,
    /// Set while the receiver of an extension method is converted, against a second round.
    pub converting_receiver: bool,
    /// The result type of a library conversion that took the receiver but has a shape teq
    /// cannot express, reported at the use when nothing else applies.
    pub blocked_conversion: Option<TypeId>,
    /// The method a `return` leaves, with its declared result type, or None where none is
    /// declared; None outside a method body.
    pub return_to: Option<(Name, Option<TypeId>, SymId)>,
    /// The `return` expressions of the body being typed, for the warning on those a lambda
    /// separates from their method.
    pub returns: Vec<(TExprId, Span)>,
    /// How many `@nowarn` definitions enclose the code being typed.
    pub nowarn: u32,
    /// The unused-import check's marks and mode (`unused.rs`).
    pub unused: unused::Unused,
    /// Set while a check runs whose findings depend on malformed syntax, the coverage of a
    /// match with a broken case: what it reports is dependent (`Diagnostic::dependent`).
    pub dependent_checks: u32,
    /// The program's trees may hold what the parser recovered from a syntax error: a one-shot
    /// build's with one, and a session's, whose texts change. Only then can a definition be
    /// incomplete, which the hot paths read it for, and is a message about a type that holds the
    /// error type marked as such (`error_unless_unknown`); set before the bodies are typed.
    pub recovered: bool,
    /// How many error nodes of the parser's recovery (`Expr::Error`, `TyExpr::Error`,
    /// `Pat::Error`) were typed: with the errors reported, what a typing failed at
    /// (`failure_mark`).
    pub error_nodes: u32,
    /// Opaque types whose definition is visible in the current environment.
    pub transparent: Vec<ClassId>,
    pub file_opaques: crate::arena::FileVec<Vec<ClassId>>,
    /// The runtime versions of a jar's inline overrides, as scalac retained them for dispatch.
    pub retained_bodies: crate::arena::Layered<SymId, crate::ast::ExprId>,
    /// The bodies of the program's inline methods typed at their definitions
    /// (`inline_definition.rs`).
    pub inline_definitions: crate::arena::Layered<SymId, std::sync::Arc<InlineDefinition>>,
    /// The records a merge replaced by another worker's of the
    /// same method (each worker that calls a method checks it), whose stored nodes' capture
    /// records are dropped uncounted with the kept ones' (`substitution::StoredKeys`).
    pub superseded_definitions: Vec<std::sync::Arc<InlineDefinition>>,
    /// In a build that captures, the capture's records the
    /// definition checks made (the stored bodies' and their typings' discarded attempts), which
    /// a compaction drops uncounted: a method is checked in each worker that calls it.
    pub check_capture_keys: Vec<crate::tir::capture::Key>,
    /// Static types of the arguments typed by the latest `type_clause_args`.
    pub last_arg_types: Vec<TypeId>,
    /// Numeric arguments typed against an open type variable: (argument, its type, parameter type).
    pub pending_widenings: Vec<(TExprId, TypeId, TypeId)>,
    /// Operands bound to temporaries so that they are evaluated once and in source order; the
    /// expression that caused them wraps the ones above its mark into a block (`Worker::hoist`).
    pub hoisted: Vec<TStmt>,
    /// Pairs whose least upper bound is being computed; a self-referential base type such as
    /// `IterableOps[A, CC, C]` with `C` the collection itself leads back to the same pair.
    pub lub_in_progress: Vec<(TypeId, TypeId)>,
    /// The expressions whose type is a union `lub` made of their branches (an `if`, a `match`,
    /// a `try`): soft, as dotc's, it widens to its join where the expression's type is inferred
    /// (a val's, a lambda's result); a union written in the program is never soft.
    pub soft_exprs: crate::intern::FxMap<TExprId, ()>,
    /// Set by `join_lower` when a variable's lower bounds joined to a union (`solve_var_inner`).
    pub joined_soft: bool,
    /// The binders of a soft scrutinee (`(if c then a else b) match { case x => .. }`): a val
    /// inferred from one widens, as dotc's `widenInferred` widens the binder's soft type.
    pub soft_syms: crate::intern::FxMap<SymId, ()>,
    /// The type of the soft scrutinee whose patterns are being typed.
    pub soft_scrutinee: Option<TypeId>,
    /// Whether a join being computed serves member selection on a union, where differing
    /// invariant arguments read as the unknown argument rather than failing the join.
    pub join_wild: bool,
    /// Set while a union is joined for member selection: unrelated covariant arguments stay a
    /// union instead of widening to their base classes.
    pub soft_lub: bool,
    /// Relating an application's result to what is expected (`constrain_result_by_expected`): an
    /// intersection of open types conforms through either part, and when both do, only what both
    /// imply is kept, as dotc's `necessaryEither` under `Mode.ConstrainResult`.
    pub necessary_either: bool,
    /// How many signature, alias or class completions are under way: a definition completes
    /// in its own scope, whatever inline expansion asked for it.
    pub completing: u32,
    /// Library classes completed inside another completion, whose output names are settled
    /// once the outermost completion is done: settling them decodes the signatures of the
    /// overloaded members, which may name an alias whose right-hand side is being resolved.
    pub pending_library_names: Vec<ClassId>,
    /// How many `name_library_members` runs are under way: none of the pending classes is
    /// named while one is, since a name being computed up the stack reads as none.
    pub naming_library: u32,
    /// The diagnostics of members of a library body's anonymous classes that did not type:
    /// reported when the reach marks the member, kept quiet where nothing calls it.
    pub poisoned: FxMap<FunId, Vec<crate::source::Diagnostic>>,
    /// Of those, the members whose bodies a product withholds: the interpreter stops where it
    /// calls one.
    pub poisoned_withheld: FxMap<FunId, ()>,
    /// The diagnostic of each body a product withholds, by the node its typing left in its
    /// place: a run that reaches the node reports it again where an attempt that typed the body
    /// gave it up (a macro's expansion tried once more).
    pub withheld_notes: FxMap<crate::tir::TExprId, crate::source::Diagnostic>,
    /// The jar classes a search for reflectively instantiatable ones entered, once a session.
    reflective_jar_found: Option<Vec<ClassId>>,
    /// Roots given a suffixed name after the final naming pass (a library body's class that
    /// inherits two of them), whose calls the reach pass recorded under the plain name.
    pub renamed_roots: Vec<SymId>,
    /// Variables carried through a for-comprehension when a guard follows value definitions.
    /// The values a `for`'s value definitions pack for the generator after a guard: the name,
    /// the type and whether it is a given.
    pub for_packs: Vec<Vec<(Name, TypeId, bool)>>,
    pub for_bases: Vec<usize>,
    pub given_indexes: FxMap<PkgId, std::sync::Arc<implicits::GivenIndex>>,
    /// Shared signatures for plain values, so that locals do not allocate one each.
    pub value_sigs: FxMap<TypeId, std::sync::Arc<MethodSig>>,
    pub temp_names: FxMap<(&'static str, u32), Name>,
    /// The package chain of each file, a memo of the worker's own.
    pub file_chains: Vec<Option<std::sync::Arc<[PkgId]>>>,
    /// Export tables, per class, filled on first use.
    pub class_exports: exports::ExportTables<ClassId>,
    pub pkg_exports: exports::ExportTables<PkgId>,
    /// The tables that are being built, innermost last.
    pub export_stack: Vec<exports::ExportFrame>,
    /// The givens a class defines, inherits and exports; `None` for a class without any.
    pub class_given_indexes: FxMap<ClassId, Option<std::sync::Arc<implicits::GivenIndex>>>,
    /// `compare_givens` per pair of member givens, which depends on their signatures and owners
    /// alone (`Worker::compare_givens` says when it is not kept).
    pub given_preferences: FxMap<(SymId, SymId), i8>,
    /// The preference order of a long list of fitting candidates, per list (`preference_order`).
    pub given_orders: FxMap<Box<[SymId]>, Box<[u32]>>,
    /// The implicit scope of a type without open variables, computed once: the objects whose
    /// members make it up, and per wanted class the givens selected from them before the
    /// accessibility filter, which depends on the scope of the search (`implicits.rs`).
    pub implicit_scopes: FxMap<TypeId, std::sync::Arc<[ClassId]>>,
    pub implicit_scope_givens: FxMap<(TypeId, u8, u32), std::sync::Arc<implicits::ScopeList>>,
    /// A package clause's level of a given search before the accessibility filter, per package,
    /// file (for the innermost clause, which holds the file's imports), and wanted class
    /// (`implicits::LevelList`): kept per worker where its inputs are settled, cleared by a
    /// retype.
    pub package_levels: FxMap<(PkgId, u32, u8, u32), std::sync::Arc<implicits::LevelList>>,
    /// The result of the last search of a target under a lexical context, with the words of
    /// that context (`context_key`), which a hit compares in full; a use copies the tree.
    /// Cleared by a retype, since a changed file's symbols move. `given_context` holds the
    /// words of the context read last.
    pub given_fast: FxMap<(TypeId, FileId, u64), (std::sync::Arc<implicits::GivenResult>, Box<[u32]>)>,
    pub given_context: Vec<u32>,
    /// What the searches under way have read since the outermost began: the lexical levels
    /// consulted, the givens opened, and what rules a result out of the memo
    /// (`implicits::GivenTree`).
    pub given_consulted: Vec<implicits::Wanted>,
    pub given_opened: Vec<SymId>,
    pub given_tree: implicits::GivenTree,
    pub trace_starts: Vec<std::time::Instant>,
    /// The shape pass of a candidate against a target without open variables, keyed by the
    /// candidate, the object it is reached through and the target (`fits_target` says when the
    /// answer is context-free).
    pub given_fits: FxMap<(SymId, u32, TypeId), bool>,
    /// A candidate's declared result as the head rejection reads it against a class, per
    /// candidate and class (`implicits::HeadSig`): kept like `given_fits`, cleared by a retype.
    pub head_sigs: FxMap<(SymId, ClassId), std::sync::Arc<implicits::HeadSig>>,
    /// A match-type alias of a generic class seen through a prefix (`K0.LiftP` for `Kind`'s
    /// `LiftP`), copied with the class's parameters as the prefix instantiates them, per
    /// alias and prefix.
    pub derived_aliases: FxMap<(AliasId, TypeId), AliasId>,
    /// An opaque type of a class or trait seen through an object deriving from it
    /// (`HtmlTagOf.Tag` of `trait TagLite`), per opaque type and object.
    pub derived_opaques: crate::arena::Layered<(ClassId, ClassId), ClassId>,
    /// The function types a conversion is being searched for as an implicit value, which a
    /// conversion's own implicit clause asks for again.
    pub function_targets: Vec<TypeId>,
    /// The extensions of an implicit scope inherited from a trait, with the object each was
    /// found in, for the search under way (`Worker::implicit_scope_extensions`).
    pub ext_modules: Vec<(SymId, ClassId)>,
    /// The value each `ValueImport` reads: its val, the object it is a member of, and the value
    /// it is selected on where it is an object nested in a class (`import o.R.S.*` reads `S` on
    /// `o.R`, an entry of its own reading `R` on `o`). One table for every worker, appended to
    /// under the loader's lock.
    pub import_values: std::sync::Arc<crate::shared::SlabVec<(SymId, Option<ClassId>, Option<ValueImport>)>>,
    /// Counts the lookups that met an export table which was still being built.
    pub export_blocks: u32,
    /// Tables left incomplete by such a lookup, to be built again on their own.
    pub export_retry: Vec<exports::ExportOwner>,
    pub export_retrying: bool,
    /// Imports are shared per (module, name); after the fork every worker registers through
    /// `js_registry`, so that the ids agree.
    pub js_import_ids: FxMap<JsImport, u32>,
    pub js_registry: Option<std::sync::Arc<JsRegistry>>,
    /// How many imports the signature phase registered: the ones the bodies register after
    /// them are numbered in a canonical order once every body is typed (`settle_js_imports`).
    pub js_imports_at_bodies: usize,
    /// Whether any worker registered a JS export: with the registry's imports, what makes
    /// the output an ES module (`is_module`).
    pub js_exported: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Which worker this is, from 0; the main worker before and after the body phase.
    pub worker: usize,
    /// The arity classes made on first use (`arity.rs`) and the by-name class, by name: a cell,
    /// so that two workers asking for `Function5` get one class.
    pub arity_classes: crate::arena::Layered<Name, ClassId>,
    /// The work items this worker typed and the own records each made (`merge_workers`).
    pub items: Vec<merge::ItemRange>,
    /// How many of this worker's own `TClass` records are published (`publish_class_bodies`).
    pub tclasses_published: usize,
    /// The worker's own `TClass`es past `tclasses_published` registered already, by their index
    /// among its own: the ones an expansion's walk was settling then were left out
    /// (`tclasses_settling`).
    pub tclasses_registered: FxMap<usize, ()>,
    /// The worker's own `TClass`es a copier made that their maker still fixes up in place (an
    /// expansion's walk settling the copies of a stored body's classes, a macro's run expanding
    /// the deferred calls its quotes' classes hold): no publication registers one before
    /// (`settling_classes`).
    pub tclasses_settling: Vec<u32>,
    /// How many of those makers are under way.
    pub tclass_settling_scopes: u32,
    /// The worker's own `TClass`es made inside the check of an own class, with the outermost
    /// such class (`note_nested_in_check`): registered once its check is done.
    pub tclasses_in_checks: Vec<(u32, ClassId)>,
    /// What the program-side work outside the loader's lock registered that another worker
    /// reads with the bodies and signatures it reaches: published with them
    /// (`publish_class_bodies`).
    pub pending_shared: Vec<PendingShared>,
    /// The entries whose inherited alternatives this worker is merging (`merge_inherited`).
    pub merging: Vec<SymId>,
    /// Whether the body phase takes the largest files first: with more than one worker.
    pub walk_by_size: bool,
    /// Whether the body phase began: the prefix typed before the fork, or the workers.
    pub bodies_started: bool,
    /// The info messages the macros' runs gave while a build or a retype is typed, held until
    /// the driver accepts the attempt (`flush_infos`); `None` outside, where a message is
    /// printed as it comes.
    pub infos: Option<Vec<InfoMessage>>,
    /// How much of `StdFiles::late_pkgs` this worker has acted on.
    pub std_seen: usize,
    /// The std slots this worker entered under the current lock hold, visible to the others
    /// when the hold ends.
    pub std_entered_pending: Vec<u16>,
    /// What the loader finished under the current lock hold, marked done for the lookups
    /// outside the lock when the hold ends (`loader_done`).
    pub loader_done_pending: Vec<LoaderDone>,
    /// How many of the shared `arity_classes` this worker's tables hold (`sync_arity_tables`).
    pub arity_seen: usize,
    /// The length of `local_news` at the fork: every worker starts with those entries, and the
    /// merge takes what each added.
    pub lists_at_fork: usize,
    /// How many type variables the signature phase made: every worker starts with them (a
    /// signature of the prefix may name one); the body phase's are each worker's own.
    pub tvars_at_fork: u32,
    pub files: &'a Sources,
    pub sites: site::DefSites,
    /// The latest application of an enum case constructor with its type before it was widened
    /// to the enum, which is the type that a member selected from it sees.
    pub enum_case_new: Option<(TExprId, TypeId)>,
    /// The last ascribed literal (`44: Int`): its type is the ascription's, so a conversion's
/// receiver of that node is no constant to adapt (`Expr::Typed`, `try_conversion`).
    pub ascribed_constant: Option<TExprId>,
    /// Parent constructor arguments that were typed early, to infer the type arguments of
    /// `case A extends E(1)`.
    pub inferred_parent_args: crate::arena::Layered<ClassId, crate::tir::ParentCall>,
    /// The same for the parameterized traits a class names without their type arguments
    /// (`class P extends Plain(3)`), each trait's call, which `fill_trait_params` passes.
    pub inferred_trait_args: crate::arena::Layered<ClassId, std::sync::Arc<[(ClassId, crate::tir::ParentCall)]>>,
    /// The enum case whose parent constructor arguments are being typed. They belong to the
    /// scope of the companion, so the members of the case are not visible by name.
    pub parent_args_of: Option<ClassId>,
    /// The class whose deferred givens' implementations are being searched, under
    /// `parent_args_of`: its own using parameters are the givens of its frame
    /// (`Worker::implement_deferred_givens`).
    pub deferred_impl_of: Option<ClassId>,
    /// The classes SAM lambdas implement. Their frame is on the stack while the lambda body
    /// is typed, and a lambda body does not see the method it becomes: `enc` in
    /// `Codec(b => enc.enc(b))` is the outer field.
    pub sam_classes: FxMap<ClassId, ()>,
    /// The accessor of each member an inline body reads or assigns through one, by the member
    /// and whether it is the setter, in a build that writes products (`add_inline_accessors`).
    pub inline_accessor_syms: FxMap<(ClassId, SymId, bool), SymId>,
    /// The result type of each inline method of a products build declared without one
    /// (`infer_inline_results`), which its pickle and the signatures naming it write.
    pub inline_results: FxMap<SymId, TypeId>,
    /// Marks on typed expressions: `UNCHECKED` for `(e: @unchecked)`, whose `isInstanceOf`
    /// warns of nothing, `CAST` for an `asInstanceOf` erased to the expression it casts, which
    /// is no path.
    pub expr_marks: FxMap<TExprId, u8>,
    /// The lambdas an eta-expansion made, which still convert to a SAM trait where one is expected.
    pub eta_expansions: FxMap<TExprId, ()>,
    /// A constant's selection written as its literal (`C.f` of `final val f = true`), with the
    /// receiver and the member it names: a path still where a singleton type is expected of it
    /// (`def cond(): C.f.type = C.f`), as scalac folds only after typing.
    pub folded_paths: FxMap<TExprId, (TExprId, SymId)>,
    /// Set while the qualifier of an explicit `.apply` is typed: `Box[Int].apply` names the
    /// `apply` of `Box`, which no summoner sugar replaces.
    pub apply_selected: bool,
    /// What the patterns of the enclosing match cases tell about type parameters in scope:
    /// (parameter, type, variance of the position that related them). Variance 0 is equality,
    /// 1 makes the type a lower bound and -1 an upper bound.
    pub gadt: Vec<(TParamId, TypeId, i8)>,
    /// Whether a wildcard type argument occurs anywhere, so that capture conversion has
    /// something to do: read off the syntax before anything is typed (`scan_wildcards`), so
    /// that a body's type does not depend on what was typed before it.
    pub wildcards_used: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The captured type of a local with wildcard arguments: one capture per local, so that
    /// `b.put(b.get)` and `b.twice(b)` agree on what `b: Box[?]` holds, as under scalac.
    pub captured_locals: crate::intern::FxMap<SymId, TypeId>,
    /// The language server's navigation index (`index.rs`), kept by a `--index` session only.
    pub index: Option<Box<index::Index>>,
    /// What the typer resolves that its types do not keep, for the dependencies of an analysis
    /// build (`deps.rs`), kept under `--analysis-version 3` only.
    pub deps: Option<Box<deps::Recorder>>,
    /// Set while the pattern of a match case is typed, the only place that adds to `gadt`.
    pub gadt_open: bool,
    /// Set while the pattern of a case after an unguarded catch-all of its match is typed: a case
    /// the pattern matcher drops, whose type tests no erasure check sees (`check_sensical_test`).
    pub dead_case: bool,
    /// Set for the match that makes up a partial function literal: cases may be missing.
    pub partial_match: bool,
    /// The working memory of the match checks, kept between matches so that a match allocates
    /// nothing.
    pub spaces: space::SpaceScratch,
    /// The checks of the matches the bodies typed, run once every body is (`run_deferred_matches`).
    pub deferred_matches: Vec<space::DeferredMatch>,
    /// The methods a macro's run typed for a class whose `TClass` other workers read already,
    /// registered with it after the merge (`Worker::program_body`).
    pub late_methods: Vec<(ClassId, FunId)>,
    /// Each method an override check found overriding an inherited one, with that one: what
    /// `decide_override_pairs` decides again once every body is typed.
    pub override_pairs: Vec<(SymId, SymId)>,
    pub derive: derive::Derivation,
    /// The environment of the `new` expression of each anonymous class, which its members are
    /// typed in.
    pub anon_envs: FxMap<ClassId, std::sync::Arc<Env>>,
    /// Extractor patterns whose `unapply` returns a `Some`, which cannot fail.
    pub irrefutable_pats: FxMap<TPatId, ()>,
    /// The type patterns over an abstract type that go through a `TypeTest` or a `ClassTag`
    /// found at the pattern (`tag_pattern`), with the type `x @ (t: T)` binds.
    pub tag_pats: FxMap<TPatId, TypeId>,
    /// The typed arguments of the calls under way with their types, for a result type that
    /// depends on a parameter (`dest.type`); cleared with the outermost call.
    pub arg_types_seen: Vec<(crate::tir::TExprId, TypeId)>,
    /// The depths (`app_depth`) of the applications under way that were given an argument whose
    /// type holds the error type, each application's truncated as it ends: an erased tag's search
    /// reads its own application's (`erased_evidence`), as scalac's application of such an
    /// argument has the error type and searches no implicit of it.
    pub erroneous_args: Vec<u32>,
    /// The lambdas typed under a result type the expected type gave whose body's type holds the
    /// error type, which their own type does not show: one that is an application's argument
    /// counts as an erroneous argument of it (`typed_arg`).
    pub erroneous_lambdas: Vec<crate::tir::TExprId>,
    /// `new A(...)` of a named local class typed before the class was, whose captures are added
    /// to the arguments once they are known.
    pub local_news: Vec<(ClassId, crate::tir::TExprId, bool, Vec<(SymId, crate::tir::TExprId)>)>,
    /// What each anonymous class receives through its constructor.
    pub anon_captures: FxMap<ClassId, Vec<SymId>>,
    /// What the expression that creates an anonymous class passes on to the superclass.
    pub anon_parent_args: FxMap<ClassId, crate::tir::ParentCall>,
    /// The members each trait calls through `super`, which the classes mixing it in bind.
    pub mixin_supers: FxMap<ClassId, Vec<SymId>>,
    /// The same of the traits of the shared region, a library's or the std's, whose bodies are
    /// typed under the loader's lock by whichever worker needs one first: every worker's, read
    /// and written by the lock's holder (`note_mixin_super`, `bind_mixin_supers`), joined to
    /// `mixin_supers` at the merge.
    pub shared_mixin_supers: std::sync::Arc<std::sync::Mutex<FxMap<ClassId, Vec<SymId>>>>,
    /// The (class, member) pairs whose super call was reported unbindable.
    pub super_problems_told: FxMap<(ClassId, SymId), ()>,
    /// What the last binding of the mixins' super calls saw: how many of the program's classes
    /// it bound, and each trait's calls (`bind_mixin_supers`).
    pub mixins_bound: (usize, FxMap<ClassId, Vec<SymId>>),
    /// The definitions whose type is left to inference and whose inference reported an error,
    /// its own or that of a body it asked for on the way: the type it gave may be nonsense, so
    /// a retype puts the old signature of such a one back (`incremental.rs`).
    pub inference_failed: FxMap<SymId, ()>,
    /// The members of `Any` that a trait calls through `super`, by trait and name.
    pub any_members: crate::arena::Layered<(ClassId, Name), SymId>,
    /// How many anonymous classes were made at each (file, offset) under each outermost
    /// expansion site, which names them apart.
    pub anon_sites: FxMap<(FileId, u32, Option<(FileId, u32)>), u32>,
    /// The selection or operator of a library body's pseudo file being typed, which a
    /// diagnostic there is placed at.
    pub body_node: Option<BodyNode>,
    /// Locals standing for the `this` of a class inside the anonymous classes nested in it.
    pub outer_this: crate::arena::Layered<ClassId, SymId>,
    /// The member of a class nested in a class that returns its enclosing instance.
    pub outer_accessors: crate::arena::Layered<ClassId, SymId>,
    /// A `new p.C(..)` is typed, whose prefix `p` gives `C` its enclosing instance.
    pub new_prefixed: bool,
    /// A `new C(..)` is typed and its constructor not resolved yet: no application of the
    /// class's name, which the capture tells apart (`Form::CaseApply`).
    pub explicit_new: bool,
    /// The constructor parameters that the reflection API tells apart from the fields of the
    /// same name, by the field they copy.
    pub reflect_ctor_params: FxMap<SymId, SymId>,
    pub reflect_param_of: FxMap<SymId, SymId>,
    /// The name being resolved is the head of a call, where a lone package extension is a term.
    pub extension_call_head: bool,
    /// `@main` methods and the `main(args: Array[String])` methods of top-level objects, each
    /// with the object it runs on when the object inherits it.
    pub entry_points: Vec<(SymId, Option<ClassId>)>,
    /// Bounds of type applications, checked after every class is complete.
    pub deferred_bounds: Vec<resolve::DeferredBound>,
    /// What each `T @uncheckedVariance` resolved to, by its type expression; the variance check
    /// passes over those of the member it checks.
    pub unchecked_variance: crate::arena::Layered<(FileId, crate::ast::TyExprId), TypeId>,
    /// The secondary constructor whose self call is being typed, which is no alternative of it.
    pub excluded_ctor: Option<SymId>,
    pub variance_skip: Vec<TypeId>,
    /// The entry point named on the command line, by its object or its method.
    pub main_name: Option<Name>,
    /// Whether an entry point is chosen at all: a resident check has no program to run, so
    /// several `@main` methods are no error there.
    pub choose_entry: bool,
    /// The module-product mode (`--products`): every definition of the program's files is a
    /// root of the reach, as in link mode, and the classes of the class path's product
    /// directories are called, never compiled.
    pub open_world: bool,
    /// The vals, vars and class parameters of the sources that `@scala.volatile` marks, resolved
    /// with the `@targetName`s before a JVM build's reach (`Worker::source_target_names`).
    pub jvm_volatile: FxMap<SymId, ()>,
    /// Whether the build writes a module's products (`--products`, either target): what it
    /// reads of upstream products keeps their link path's model, its pickles naming what
    /// scalac's do (a product's top-level definition through its file's object).
    pub writes_products: bool,
    /// How many bodies withheld from their products the typing met (`Expr::Withheld`), each an
    /// error: what stops the interpreter before it runs on past one.
    pub withheld_met: u32,
    /// The dialect flags of the build (`src/dialect.rs`), `strict_equality` among them.
    pub dialect: crate::dialect::Dialect,
    /// The JVM target: intrinsics come from `@jvm`, and expressions keep their static types.
    pub jvm: bool,
    /// `scala.scalajs.js.internal.UnitOps` once looked up, which is in the implicit scope of `Unit`
    /// on JavaScript.
    pub unit_ops: Option<Option<ClassId>>,
    /// Whether the program runs under the interpreter, which tells a `Char` from a `String`.
    pub interp: bool,
    /// Set while a body is typed for the interpreter on the JVM (`deferred_body`): a def with a
    /// `@jvm` template gets its Scala body there, as under `teq interp` and as a def with
    /// a body and no `@js` template gets it on JavaScript.
    pub for_interpreter: bool,
    /// Set while the signature of a `@jvmEvidence` definition is resolved where its evidence
    /// is erased: the type parameters whose `ClassTag` bound was left out, by index.
    pub erased_tags: Option<u32>,
    /// The type arguments of a polymorphic function value being applied, which the call of the
    /// function type it instantiates to takes (`poly_pending`).
    pub poly_targs: Option<Vec<TypeId>>,
    /// The calls of polymorphic function values with their type arguments, which the
    /// application solves once its last clause is typed and records for the pickle.
    pub poly_pending: Vec<(TExprId, Vec<TypeId>)>,
    /// Entries of standard library classes that meet an overloaded inherited name and that no
    /// lookup may ever settle.
    pub unsettled_std_entries: Vec<(ClassId, SymId)>,
    /// Methods that override one whose name in the output may not be the plain one; they are
    /// named with the alternatives, once every class is checked.
    pub dispatch_pending: Vec<SymId>,
    /// Methods that take the name of the one method they implement, as `dispatch_pending`.
    pub named_like: Vec<(SymId, SymId)>,
    /// Whether a val of the program was given an accessor (`needs_accessor`) by
    /// `mark_accessors` or `reach_through_accessor`: the final passes then make its kin agree
    /// (`settle_val_accessors`), and skip them otherwise.
    pub vals_by_accessor: bool,
    /// Methods that override nothing and get a suffixed name although their class does not
    /// overload them (`Worker::suffix_clashing_roots`).
    pub suffixed_roots: FxMap<SymId, ()>,
    pub prog_index: check::ProgIndex,
    pub roots_memo: FxMap<SymId, std::sync::Arc<[SymId]>>,
    pub subclasses: overload::SubclassIndex,
    /// Set once the alternatives of every overloaded name have their names in the output;
    /// an entry that is settled later names its alternatives right away.
    pub alternatives_named: bool,
    /// Pairs of concrete members that a class inherits under one name from unrelated ancestors,
    /// which conflict unless their parameters differ: (class, name, first, second, marked).
    pub inherited_pairs: Vec<(ClassId, Name, ClassId, ClassId, bool)>,
    /// The jars of `--classpath` and what has been read from them, shared by every worker;
    /// changed through `loaded_mut`, under the loader's lock.
    pub loaded: Option<std::sync::Arc<loader::LoadedCell>>,
    /// The std files of the build and which of them entered (`stdlib.rs`), shared by every
    /// worker; changed through `std_mut`, before the fork or under the loader's lock.
    pub std: std::sync::Arc<stdlib::StdCell>,
    /// Set once `enter_all` is done: a std file entered later completes and checks its own
    /// definitions, since the eager passes are past.
    pub entering_done: bool,
    /// Where the body phase stands: whether its walk over the files began, which files it has
    /// taken (a std file entered behind the walk is checked at entry, one ahead of it by the
    /// walk), and whether it is over.
    pub walk: Walk,
    /// The std object the namer just skipped for a program definition of its name.
    pub shadowed_object: Option<ClassId>,
    /// A bare identifier is being looked up through a class frame: a miss among the members
    /// is no reason to read the JDK's, since the name goes on to the imports and packages.
    pub bare_lookup: bool,
    /// The type parameters whose bounds the member lookup is inside: a cyclic bound
    /// (`A <: S[A]` with `type S[X] = X`, scalac's illegal cyclic reference) finds nothing
    /// instead of recursing without end.
    bound_path: Vec<TParamId>,
    /// Whether `ArraySeq` was asked for with `Seq`.
    array_seq_asked: bool,
    /// The files of sealed std classes completed inside another completion, whose classes
    /// complete once the outermost is done.
    pub sealed_std_files: Vec<FileId>,
    /// The first overload set of the std files entered after the final passes, whose sets are
    /// named once no completion is under way.
    pub late_naming_from: Option<usize>,
    /// How deep the lookup of type members through bounds is nested, and how many steps the
    /// outermost lookup may still take (`members.rs`).
    pub dep_depth: u32,
    /// How many bodies are being typed inside one another, which a definition whose type
    /// needs its own body would grow without bound.
    pub body_depth: u32,
    /// The nesting of the type printer, which cuts a type short past a depth.
    pub show_depth: std::cell::Cell<u32>,
    pub dep_budget: u32,
    /// The member the outermost lookup under way asked for, which a cycle is reported at.
    pub dep_root: (TypeId, Name),
    /// Whether the outermost lookup under way met an alias that comes back to itself; its
    /// result is then an error.
    pub dep_exhausted: bool,
    /// How deep the check of a type against `Singleton` follows bounds.
    pub singleton_depth: u32,
    /// Set while a type written in the source is resolved: whether a lookup through a term
    /// path met such an alias there, which is reported at that use.
    pub cycle_at_use: Option<bool>,
    /// The lookups under way, outermost first, each flagged once it is known to resolve an
    /// abstract member's bounds: such a member named in its own bounds (`type T <: Element[T]`)
    /// is kept by name where a rebasing meets it again, while an alias that expands to itself
    /// is a cycle.
    pub lookups: Vec<(TypeId, Name, bool)>,
    /// The parameters of the applications under way whose arguments were typed, with the path
    /// each argument gives the parameter, for the clauses and the result that name it.
    pub param_paths: Vec<(SymId, TypeId)>,
    /// The members over a candidate's own parameters replaced by variables while its result
    /// is matched against a search's target, with the variable standing for each.
    pub path_approx: Vec<(TypeId, TypeId)>,
    /// The signatures of term refinements whose types are not their symbols' own, by the
    /// symbol and the types (`refinement_sig`), the type parameters rebound once.
    pub refinement_sigs: FxMap<(SymId, TList), std::sync::Arc<MethodSig>>,
    /// The parameters and function types of polymorphic function types whose bounds are not
    /// their parameters' own (`poly_binders`), rebound once.
    pub poly_rebound: FxMap<TypeId, (Vec<TParamId>, TypeId)>,
    /// The function type a polymorphic function literal being typed is expected to have, its
    /// `apply`, which `check_curried_dependent` leaves alone.
    pub poly_literal_fun: Option<TypeId>,
    /// For a member selected on an inner-class value made by a call on the outer instance, the
    /// enclosing class and the prefix its `this` is seen from, by the receiver expression.
    pub outer_prefixes: FxMap<TExprId, Vec<(ClassId, TypeId)>>,
    /// Whether a class is a SAM type of a given arity, as the overload shape pass asks.
    pub sam_arity: FxMap<(ClassId, u8), bool>,
    /// The type members whose lookup ran into a cycle, reported once each.
    pub member_cycles: Vec<(ClassId, Name)>,
    /// The overload sets made of a member of a class and one of its self type, by their symbols.
    pub merged_overloads: crate::arena::Layered<(SymId, SymId), SymId>,
    /// Whether the worker's work reads the shared records and the signatures in its view (the
    /// overlays on, `work`).
    pub views_on: bool,
    /// Set while the arguments of a dependent method are typed: each argument's path is noted
    /// for the parameters and the result that name its parameter.
    pub note_paths: bool,
    /// The inline expansions under way and what they bind.
    pub inline: inline::InlineState,
    /// Where the time of the type phase goes, under `--profile`.
    pub profile: profile::Profile,
    /// The match types under reduction and what they reduced to (`matchtypes.rs`).
    pub matches: matchtypes::MatchState,
    /// The memos whose entries hold types in the view they were made in, of the view the worker
    /// is not in (`swap_view_memos`).
    pub other_view_memos: ViewMemos,
    /// The type variables of the match type case being resolved, by name.
    pub case_binders: Vec<(Name, TypeId)>,
    /// The type the last typed pattern (`x: T`) gave its binder, which the enclosing `Bind`
    /// reads: the ascription with its wildcard arguments inferred from the scrutinee.
    pub last_typed_pattern: Option<(crate::ast::PatId, TypeId)>,
    /// A generator's pattern written without `case` is being typed: a type test of it that
    /// cannot be checked is a refutable part, which the irrefutability check reports as an error,
    /// before which scalac's pattern matcher, that warns of such a test, never runs.
    pub refutable_is_error: bool,
    /// The extension method the last extension application applied (`apply_extension`), which a
    /// right-associative selection reads to know which overload it took.
    pub applied_extension: Option<SymId>,
    /// The quotes being typed and the macro expansions under way (`quoted.rs`).
    pub quote: quoted::QuoteState,
    /// The files whose definitions a macro ran: an edit to one of them changes what the
    /// expansions elsewhere produced, so watch mode builds afresh.
    pub macro_files: FxMap<FileId, ()>,
    /// The objects declared to hold cacheable state, by their qualified names
    /// (`--cacheable-state`), and the classes they resolved to after the signature phase
    /// (`resolve_cacheable_state`): a watch session's interpreter keeps them and what they reach
    /// across its retypes.
    pub cacheable_state: Vec<String>,
    pub cacheable_state_classes: Vec<ClassId>,
    /// Whether the libraries' objects teq knows to hold nothing but a cache count as declared
    /// for the watch over the state macro runs share (`KNOWN_CACHES`); `--no-known-caches`
    /// turns them off, for testing a suspicion.
    pub known_caches: bool,
    /// `--macro-state per-worker`: a change of state other runs share stays on its worker's heap
    /// and gives no build away (`shared_state_changed`).
    pub macro_state_per_worker: bool,
    /// Per file holding an inline body, the other files that expanded it: an edit of the body
    /// changes what those files' bodies became, so watch mode types them again with it.
    pub inline_deps: FxMap<FileId, FxMap<FileId, ()>>,
    /// The loader's lock: held across the loader's work and
    /// every change of a shared record; `with_loader` takes it.
    pub lock: std::sync::Arc<crate::shared::ReentrantLock>,
    /// Whether the arenas have a shared region (the body phase of the parallel typer): the
    /// worker's own records carry `arena::LOCAL_BASE`, and a shared one changes under the lock.
    pub forked: bool,
    /// Set while this worker maps a library signature (the loader's lock holder alone maps one):
    /// a class it names becomes a placeholder rather than a reason to open the JDK, and the std's
    /// files see the classpath (`sees_classpath`).
    pub mapping_signature: bool,
    /// The merge's check found a worker's id or an overlay's entry left in the merged program
    /// (`Worker::check_merge`): the build ends with its error before anything reads the program.
    pub merge_failed: bool,
    /// How many types the store held at the fork: the ones made since may mention a worker's
    /// ids, which the merge makes again (`merge.rs`).
    pub types_at_fork: u32,
    /// The templates the conversion of a jar class's root decodes, with the root, decoded before
    /// the loader's lock was taken for it (`predecode_conversion`), for that conversion to enter
    /// under the lock.
    pub predecoded: Option<(ClassId, Vec<((u32, crate::tasty::tree::Addr), std::sync::Arc<crate::tasty::terms::ClassDef>)>)>,
    /// Set by a worker that met an outcome the order of the workers decides (`need_serial`):
    /// the build is typed again by one worker.
    pub serial: std::sync::Arc<SerialNeed>,
}

/// A registration made outside the loader's lock that another worker reads with the body or
/// signature it was made for (`Worker::pending_shared`).
pub enum PendingShared {
    /// A type written `@uncheckedVariance`, which the variance check of its member reads.
    Variance((FileId, crate::ast::TyExprId), TypeId),
    /// The def behind a `@js` template call, which an interpreter running the call reads.
    Template(crate::tir::StrRef, SymId),
}

/// Why the parallel body phase gave way to one worker's (`Worker::need_serial`): the first
/// reason a worker gave, for `TEQ_SERIAL_TRACE`, with the undeclared module whose state the
/// reason's macro run changed, by name and class.
#[derive(Default)]
pub struct SerialNeed {
    needed: std::sync::atomic::AtomicBool,
    reason: std::sync::Mutex<Option<(String, Option<(String, ClassId)>)>>,
    /// Whether the reason is two diagnostics at one place (`Worker::diagnostics_meeting`): this
    /// build's own errors, which say nothing of the next build's.
    diagnostics: std::sync::atomic::AtomicBool,
}

impl SerialNeed {
    pub fn is_needed(&self) -> bool {
        self.needed.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Whether the build gave way for two diagnostics at one place: nothing to say of it, and a
    /// session's later full builds keep their workers.
    pub fn of_diagnostics(&self) -> bool {
        self.diagnostics.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn reason(&self) -> Option<String> {
        self.reason.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(why, _)| why.clone())
    }

    pub fn undeclared_module(&self) -> Option<(String, ClassId)> {
        self.reason.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|(_, module)| module.clone())
    }
}

/// The most workers a body phase runs: what the ids tell apart (`arena::MAX_WORKERS`) and
/// what the cells' wait slots hold (`shared::MAX_THREADS`, with room for the other threads).
pub const MAX_WORKERS: usize = 128;

/// The JS imports registered during the body phase, by every worker: the ids agree across
/// them, and the merge renumbers them by their first definition in source order.
pub struct JsRegistry {
    pub ids: crate::arena::SharedMap<JsImport, u32>,
    pub list: crate::shared::SlabVec<JsImport>,
    serial: std::sync::Mutex<()>,
}

impl JsRegistry {
    fn from_vec(v: &[JsImport]) -> JsRegistry {
        let r = JsRegistry { ids: crate::arena::SharedMap::new(), list: crate::shared::SlabVec::with_capacity(v.len().max(16)), serial: std::sync::Mutex::new(()) };
        for &i in v {
            let n = r.list.push(i) as u32;
            r.ids.insert(i, n);
        }
        r
    }

    pub fn register(&self, import: JsImport) -> u32 {
        if let Some(&id) = self.ids.get(&import) {
            return id;
        }
        let _g = self.serial.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&id) = self.ids.get(&import) {
            return id;
        }
        let id = self.list.push(import) as u32;
        self.ids.insert(import, id);
        id
    }
}

/// The libraries' objects that hold nothing but a cache, with the reason each is one: a macro's
/// run that changes their state computes what it would compute without them, so the change is
/// no state another run reads differently. They
/// count as declared with `--cacheable-state` for the watch over the state the
/// runs share, not for what a session keeps between its builds; `--no-known-caches` turns them
/// off (docs/TARGETS.md). Unlike a declared object, an entry vouches for its whole reachable
/// state, what a run puts in its containers included (`interp::Cache::Known`), so an entry is
/// one whose containers hand the program nothing it can change as state: they stay private to
/// the library (a pool's buffers), or what they hand out changes only in memos of its own value
/// (a `BigInt`'s `BigInteger`).
pub const KNOWN_CACHES: &[(&str, &str)] = &[
    // `filePrefixCache` and `linePrefixCache`: memo maps keyed by the source file, each value
    // a search of that file's text for a marker line.
    ("sourcecode.Macros", "memo maps keyed by the source file, their values a function of its text"),
    // A pool of the byte buffers its pickler writes type tags into, each reused whole, and
    // counters of the pool's hits and misses that nothing reads back.
    ("izumi.reflect.thirdparty.internal.boopickle.BufferPool", "a pool of byte buffers reused whole"),
    // scala-library's `getCached`: a slot of `cache` filled once with the `BigInt` its index
    // stands for, -1024 to 1024.
    ("scala.math.BigInt", "a memo of the BigInts from -1024 to 1024"),
    // scala-library's `apply(i: Int, mc: MathContext)`: a slot of the lazy `cache` filled once
    // with the `BigDecimal` its index stands for in the default context, -512 to 512.
    ("scala.math.BigDecimal", "a memo of the BigDecimals from -512 to 512 in the default context"),
];

/// The typer as a build drives it: the signature phase, the body phase over one worker (the
/// parallel typer's several), the merge of what the bodies made and
/// the final passes, all on the typing thread of a session (`thread.rs`) with the compiler
/// thread blocked meanwhile, and on the compiler thread of a one-shot build. The worker holds
/// every table and every rule of the language; the driver sequences the phases and knows where
/// the signature phase's prefix of the arenas ends, which is what the merge renumbers from.
/// What a build reads of the typer afterwards it reads through the worker, and what types on
/// demand afterwards (the reach pass) runs where the phases run, through `on_thread`.
pub struct Typer<'a> {
    pub w: Worker<'a>,
    /// The workers of the body phase (`--threads`, `frontend::threads`).
    pub threads: usize,
}

/// Where the body phase's walk over the files stands.
#[derive(Default, Clone)]
pub struct Walk {
    pub started: bool,
    pub done: bool,
    /// Per file, whether a worker took it already.
    pub taken: std::sync::Arc<Vec<std::sync::atomic::AtomicBool>>,
    /// Per file, the thread that took it (`shared::thread_number`), 0 for none yet: a
    /// signature claimed for a body of a file another thread took is a steal.
    pub owner: std::sync::Arc<Vec<std::sync::atomic::AtomicUsize>>,
}

impl Walk {
    /// Whether a std file entered now is behind the walk, which then checks it at entry.
    pub fn passed(&self, f: FileId) -> bool {
        self.done || self.taken.get(f.0 as usize).map_or(false, |t| t.load(std::sync::atomic::Ordering::Acquire))
    }

    /// A file appended since the walk began (a library body's) has no slot and is never taken.
    pub fn take(&self, f: FileId) {
        let (Some(taken), Some(owner)) = (self.taken.get(f.0 as usize), self.owner.get(f.0 as usize)) else { return };
        taken.store(true, std::sync::atomic::Ordering::Release);
        owner.store(crate::shared::thread_number(), std::sync::atomic::Ordering::Release);
    }

    /// Whether another thread took `f` from the queue.
    pub fn taken_elsewhere(&self, f: FileId) -> bool {
        let o = self.owner.get(f.0 as usize).map_or(0, |o| o.load(std::sync::atomic::Ordering::Acquire));
        o != 0 && o != Self::PREFIX && o != crate::shared::thread_number()
    }

    /// The owner of a file typed before the fork: no worker's. What is typed of it later (a
    /// body behind an inferred result, a std file's class) goes to the shared region.
    const PREFIX: usize = usize::MAX;

    /// Takes `f` for the prefix (`check_before_fork`).
    pub fn take_for_prefix(&self, f: FileId) {
        self.taken[f.0 as usize].store(true, std::sync::atomic::Ordering::Release);
        self.owner[f.0 as usize].store(Self::PREFIX, std::sync::atomic::Ordering::Release);
    }

    pub fn reset(&mut self, files: usize) {
        self.taken = std::sync::Arc::new((0..files).map(|_| std::sync::atomic::AtomicBool::new(false)).collect());
        self.owner = std::sync::Arc::new((0..files).map(|_| std::sync::atomic::AtomicUsize::new(0)).collect());
        self.started = true;
        self.done = false;
    }

    /// Nothing to do: the table is shared already.
    fn share(&mut self) {}
}

impl<'a> std::ops::Deref for Typer<'a> {
    type Target = Worker<'a>;
    #[inline]
    fn deref(&self) -> &Worker<'a> {
        &self.w
    }
}

impl<'a> std::ops::DerefMut for Typer<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Worker<'a> {
        &mut self.w
    }
}

/// An info message of a macro's run (`report.info`), kept with where its expansion stands so
/// that a build prints its messages in one order whatever its workers' schedule: the positions
/// of the inline calls under expansion, the outermost first, then the message's number within
/// its run.
/// What the loader finished that a lookup outside its lock may rely on: a class file's
/// top-level definitions entered into its package, a package's objects
/// entered (by the package's slot), a Java class's members absorbed from the JDK or known absent
/// for good, a std class's JDK constructors looked at, a builtin's Java members entered (0 for
/// `String`, 1 for `AnyRef`).
#[derive(Clone, Copy)]
pub enum LoaderDone {
    File(crate::classpath::CpFile),
    Objects(usize),
    JavaClass(ClassId),
    JavaCtors(ClassId),
    JavaBuiltin(usize),
}

pub struct InfoMessage {
    pub path: Vec<(FileId, u32, u32)>,
    pub ordinal: u32,
    pub text: String,
}

/// Whether the build ran with the type store's overlays (`TEQ_TYPE_OVERLAYS`) and forked.
/// Whether `TEQ_INLINE_COUNTS` asks for the counts of the expansion by substitution.
pub fn substitution_counts_on() -> bool {
    substitution::counts::on()
}

pub fn overlays_measured() -> bool {
    crate::types::overlays_wanted()
}

impl<'a> Typer<'a> {
    pub fn new(asts: &'a Asts, files: &'a Sources, interner: &'a Interner) -> Typer<'a> {
        Typer { w: Worker::new(asts, files, interner), threads: 1 }
    }

    pub fn run(&mut self) {
        let threads = self.threads;
        self.on_thread(move |w| {
            // What the interpreter cached for an earlier program on this thread (a watch
            // session's last build) indexes that program's IR, and what its runs left in
            // cycles is freed with it.
            let in_use = || {
                crate::alloc::settle();
                let held = crate::alloc::held();
                held.reserved.saturating_sub(held.centre_free) + held.large
            };
            let before = crate::measure::inventory_on().then(in_use);
            crate::interp::dispose();
            crate::interp::forget_files();
            if let Some(before) = before {
                crate::measure::inventory_line("disposed at the build's start", &[("bytes", before.saturating_sub(in_use()))]);
            }
            // Several workers' macro runs, and every run for the census, are watched for the
            // state they share (`interp::watch`), the ones before the fork included: a change
            // one of them makes is in the first worker's heap alone.
            let threads = threads.clamp(1, MAX_WORKERS);
            // The overlays' mappings are taken before anything follows the worker count: where
            // the system gives none, one worker types the build in place. The store shared by the
            // workers (`TEQ_TYPE_OVERLAYS=off`) is the overlays' control, correct on the suites;
            // on a large build a worker's own records reach the base through it (a production
            // application's API: the arena's `no record` panic), so a refusal does not lead there.
            let fork = threads > 1 || std::env::var_os("TEQ_FORK").is_some_and(|v| v == "1");
            let mappings = (fork && crate::types::overlays_wanted()).then(|| TypeStore::overlay_mappings(threads));
            let refused = matches!(mappings, Some(None));
            if refused {
                eprintln!("teq: {}", crate::types::MAPPING_REFUSED);
                crate::types::note_mapping_refused();
            }
            let (threads, fork, mappings) = if refused { (1, false, None) } else { (threads, fork, mappings.flatten()) };
            crate::types::view::typing();
            w.walk_by_size = threads > 1;
            let census = std::env::var_os("TEQ_MACRO_CENSUS").is_some();
            crate::interp::watch(threads > 1 || census);
            w.infos = Some(Vec::new());
            crate::measure::phases_begin();
            w.signature_phase();
            w.resolve_cacheable_state();
            crate::measure::phase_done("signatures");
            w.js_imports_at_bodies = w.prog.js_imports.len();
            let mut prefix = merge::Prefix::of(w);
            // From here on the bodies are typed, which a retype does again; what is resolved
            // once on the way (a signature, an import) says so where it is (`once`).
            w.diags.of_bodies = true;
            w.bodies_started = true;
            prep::capture(true);
            if std::env::var_os("TEQ_PREP_REPLAY").is_some() {
                w.replay_preparation();
                crate::measure::phase_done("the std and the libraries prepared (TEQ_PREP_REPLAY)");
            }
            if std::env::var_os("TEQ_PREP_PREDICT").is_some() {
                w.predict_preparation();
                crate::measure::phase_done("the std and the libraries predicted (TEQ_PREP_PREDICT)");
            }
            // The body phase over the workers, with the merge after
            // it; `TEQ_FORK=1` takes that path with one worker, for its tests. With one worker and
            // no fork the bodies are typed in place and nothing is renumbered.
            if fork {
                let order = w.file_order();
                w.walk.reset(w.asts.len());
                w.check_before_fork(&order);
                crate::measure::phase_done("prefix, the macro reach");
                if w.serial.is_needed() {
                    crate::interp::watch(false);
                    return;
                }
                prefix = merge::Prefix::of(w);
                let clock = std::time::Instant::now();
                w.fork(threads > 1);
                crate::measure::fork_part("the arenas forked", clock.elapsed());
                let clock = std::time::Instant::now();
                let overlays = w.types.fork_for(threads, mappings);
                crate::types::view::forked(overlays);
                w.profile.sig_hook = w.profile.on || (overlays && (cfg!(debug_assertions) || crate::types::noting()));
                crate::measure::fork_part("the overlays mapped", clock.elapsed());
                let clock = std::time::Instant::now();
                let queue = check::Queue::new(w, &order);
                crate::measure::fork_part("the queue", clock.elapsed());
                if let Some(path) = std::env::var_os("TEQ_QUEUE_SHAPE") {
                    queue.write_shape(w, &path);
                }
                if crate::measure::on() && std::env::var_os("TEQ_FORK_INVENTORY").is_some_and(|v| v == "1") {
                    w.fork_inventory();
                }
                // Before the workers attach, each takes its first file, this one first, as the race
                // at the start mostly left it. Of the workers without a file, as many start as there
                // are items left to steal, which they take as they run. A worker beyond them is
                // neither attached nor started, its slot empty through the merge, which keeps the
                // slots' numbers (`TEQ_IDLE_WORKERS` starts every one).
                let clock = std::time::Instant::now();
                let first = queue.reserve_file(0);
                let files: Vec<Option<check::Reserved>> = (1..threads).map(|k| queue.reserve_file(k)).collect();
                let mut stealable = queue.items_left().saturating_sub(first.is_none() as usize);
                let mut started = |file: &Option<check::Reserved>| {
                    let steals = file.is_none() && stealable > 0;
                    stealable -= steals as usize;
                    file.is_some() || steals || check::idle_workers_asked()
                };
                let others: Vec<Option<(Worker<'a>, Option<check::Reserved>)>> = files.into_iter().enumerate().map(|(i, file)| started(&file).then(|| (w.attach(i + 1), file))).collect();
                crate::measure::empty_slots(others.iter().filter(|o| o.is_none()).count());
                for (i, _) in others.iter().enumerate().filter(|(_, o)| o.is_none()) {
                    crate::types::view::slot_left_empty(&w.types, i + 1);
                }
                crate::measure::fork_part("the attachments", clock.elapsed());
                crate::measure::phase_done("fork");
                let inventory = crate::measure::inventory_on();
                let spawning = inventory.then(std::time::Instant::now);
                let since = move || spawning.map_or(0, |at| at.elapsed().as_micros() as usize);
                // A worker hands itself to the merge as soon as its work is done; what its
                // thread does after that (a session's interpreter caches disposed, its lists
                // handed back as it ends) runs beside the merge, the scope joining it at its end.
                let gave_way = std::thread::scope(|s| {
                    let (handed, collected) = std::sync::mpsc::channel::<(usize, Worker<'a>)>();
                    let n = others.len();
                    let empty: Vec<bool> = others.iter().map(Option::is_none).collect();
                    let mut handles = Vec::with_capacity(n);
                    for (k, (mut worker, first)) in others.into_iter().enumerate().filter_map(|(k, o)| Some((k, o?))) {
                        let queue = &queue;
                        let handed = handed.clone();
                        let handle = std::thread::Builder::new()
                            .name(format!("teq-worker-{}", k + 1))
                            .stack_size(1 << 30)
                            .spawn_scoped(s, move || {
                                crate::alloc::enter();
                                let launched = since();
                                worker.work(queue, first);
                                let ended = since();
                                let kept = inventory.then(|| (crate::alloc::thread_free(), crate::memory::stack_resident(), crate::interp::inventory()));
                                let _ = handed.send((k, worker));
                                // The merge waits for every sender to go, this one before the end below.
                                drop(handed);
                                // A session outlives the build: the cycles this worker's
                                // macro runs left go with its thread (`interp::dispose`).
                                if crate::alloc::is_resident() {
                                    crate::interp::dispose();
                                }
                                if let Some((free, stack, interp)) = kept {
                                    let mut items = vec![
                                        ("launched us", launched),
                                        ("ended us", ended),
                                        ("disposed us", since()),
                                        ("free bytes", free),
                                        ("disposed bytes", crate::alloc::thread_free().saturating_sub(free)),
                                        ("stack resident", stack),
                                    ];
                                    items.extend(interp);
                                    items.push(("returned us", since()));
                                    crate::measure::inventory_line(&format!("worker {}", k + 1), &items);
                                }
                            })
                            .expect("cannot start a typing worker");
                        handles.push(handle);
                    }
                    drop(handed);
                    let spawned = since();
                    w.work(&queue, first);
                    let main_ended = since();
                    crate::measure::phase_done("workers, until the main worker's end");
                    let mut others: Vec<Option<Worker<'a>>> = (0..n).map(|_| None).collect();
                    for (k, worker) in collected.iter() {
                        others[k] = Some(worker);
                    }
                    // A worker that panicked handed nothing; its thread reported the panic. An
                    // empty slot was never started.
                    if others.iter().zip(&empty).any(|(o, &empty)| o.is_none() && !empty) {
                        std::process::exit(101);
                    }
                    if inventory {
                        crate::measure::inventory_line(
                            "handed over",
                            &[
                                ("workers", threads),
                                ("spawned us", spawned),
                                ("main ended us", main_ended),
                                ("handed over us", since()),
                                ("main stack resident", crate::memory::stack_resident()),
                            ],
                        );
                    }
                    crate::measure::phase_done("workers, the others joined");
                    if let Some((file, at)) = w.diagnostics_meeting(&others) {
                        w.need_serial_for_diagnostics(|| format!("two diagnostics at one place (file {} at {})", file.0, at));
                    }
                    // The build is typed again by one worker (`need_serial`): what the workers
                    // made goes with them.
                    let gave_way = w.serial.is_needed();
                    if !gave_way {
                        w.merge_workers(others, &prefix);
                    }
                    // Joined through their handles, the threads' own handles are freed here, not
                    // on the ending threads, whose lists they would be lost with (`alloc.rs`).
                    for h in handles {
                        if h.join().is_err() {
                            std::process::exit(101);
                        }
                    }
                    gave_way
                });
                if gave_way {
                    crate::interp::watch(false);
                    return;
                }
                crate::interp::watch(census);
                crate::measure::phase_done("merge");
                if w.merge_failed {
                    prep::capture(false);
                    return;
                }
            } else {
                w.check_files();
                crate::measure::phase_done("walk");
                if w.profile.on {
                    // With one worker the merge renumbers nothing; under `--profile` its walk
                    // is the measurement of what the parallel typer's merge costs (`merge.rs`).
                    w.merge(&prefix);
                    crate::measure::phase_done("merge");
                }
                w.complete_pending_sealed_files();
            }
            w.register_late_methods();
            w.settle_js_imports();
            w.run_deferred_matches();
            w.check_outer_accessors_implemented(None);
            w.final_passes();
            let p = w.phase(profile::Phase::Final);
            w.check_deferred_bounds();
            w.phase_end(p);
            w.report_unused(None);
            w.tell_cacheable_warnings();
            w.index_settle();
            crate::measure::phase_done("final");
            prep::capture(false);
            if census {
                crate::interp::watch(false);
                quoted::print_census();
            }
            // A session keeps the stores across builds: what they outgrew is freed here as at a
            // retype. A build that ends with the process keeps it (`incremental.rs`).
            if crate::alloc::is_resident() {
                w.types.release_retired();
                w.interner.release_retired();
            }
        });
    }

    /// Runs `f` over the worker where the phases run (`thread::run`).
    pub fn on_thread<R: Send>(&mut self, f: impl FnOnce(&mut Worker<'a>) -> R + Send) -> R {
        let w = &mut self.w;
        thread::run(move || f(w))
    }
}

impl<'a> Worker<'a> {
    pub fn new(asts: &'a Asts, files: &'a Sources, interner: &'a Interner) -> Worker<'a> {
        let mut syms = Symbols::new();
        let types = TypeStore::new();
        let scala_pkg = syms.sub_pkg(ROOT_PKG, crate::names::SCALA);
        let builtin = |name: Name, syms: &mut Symbols| {
            let c = syms.new_class(
                name,
                ClassKind::Builtin,
                crate::ast::mods::FINAL,
                Owner::Package(scala_pkg),
                FileId(0),
                None,
                Span::default(),
            );
            syms.class(c).state().set(Completion::Done);
            syms.pkgs[scala_pkg.idx()].entries.entry(name).or_default().class = Some(c);
            c
        };
        use crate::names as n;
        let int = builtin(n::INT, &mut syms);
        let long = builtin(n::LONG, &mut syms);
        let double = builtin(n::DOUBLE, &mut syms);
        let byte = builtin(n::BYTE, &mut syms);
        let short = builtin(n::SHORT, &mut syms);
        let float = builtin(n::FLOAT, &mut syms);
        let boolean = builtin(n::BOOLEAN, &mut syms);
        let string = builtin(n::STRING, &mut syms);
        let char = builtin(n::CHAR, &mut syms);
        let unit = builtin(n::UNIT, &mut syms);
        let array = builtin(n::ARRAY, &mut syms);
        let null = builtin(n::NULL, &mut syms);
        let any_ref = builtin(n::ANY_REF, &mut syms);
        let any_val = builtin(n::ANY_VAL, &mut syms);
        let elem = syms.new_tparam(interner.intern("T"), 0);
        syms.classes[array.idx()].tparams.push(elem);
        for c in [int, long, double, byte, short, float, boolean, string, char, unit, array, null, any_ref, any_val] {
            let targs: Vec<TypeId> =
                syms.class(c).tparams.iter().map(|&p| types.param(p)).collect();
            let self_ty = types.class(c, &targs);
            syms.classes[c.idx()].base_types.push((c, self_ty));
        }
        let mut num_rank = [NO_RANK; NUM_RANK_TABLE];
        for (c, rank) in [
            (byte, prims::R_BYTE),
            (short, prims::R_SHORT),
            (char, prims::R_CHAR),
            (int, prims::R_INT),
            (long, prims::R_LONG),
            (float, prims::R_FLOAT),
            (double, prims::R_DOUBLE),
        ] {
            num_rank[types.class(c, &[]).idx()] = rank;
        }
        let b = Builtins {
            int,
            long,
            double,
            byte,
            short,
            float,
            boolean,
            string,
            char,
            unit,
            array,
            null,
            any_ref,
            any_val,
            t_int: types.class(int, &[]),
            t_long: types.class(long, &[]),
            t_double: types.class(double, &[]),
            t_byte: types.class(byte, &[]),
            t_short: types.class(short, &[]),
            t_float: types.class(float, &[]),
            t_boolean: types.class(boolean, &[]),
            t_string: types.class(string, &[]),
            t_char: types.class(char, &[]),
            t_unit: types.class(unit, &[]),
            t_null: types.class(null, &[]),
            t_any_ref: types.class(any_ref, &[]),
            t_any_val: types.class(any_val, &[]),
            functions: Vec::new(),
            context_functions: Vec::new(),
            by_name: None,
            repeated: None,
            tuples: Vec::new(),
            seq: None,
            string_context: None,
            conversion: None,
            partial_function: None,
            sub_evidence: None,
            eq_evidence: None,
            option: None,
            named_tuple: None,
            either: None,
            set: None,
            map: None,
            can_equal: None,
            value_of: None,
            selectable: None,
            number: None,
            js_dynamic: None,
            throwable: None,
            js_exception: None,
            cons_tuple: None,
            product: None,
            t_product: ERROR,
            t_singleton: ERROR,
            t_equals: ERROR,
            reflect_enum: None,
            t_enum: ERROR,
            tuple_trait: None,
            non_empty_tuple: None,
            empty_tuple: None,
            tuple_members: Vec::new(),
            tuple_member_bits: [0; 4],
            predef_names: Default::default(),
            product_helpers: Vec::new(),
            scala_pkg,
            num_rank,
        };
        let n_files = asts.len();
        Worker {
            asts,
            interner,
            syms,
            types: std::sync::Arc::new(types),
            diags: Diagnostics::default(),
            prog: Program { file_tags: crate::arena::FileVec::from_vec(crate::source::file_tags(files.as_slice().iter().map(|f| f.key.as_str())).into_iter().map(u64::from).collect()), ..Program::default() },
            b,
            env: Env { file: FileId(0), frames: Vec::new(), imports: Vec::new() },
            def_syms: crate::arena::FileMaps::new(n_files),
            def_classes: crate::arena::FileMaps::new(n_files),
            def_aliases: crate::arena::FileMaps::new(n_files),
            file_pkgs: crate::arena::FileVec::from_vec(vec![ROOT_PKG; n_files]),
            file_imports: crate::arena::FileVec::from_vec((0..n_files).map(|_| None).collect()),
            import_hidden: std::sync::Arc::new(crate::shared::SlabVec::with_capacity(16)),
            import_sels: std::sync::Arc::new(crate::shared::SlabVec::with_capacity(16)),
            class_imports: Default::default(),
            ext_scope: None,
            tvars: TVars::new(),
            app_base: 0,
            trail: Vec::new(),
            attempts: Default::default(),
            attempt_caches: Vec::new(),
            fun_of_sym: Default::default(),
            interpolations: FxMap::default(),
            bodies_in_progress: Vec::new(),
            var_frames: Vec::new(),
            val_init: Default::default(),
            class_done: Default::default(),
            implicit_depth: 0,
            search_only_depth: None,
            implicit_budget: 0,
            given_ambiguity: None,
            failed_givens: Vec::new(),
            given_stack: Vec::new(),
            search_byname: false,
            typing_annotation: false,
            annotation_args: false,
            measured_target: None,
            defining: None,
            conversion_indexes: FxMap::default(),
            view_compat: false,
            spread_arg: false,
            mismatches: Vec::new(),
            logging: 0,
            app_depth: 0,
            retry_depth: None,
            retry_typed: FxMap::default(),
            arg_cache: None,
            arg_caches: Vec::new(),
            infix_tuples: FxMap::default(),
            lexical_selected: None,
            ext_undecided: false,
            untyped_lambda: None,
            no_receiver_conversion: false,
            declared_call: None,
            declared_took: None,
            infix_declared: None,
            direct_ext_targs: false,
            macro_targs: None,
            dropped_type_list: None,
            assign_op_target: None,
            assign_op_receiver: None,
            open_receiver: false,
            receiver_left_open: false,
            open_applied_receiver: None,
            inferred_outcomes: FxMap::default(),
            inferred_val_outcomes: FxMap::default(),
            soft_exprs: FxMap::default(),
            joined_soft: false,
            soft_syms: FxMap::default(),
            soft_scrutinee: None,
            converting_receiver: false,
            blocked_conversion: None,
            return_to: None,
            returns: Vec::new(),
            nowarn: 0,
            unused: Default::default(),
            dependent_checks: 0,
            recovered: false,
            error_nodes: 0,
            transparent: Vec::new(),
            file_opaques: crate::arena::FileVec::from_vec(vec![Vec::new(); n_files]),
            retained_bodies: Default::default(),
            inline_definitions: Default::default(),
            superseded_definitions: Vec::new(),
            check_capture_keys: Vec::new(),
            last_arg_types: Vec::new(),
            pending_widenings: Vec::new(),
            hoisted: Vec::new(),
            lub_in_progress: Vec::new(),
            join_wild: false,
            necessary_either: false,
            soft_lub: false,
            completing: 0,
            pending_library_names: Vec::new(),
            naming_library: 0,
            poisoned: FxMap::default(),
            poisoned_withheld: FxMap::default(),
            withheld_notes: FxMap::default(),
            reflective_jar_found: None,
            renamed_roots: Vec::new(),
            unchecked_variance: Default::default(),
            excluded_ctor: None,
            variance_skip: Vec::new(),
            for_packs: Vec::new(),
            for_bases: Vec::new(),
            given_indexes: FxMap::default(),
            value_sigs: FxMap::default(),
            temp_names: FxMap::default(),
            file_chains: vec![None; n_files],
            class_exports: exports::ExportTables::new(),
            pkg_exports: exports::ExportTables::new(),
            export_stack: Vec::new(),
            class_given_indexes: FxMap::default(),
            given_preferences: FxMap::default(),
            given_orders: FxMap::default(),
            implicit_scopes: FxMap::default(),
            implicit_scope_givens: FxMap::default(),
            package_levels: FxMap::default(),
            given_fast: FxMap::default(),
            given_context: Vec::new(),
            given_consulted: Vec::new(),
            given_opened: Vec::new(),
            given_tree: Default::default(),
            trace_starts: Vec::new(),
            given_fits: FxMap::default(),
            head_sigs: FxMap::default(),
            derived_aliases: FxMap::default(),
            derived_opaques: Default::default(),
            function_targets: Vec::new(),
            ext_modules: Vec::new(),
            import_values: std::sync::Arc::new(crate::shared::SlabVec::with_capacity(16)),
            export_blocks: 0,
            export_retry: Vec::new(),
            export_retrying: false,
            js_import_ids: FxMap::default(),
            js_registry: None,
            js_imports_at_bodies: 0,
            js_exported: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            worker: 0,
            arity_classes: Default::default(),
            items: Vec::new(),
            tclasses_published: 0,
            tclasses_registered: FxMap::default(),
            tclasses_settling: Vec::new(),
            tclass_settling_scopes: 0,
            tclasses_in_checks: Vec::new(),
            pending_shared: Vec::new(),
            merging: Vec::new(),
            walk_by_size: false,
            bodies_started: false,
            infos: None,
            std_seen: 0,
            lists_at_fork: 0,
            std_entered_pending: Vec::new(),
            loader_done_pending: Vec::new(),
            arity_seen: 0,
            tvars_at_fork: 0,
            files,
            sites: site::DefSites::default(),
            enum_case_new: None,
            ascribed_constant: None,
            inferred_parent_args: Default::default(),
            inferred_trait_args: Default::default(),
            parent_args_of: None,
            deferred_impl_of: None,
            sam_classes: FxMap::default(),
            inline_accessor_syms: FxMap::default(),
            inline_results: FxMap::default(),
            expr_marks: FxMap::default(),
            eta_expansions: FxMap::default(),
            folded_paths: FxMap::default(),
            apply_selected: false,
            gadt: Vec::new(),
            wildcards_used: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            captured_locals: Default::default(),
            index: None,
            deps: None,
            gadt_open: false,
            dead_case: false,
            partial_match: false,
            spaces: space::SpaceScratch::default(),
            deferred_matches: Vec::new(),
            late_methods: Vec::new(),
            override_pairs: Vec::new(),
            derive: derive::Derivation::default(),
            anon_envs: FxMap::default(),
            irrefutable_pats: FxMap::default(),
            tag_pats: FxMap::default(),
            arg_types_seen: Vec::new(),
            erroneous_args: Vec::new(),
            erroneous_lambdas: Vec::new(),
            local_news: Vec::new(),
            anon_captures: FxMap::default(),
            anon_parent_args: FxMap::default(),
            mixin_supers: FxMap::default(),
            shared_mixin_supers: Default::default(),
            super_problems_told: FxMap::default(),
            mixins_bound: Default::default(),
            inference_failed: FxMap::default(),
            any_members: Default::default(),
            anon_sites: FxMap::default(),
            body_node: None,
            outer_this: Default::default(),
            outer_accessors: Default::default(),
            new_prefixed: false,
            explicit_new: false,
            reflect_ctor_params: FxMap::default(),
            reflect_param_of: FxMap::default(),
            extension_call_head: false,
            entry_points: Vec::new(),
            main_name: None,
            choose_entry: true,
            open_world: false,
            jvm_volatile: FxMap::default(),
            writes_products: false,
            withheld_met: 0,
            dialect: crate::dialect::Dialect::default(),
            deferred_bounds: Vec::new(),
            jvm: false,
            unit_ops: None,
            interp: false,
            for_interpreter: false,
            erased_tags: None,
            poly_targs: None,
            poly_pending: Vec::new(),
            unsettled_std_entries: Vec::new(),
            dispatch_pending: Vec::new(),
            named_like: Vec::new(),
            vals_by_accessor: false,
            suffixed_roots: FxMap::default(),
            prog_index: check::ProgIndex::default(),
            roots_memo: FxMap::default(),
            subclasses: overload::SubclassIndex::default(),
            alternatives_named: false,
            inherited_pairs: Vec::new(),
            loaded: None,
            std: std::sync::Arc::new(stdlib::StdCell::new(stdlib::StdFiles::default())),
            entering_done: false,
            walk: Walk::default(),
            shadowed_object: None,
            bare_lookup: false,
            bound_path: Vec::new(),
            array_seq_asked: false,
            sealed_std_files: Vec::new(),
            late_naming_from: None,
            dep_depth: 0,
            lookups: Vec::new(),
            body_depth: 0,
            show_depth: std::cell::Cell::new(0),
            dep_budget: 0,
            dep_root: (ERROR, crate::names::EMPTY),
            dep_exhausted: false,
            singleton_depth: 0,
            cycle_at_use: None,
            param_paths: Vec::new(),
            path_approx: Vec::new(),
            refinement_sigs: FxMap::default(),
            poly_rebound: FxMap::default(),
            poly_literal_fun: None,
            outer_prefixes: FxMap::default(),
            sam_arity: FxMap::default(),
            member_cycles: Vec::new(),
            merged_overloads: Default::default(),
            views_on: false,
            note_paths: false,
            inline: inline::InlineState::new(),
            profile: profile::Profile::default(),
            matches: matchtypes::MatchState::default(),
            other_view_memos: Default::default(),
            case_binders: Vec::new(),
            last_typed_pattern: None,
            refutable_is_error: false,
            applied_extension: None,
            quote: quoted::QuoteState::default(),
            macro_files: FxMap::default(),
            cacheable_state: Vec::new(),
            cacheable_state_classes: Vec::new(),
            known_caches: true,
            macro_state_per_worker: false,
            inline_deps: FxMap::default(),
            lock: std::sync::Arc::new(crate::shared::ReentrantLock::new()),
            forked: false,
            mapping_signature: false,
            merge_failed: false,
            serial: Default::default(),
            types_at_fork: 0,
            predecoded: None,
        }
    }

    /// Another worker over the same shared regions: what the signature
    /// phase made is read by reference or copied, its own records go to its own chunk under
    /// `worker`'s ids, and `merge_workers` takes them back after the bodies.
    pub fn attach(&self, worker: usize) -> Worker<'a> {
        assert!(self.forked && worker < crate::arena::MAX_WORKERS);
        Worker {
            asts: self.asts,
            interner: self.interner,
            syms: self.syms.attach(worker),
            types: self.types.clone(),
            diags: Default::default(),
            prog: self.prog.attach(worker),
            b: self.b.clone(),
            env: Env { file: FileId(0), frames: Vec::new(), imports: Vec::new() },
            def_syms: self.def_syms.attach(),
            def_classes: self.def_classes.attach(),
            def_aliases: self.def_aliases.attach(),
            file_pkgs: self.file_pkgs.attach(worker),
            file_imports: self.file_imports.attach(worker),
            import_hidden: self.import_hidden.clone(),
            import_sels: self.import_sels.clone(),
            class_imports: self.class_imports.attach(),
            ext_scope: Default::default(),
            tvars: TVars::attach(&self.tvars, worker as u32, self.tvars_at_fork),
            app_base: Default::default(),
            trail: Default::default(),
            attempts: Default::default(),
            attempt_caches: Vec::new(),
            fun_of_sym: self.fun_of_sym.attach(),
            // The navigation index's tables the typing reads, its records the worker's own,
            // which the merge joins in the work items' order (`merge_workers`).
            index: self.index.as_ref().map(|ix| Box::new(ix.attach())),
            deps: self.deps.as_ref().map(|_| Box::default()),
            soft_exprs: self.soft_exprs.clone(),
            soft_syms: self.soft_syms.clone(),
            joined_soft: false,
            soft_scrutinee: None,
            ascribed_constant: None,
            inline_deps: Default::default(),
            interpolations: self.interpolations.clone(),
            bodies_in_progress: Default::default(),
            var_frames: Vec::new(),
            val_init: self.val_init.attach(),
            class_done: self.class_done.attach(),
            implicit_depth: Default::default(),
            search_only_depth: Default::default(),
            implicit_budget: Default::default(),
            given_ambiguity: Default::default(),
            failed_givens: Default::default(),
            given_stack: Default::default(),
            search_byname: Default::default(),
            typing_annotation: false,
            annotation_args: false,
            measured_target: Default::default(),
            defining: Default::default(),
            conversion_indexes: self.conversion_indexes.clone(),
            view_compat: Default::default(),
            spread_arg: false,
            mismatches: Vec::new(),
            logging: 0,
            app_depth: 0,
            retry_depth: None,
            retry_typed: FxMap::default(),
            arg_cache: None,
            arg_caches: Vec::new(),
            infix_tuples: FxMap::default(),
            lexical_selected: None,
            ext_undecided: false,
            untyped_lambda: None,
            no_receiver_conversion: Default::default(),
            declared_call: None,
            declared_took: None,
            infix_declared: None,
            direct_ext_targs: Default::default(),
            macro_targs: None,
            dropped_type_list: Default::default(),
            assign_op_target: None,
            assign_op_receiver: None,
            open_receiver: Default::default(),
            receiver_left_open: Default::default(),
            open_applied_receiver: Default::default(),
            inferred_outcomes: self.inferred_outcomes.clone(),
            inferred_val_outcomes: self.inferred_val_outcomes.clone(),
            converting_receiver: Default::default(),
            blocked_conversion: Default::default(),
            return_to: Default::default(),
            returns: Default::default(),
            nowarn: Default::default(),
            unused: self.unused.attach(),
            dependent_checks: Default::default(),
            recovered: self.recovered,
            error_nodes: Default::default(),
            transparent: Default::default(),
            file_opaques: self.file_opaques.attach(worker),
            retained_bodies: self.retained_bodies.attach(),
            inline_definitions: self.inline_definitions.attach(),
            superseded_definitions: Vec::new(),
            check_capture_keys: Vec::new(),
            last_arg_types: Default::default(),
            pending_widenings: Default::default(),
            hoisted: Default::default(),
            lub_in_progress: Default::default(),
            join_wild: Default::default(),
            soft_lub: Default::default(),
            necessary_either: Default::default(),
            completing: Default::default(),
            pending_library_names: Default::default(),
            naming_library: Default::default(),
            poisoned: self.poisoned.clone(),
            poisoned_withheld: self.poisoned_withheld.clone(),
            withheld_notes: self.withheld_notes.clone(),
            reflective_jar_found: self.reflective_jar_found.clone(),
            renamed_roots: Default::default(),
            for_packs: Default::default(),
            for_bases: Default::default(),
            given_indexes: self.given_indexes.clone(),
            value_sigs: self.value_sigs.clone(),
            temp_names: self.temp_names.clone(),
            file_chains: self.file_chains.clone(),
            class_exports: self.class_exports.attach(),
            pkg_exports: self.pkg_exports.attach(),
            export_stack: Default::default(),
            class_given_indexes: self.class_given_indexes.clone(),
            given_preferences: self.given_preferences.clone(),
            given_orders: self.given_orders.clone(),
            implicit_scopes: self.implicit_scopes.clone(),
            implicit_scope_givens: self.implicit_scope_givens.clone(),
            package_levels: self.package_levels.clone(),
            given_fast: self.given_fast.clone(),
            given_context: Default::default(),
            given_consulted: Default::default(),
            given_opened: Default::default(),
            given_tree: Default::default(),
            trace_starts: Default::default(),
            given_fits: self.given_fits.clone(),
            head_sigs: self.head_sigs.clone(),
            derived_aliases: self.derived_aliases.clone(),
            derived_opaques: self.derived_opaques.attach(),
            arity_classes: self.arity_classes.attach(),
            merging: Vec::new(),
            walk_by_size: self.walk_by_size,
            bodies_started: self.bodies_started,
            infos: self.infos.as_ref().map(|_| Vec::new()),
            std_seen: self.std_seen,
            lists_at_fork: self.lists_at_fork,
            std_entered_pending: Vec::new(),
            loader_done_pending: Vec::new(),
            arity_seen: 0,
            tvars_at_fork: self.tvars_at_fork,
            function_targets: Default::default(),
            ext_modules: Default::default(),
            import_values: self.import_values.clone(),
            export_blocks: Default::default(),
            export_retry: Default::default(),
            export_retrying: Default::default(),
            js_import_ids: Default::default(),
            js_registry: self.js_registry.clone(),
            js_imports_at_bodies: self.js_imports_at_bodies,
            js_exported: self.js_exported.clone(),
            worker: worker,
            files: self.files,
            sites: self.sites.clone(),
            enum_case_new: Default::default(),
            inferred_parent_args: self.inferred_parent_args.attach(),
            inferred_trait_args: self.inferred_trait_args.attach(),
            parent_args_of: Default::default(),
            deferred_impl_of: None,
            sam_classes: self.sam_classes.clone(),
            inline_accessor_syms: self.inline_accessor_syms.clone(),
            inline_results: self.inline_results.clone(),
            expr_marks: self.expr_marks.clone(),
            eta_expansions: self.eta_expansions.clone(),
            folded_paths: self.folded_paths.clone(),
            apply_selected: Default::default(),
            gadt: Default::default(),
            wildcards_used: self.wildcards_used.clone(),
            captured_locals: self.captured_locals.clone(),
            gadt_open: Default::default(),
            dead_case: Default::default(),
            partial_match: Default::default(),
            spaces: Default::default(),
            deferred_matches: Vec::new(),
            late_methods: Vec::new(),
            override_pairs: Vec::new(),
            derive: self.derive.attach(),
            anon_envs: self.anon_envs.clone(),
            irrefutable_pats: self.irrefutable_pats.clone(),
            tag_pats: self.tag_pats.clone(),
            arg_types_seen: Default::default(),
            erroneous_args: Default::default(),
            erroneous_lambdas: Default::default(),
            local_news: self.local_news.clone(),
            anon_captures: self.anon_captures.clone(),
            anon_parent_args: self.anon_parent_args.clone(),
            mixin_supers: self.mixin_supers.clone(),
            shared_mixin_supers: self.shared_mixin_supers.clone(),
            super_problems_told: self.super_problems_told.clone(),
            mixins_bound: Default::default(),
            any_members: self.any_members.attach(),
            anon_sites: self.anon_sites.clone(),
            body_node: None,
            outer_this: self.outer_this.attach(),
            outer_accessors: self.outer_accessors.attach(),
            new_prefixed: Default::default(),
            explicit_new: false,
            reflect_ctor_params: self.reflect_ctor_params.clone(),
            reflect_param_of: self.reflect_param_of.clone(),
            extension_call_head: Default::default(),
            entry_points: Default::default(),
            deferred_bounds: Default::default(),
            unchecked_variance: self.unchecked_variance.attach(),
            inference_failed: FxMap::default(),
            excluded_ctor: Default::default(),
            variance_skip: Default::default(),
            main_name: self.main_name,
            choose_entry: self.choose_entry,
            open_world: self.open_world,
            jvm_volatile: FxMap::default(),
            writes_products: self.writes_products,
            withheld_met: 0,
            dialect: self.dialect,
            jvm: self.jvm,
            unit_ops: self.unit_ops,
            interp: self.interp,
            for_interpreter: self.for_interpreter,
            erased_tags: None,
            poly_targs: None,
            poly_pending: Vec::new(),
            cacheable_state: self.cacheable_state.clone(),
            known_caches: self.known_caches,
            macro_state_per_worker: self.macro_state_per_worker,
            cacheable_state_classes: self.cacheable_state_classes.clone(),
            unsettled_std_entries: Default::default(),
            dispatch_pending: Default::default(),
            named_like: Default::default(),
            vals_by_accessor: false,
            suffixed_roots: self.suffixed_roots.clone(),
            prog_index: Default::default(),
            roots_memo: self.roots_memo.clone(),
            subclasses: Default::default(),
            alternatives_named: Default::default(),
            inherited_pairs: Default::default(),
            loaded: self.loaded.clone(),
            std: self.std.clone(),
            entering_done: self.entering_done,
            walk: self.walk.clone(),
            shadowed_object: Default::default(),
            bare_lookup: Default::default(),
            bound_path: Vec::new(),
            array_seq_asked: Default::default(),
            sealed_std_files: Default::default(),
            late_naming_from: Default::default(),
            dep_depth: Default::default(),
            body_depth: Default::default(),
            show_depth: Default::default(),
            dep_budget: self.dep_budget,
            dep_root: (ERROR, crate::names::EMPTY),
            dep_exhausted: Default::default(),
            singleton_depth: Default::default(),
            cycle_at_use: Default::default(),
            lookups: Default::default(),
            param_paths: Default::default(),
            path_approx: Default::default(),
            refinement_sigs: Default::default(),
            poly_rebound: Default::default(),
            poly_literal_fun: Default::default(),
            outer_prefixes: self.outer_prefixes.clone(),
            sam_arity: self.sam_arity.clone(),
            member_cycles: Default::default(),
            merged_overloads: self.merged_overloads.attach(),
            views_on: false,
            note_paths: Default::default(),
            inline: inline::InlineState::worker(&self.inline),
            profile: profile::Profile::worker(self.profile.on, self.profile.sig_hook),
            matches: Default::default(),
            other_view_memos: Default::default(),
            case_binders: Default::default(),
            last_typed_pattern: Default::default(),
            refutable_is_error: false,
            applied_extension: None,
            quote: self.quote.attach(),
            macro_files: self.macro_files.clone(),
            lock: self.lock.clone(),
            forked: true,
            mapping_signature: false,
            merge_failed: false,
            serial: self.serial.clone(),
            types_at_fork: self.types_at_fork,
            predecoded: None,
            items: Vec::new(),
            tclasses_published: 0,
            tclasses_registered: FxMap::default(),
            tclasses_settling: Vec::new(),
            tclass_settling_scopes: 0,
            tclasses_in_checks: Vec::new(),
            pending_shared: Vec::new(),
        }
    }

    /// A typing whose variables are numbered in its own order for their display from here
    /// (`TVarInfo::shown`): an item's, a class's or a body's, which a worker may type on its own or
    /// inside another's, as one worker may. The frame, for `var_frame_end`.
    pub fn var_frame_begin(&mut self) -> usize {
        self.var_frames.push((self.tvars.len() as u32, 0));
        self.var_frames.len()
    }

    /// The typing `var_frame_begin` began is done: what it made is the enclosing frame's to leave
    /// out of its own numbering.
    pub fn var_frame_end(&mut self, frame: usize) {
        // The end of an item: its pending plain inline calls are expanded, inside its frame
        // (`state::flush_pending_inline`).
        if frame == 1 && self.attempts.flushes_at_item_end() {
            self.flush_pending_inline();
        }
        self.var_frames.truncate(frame);
        let Some((start, _)) = self.var_frames.pop() else { return };
        let made = self.tvars.len() as u32 - start;
        if let Some(outer) = self.var_frames.last_mut() {
            outer.1 += made;
        }
    }

    /// The lengths of the worker's own chunks (`ItemRange`).
    pub fn own_marks(&self) -> merge::Prefix {
        let p = &self.prog;
        merge::Prefix {
            syms: self.syms.syms.own().len() as u32,
            classes: self.syms.classes.own().len() as u32,
            tparams: self.syms.tparams.own().len() as u32,
            aliases: self.syms.aliases.own().len() as u32,
            pkgs: self.syms.pkgs.own().len() as u32,
            overloads: self.syms.overloads.own().len() as u32,
            exprs: p.exprs.own().len() as u32,
            pats: p.pats.own().len() as u32,
            strings: p.strings.own().len() as u32,
            expr_lists: p.expr_lists.own().len() as u32,
            pat_lists: p.pat_lists.own().len() as u32,
            sym_lists: p.sym_lists.own().len() as u32,
            stmts: p.stmts.own().len() as u32,
            cases: p.cases.own().len() as u32,
            tries: p.tries.own().len() as u32,
            tests: p.tests.own().len() as u32,
            funs: p.funs.own().len() as u32,
            tclasses: p.classes.own().len() as u32,
            quotes: p.quotes.own().len() as u32,
            quote_pats: p.quote_pats.own().len() as u32,
            types: 0,
            names: 0,
            journal: self.index.as_ref().map_or(0, |ix| ix.journal_len()),
        }
    }

    /// Records that `method` overrides `base` for the dispatch tables (`Program::overrides`):
    /// under the loader's lock the entry is every worker's.
    pub fn push_override(&mut self, method: SymId, base: SymId) {
        if self.prog.overrides.to_shared {
            let mut bases = self.prog.overrides.get(&method).cloned().unwrap_or_default();
            if !bases.contains(&base) {
                bases.push(base);
            }
            self.prog.overrides.insert(method, bases);
        } else {
            let entry = self.prog.overrides.local.entry(method).or_default();
            if !entry.contains(&base) {
                entry.push(base);
            }
        }
    }

    /// Whether the output is an ES module: a JS import or export anywhere in the program, on
    /// any worker.
    pub fn is_module(&self) -> bool {
        match &self.js_registry {
            Some(r) => r.list.len() > 0 || self.js_exported.load(std::sync::atomic::Ordering::Acquire),
            None => self.prog.is_module(),
        }
    }

    /// Runs `f` under the loader's lock: what it pushes goes to the
    /// shared region and what it changes of a shared record is published when the outermost hold
    /// ends. Without a shared region there is nothing to lock, and the call is `f`'s alone.
    #[inline(always)]
    pub fn with_loader<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        if !self.forked {
            return f(self);
        }
        self.with_loader_locked(f)
    }

    /// `with_loader` for work of the kind `cat`, which the measurement of the lock's holds
    /// charges its stretches to (`measure::Hold`).
    #[inline(always)]
    pub fn with_loader_for<R>(&mut self, cat: crate::measure::Hold, f: impl FnOnce(&mut Self) -> R) -> R {
        if !self.forked {
            return f(self);
        }
        self.with_loader_as(cat, f)
    }

    #[inline(never)]
    fn with_loader_as<R>(&mut self, cat: crate::measure::Hold, f: impl FnOnce(&mut Self) -> R) -> R {
        let _measured = crate::measure::hold_for(cat);
        self.with_loader_locked(f)
    }

    #[inline(never)]
    fn with_loader_locked<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let lock = self.lock.clone();
        lock.lock();
        let outermost = crate::shared::lock_depth() == 1;
        if outermost {
            crate::measure::hold_begin();
            let _measured = crate::measure::hold_for(crate::measure::Hold::Entry);
            crate::shake::point(crate::shake::Point::LockTaken);
            self.lock_taken();
        }
        let r = f(self);
        if outermost {
            // Releasing is charged to its category up to the hold's end, the unlock included.
            let _measured = crate::measure::hold_for(crate::measure::Hold::Release);
            self.lock_released();
            for i in std::mem::take(&mut self.std_entered_pending) {
                self.std.slots[i as usize].visible.store(true, std::sync::atomic::Ordering::Release);
            }
            if !self.loader_done_pending.is_empty() {
                let loaded = self.loaded.as_ref().expect("the loader finished work without a classpath");
                for done in std::mem::take(&mut self.loader_done_pending) {
                    loaded.mark_done(done);
                }
            }
            lock.unlock();
            crate::measure::hold_end();
        } else {
            lock.unlock();
        }
        r
    }

    fn arenas_alloc_shared(&mut self, on: bool) {
        self.fun_of_sym.to_shared = on;
        self.val_init.to_shared = on;
        self.class_done.to_shared = on;
        self.class_imports.to_shared = on;
        self.retained_bodies.to_shared = on;
        self.inline_definitions.to_shared = on;
        self.any_members.to_shared = on;
        self.outer_accessors.to_shared = on;
        self.merged_overloads.to_shared = on;
        self.derived_opaques.to_shared = on;
        self.derive.to_shared(on);
        self.syms.dispatch_names.to_shared = on;
        self.arity_classes.to_shared = on;
        self.prog.template_syms.to_shared = on;
        self.prog.top_funs.alloc_shared = on;
        self.prog.top_vals.alloc_shared = on;
        self.prog.overrides.to_shared = on;
        self.inferred_parent_args.to_shared = on;
        self.inferred_trait_args.to_shared = on;
        self.unchecked_variance.to_shared = on;
        self.outer_this.to_shared = on;
        self.quote.to_shared(on);
        self.def_syms.to_shared = on;
        self.def_classes.to_shared = on;
        self.def_aliases.to_shared = on;
        let s = &mut self.syms;
        for a in [&mut s.syms.alloc_shared, &mut s.classes.alloc_shared, &mut s.tparams.alloc_shared, &mut s.aliases.alloc_shared, &mut s.pkgs.alloc_shared, &mut s.overloads.alloc_shared] {
            *a = on;
        }
        let p = &mut self.prog;
        for a in [
            &mut p.exprs.alloc_shared,
            &mut p.pats.alloc_shared,
            &mut p.strings.alloc_shared,
            &mut p.expr_lists.alloc_shared,
            &mut p.pat_lists.alloc_shared,
            &mut p.sym_lists.alloc_shared,
            &mut p.stmts.alloc_shared,
            &mut p.cases.alloc_shared,
            &mut p.tries.alloc_shared,
            &mut p.tests.alloc_shared,
            &mut p.funs.alloc_shared,
            &mut p.classes.alloc_shared,
            &mut p.quotes.alloc_shared,
            &mut p.quote_pats.alloc_shared,
            &mut p.expr_types.alloc_shared,
            &mut p.expr_spans.alloc_shared,
            &mut p.expansion_bits.alloc_shared,
            &mut p.leaf_bits.alloc_shared,
        ] {
            *a = on;
        }
    }

    fn lock_taken(&mut self) {
        self.syms.sweep_views();
        self.swap_view_memos();
        self.refresh_std_if_stale();
        self.sync_arity_tables();
        self.file_pkgs.lock_taken();
        self.file_imports.lock_taken();
        self.file_opaques.lock_taken();
        self.prog.file_tags.lock_taken();
        let s = &mut self.syms;
        s.syms.lock_taken();
        s.classes.lock_taken();
        s.tparams.lock_taken();
        s.aliases.lock_taken();
        s.pkgs.lock_taken();
        s.overloads.lock_taken();
        let p = &mut self.prog;
        p.exprs.lock_taken();
        p.pats.lock_taken();
        p.strings.lock_taken();
        p.expr_lists.lock_taken();
        p.pat_lists.lock_taken();
        p.sym_lists.lock_taken();
        p.stmts.lock_taken();
        p.cases.lock_taken();
        p.tries.lock_taken();
        p.tests.lock_taken();
        p.funs.lock_taken();
        p.classes.lock_taken();
        p.quotes.lock_taken();
        p.quote_pats.lock_taken();
        self.arenas_alloc_shared(true);
    }

    /// The loader's lock is released: what this hold made becomes every worker's, in the
    /// order the readers need it. The staged records first, then the shared entries of the
    /// maps that name them, then the working copies of the published records, which carry
    /// the cells' `Done` states: a worker that sees a cell done finds the map entry and the
    /// records it names. Of the maps, the ones a reader enters by (a function's or a val's
    /// body, a class's `TClass`, a class checked) go last, since a reader may enter there
    /// without the cell (`Interp::fun_of`) and then reads the others by what it finds: a body's
    /// templates, its quotes, the names it calls by.
    fn lock_released(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.check_publications();
            // What this worker made so far escapes with what the release publishes: marked before
            // any of it is published, so that a peer acquiring an entry point sees the mark
            // (`SharedArena::escaped`).
            self.syms.escape_own();
            self.prog.escape_own();
            self.seal_released();
        }
        self.swap_view_memos();
        self.arenas_alloc_shared(false);
        let s = &mut self.syms;
        s.syms.publish_staged();
        s.classes.publish_staged();
        s.tparams.publish_staged();
        s.aliases.publish_staged();
        s.pkgs.publish_staged();
        s.overloads.publish_staged();
        let p = &mut self.prog;
        p.exprs.publish_staged();
        p.pats.publish_staged();
        p.strings.publish_staged();
        p.expr_lists.publish_staged();
        p.pat_lists.publish_staged();
        p.sym_lists.publish_staged();
        p.stmts.publish_staged();
        p.cases.publish_staged();
        p.tries.publish_staged();
        p.tests.publish_staged();
        p.funs.publish_staged();
        p.classes.publish_staged();
        p.quotes.publish_staged();
        p.quote_pats.publish_staged();
        p.file_tags.publish_staged();
        self.file_pkgs.publish_staged();
        self.file_imports.publish_staged();
        self.file_opaques.publish_staged();
        crate::shake::point(crate::shake::Point::Staged);
        self.class_imports.apply_pending();
        self.retained_bodies.apply_pending();
        self.inline_definitions.apply_pending();
        self.any_members.apply_pending();
        self.outer_accessors.apply_pending();
        self.merged_overloads.apply_pending();
        self.derived_opaques.apply_pending();
        self.arity_classes.apply_pending();
        self.prog.template_syms.apply_pending();
        self.prog.overrides.apply_pending();
        self.inferred_parent_args.apply_pending();
        self.inferred_trait_args.apply_pending();
        self.unchecked_variance.apply_pending();
        self.outer_this.apply_pending();
        self.quote.apply_pending();
        self.derive.apply_pending();
        self.syms.dispatch_names.apply_pending();
        self.fun_of_sym.apply_pending();
        self.val_init.apply_pending();
        self.prog.class_bodies.apply_pending();
        self.class_done.apply_pending();
        // The records without cells first, then the ones whose cells readers key on: a
        // reader that sees a symbol done finds its class and the program's records new.
        let p = &mut self.prog;
        p.exprs.flush();
        p.pats.flush();
        p.strings.flush();
        p.expr_lists.flush();
        p.pat_lists.flush();
        p.sym_lists.flush();
        p.stmts.flush();
        p.cases.flush();
        p.tries.flush();
        p.tests.flush();
        p.funs.flush();
        p.classes.flush();
        p.quotes.flush();
        p.quote_pats.flush();
        p.file_tags.flush();
        self.file_pkgs.flush();
        self.file_imports.flush();
        self.file_opaques.flush();
        let s = &mut self.syms;
        s.tparams.flush();
        s.pkgs.flush();
        s.overloads.flush();
        s.aliases.flush();
        s.classes.flush();
        s.syms.flush();
        crate::shake::point(crate::shake::Point::Flushed);
        crate::shared::flush_staged_done();
    }

    /// With the overlays on, the memos a worker keeps outside the loader's lock and the ones its
    /// holder keeps under it are apart: a memo's entry holds types in
    /// the view it was made in (a worker's overlay types outside the lock, the base's under it), so
    /// the outermost hold swaps the holder's set in and its release swaps it out again. Each view
    /// makes its own entries, structurally the same answers in its own types.
    fn swap_view_memos(&mut self) {
        if !self.types.overlays_on() {
            return;
        }
        let other = &mut self.other_view_memos;
        self.matches.swap_memos(&mut other.matches);
        std::mem::swap(&mut self.given_fast, &mut other.given_fast);
    }

    /// What the release is about to make every worker's, checked to name the base alone
    /// (`view::published`, the assertion-enabled builds'): the staged records and working copies
    /// of the symbols' tables and the variance registrations, whatever wrote them, so that a
    /// publication a site did not export is caught before its visibility boundary.
    #[cfg(debug_assertions)]
    fn check_publications(&self) {
        if self.types.view_here() != crate::types::View::Hold {
            return;
        }
        self.syms.each_unpublished_type(&mut |what, t| crate::types::view::published(&self.types, what, t));
        for (_, &t) in self.unchecked_variance.pending() {
            crate::types::view::published(&self.types, "a variance registration (unchecked_variance)", t);
        }
    }

    /// Makes the arenas' records so far the shared region (the signature phase's prefix): the
    /// body phase's workers read it by reference and push their own records apart from it.
    /// With `attaching`, other workers will attach: the tables they share are kept as the fork
    /// leaves them (`FileMaps::snapshot`, `ExportTables::fork`).
    pub fn fork(&mut self, attaching: bool) {
        let s = &mut self.syms;
        s.syms.fork();
        s.classes.fork();
        s.tparams.fork();
        s.aliases.fork();
        s.pkgs.fork();
        s.overloads.fork();
        self.prog.fork();
        self.fun_of_sym.fork();
        self.val_init.fork();
        self.class_done.fork();
        self.class_imports.fork();
        self.retained_bodies.fork();
        self.inline_definitions.fork();
        self.any_members.fork();
        self.outer_accessors.fork();
        self.merged_overloads.fork();
        self.derived_opaques.fork();
        self.derive.fork();
        self.syms.dispatch_names.fork();
        self.arity_classes.fork();
        self.prog.template_syms.fork();
        self.prog.class_bodies.fork();
        self.prog.top_funs.fork();
        self.prog.top_vals.fork();
        self.prog.overrides.fork();
        self.inferred_parent_args.fork();
        self.inferred_trait_args.fork();
        self.unchecked_variance.fork();
        self.js_registry = Some(std::sync::Arc::new(JsRegistry::from_vec(&self.prog.js_imports)));
        self.js_exported.store(!self.prog.js_exports.is_empty(), std::sync::atomic::Ordering::Release);
        self.walk.share();
        self.def_syms.fork(attaching);
        self.def_classes.fork(attaching);
        self.def_aliases.fork(attaching);
        self.file_pkgs.fork();
        self.file_imports.fork();
        self.file_opaques.fork();
        self.types_at_fork = self.types.len().0 as u32;
        self.tvars_at_fork = self.tvars.len() as u32;
        self.lists_at_fork = self.local_news.len();
        self.outer_this.fork();
        self.quote.fork();
        if attaching {
            self.class_exports.fork();
            self.pkg_exports.fork();
        }
        // The type store and the interner take every worker's inserts: serialised from here
        // to the merge (`types.rs`, `intern.rs`).
        self.types.set_exclusive(false);
        self.interner.set_exclusive(false);
        self.forked = true;
    }

    /// Gives the build to one worker: an outcome met here depends on the order the bodies are
    /// typed in (a cycle through two workers' cells, whose diagnostic names the member one
    /// worker's walk would have met first; literal types turned on), which one worker's build
    /// decides as scalac's order does. The bodies before the fork (the prefix) are typed in an
    /// order of their own too; the signature phase is the same at every worker count. The
    /// workers stop taking items, and the build is typed again from the start by one worker.
    pub fn need_serial(&self, why: impl FnOnce() -> String) {
        if self.bodies_started {
            self.need_serial_for_state(why, None);
        }
    }

    /// Gives the build to one worker for two different diagnostics at one place, whose order the
    /// workers' schedule decided (`diagnostics_meeting`). Asked once the workers are joined.
    pub fn need_serial_for_diagnostics(&self, why: impl FnOnce() -> String) {
        if self.bodies_started && self.walk_by_size && !self.serial.is_needed() {
            self.serial.diagnostics.store(true, std::sync::atomic::Ordering::Release);
            self.need_serial(why);
        }
    }

    /// Gives the build to one worker for a macro's run, or a constant's folding, that changed
    /// state other runs share: at any point of a build with several workers, since before the
    /// fork the change is in the first worker's heap alone.
    pub fn need_serial_for_state(&self, why: impl FnOnce() -> String, module: Option<(std::rc::Rc<str>, ClassId)>) {
        if !self.walk_by_size || self.serial.is_needed() {
            return;
        }
        let mut reason = self.serial.reason.lock().unwrap_or_else(|e| e.into_inner());
        if reason.is_none() {
            *reason = Some((why(), module.map(|(name, c)| (name.to_string(), c))));
            crate::measure::gave_way();
        }
        self.serial.needed.store(true, std::sync::atomic::Ordering::Release);
    }

    /// A macro's run or a constant's folding changed state other runs share (`need_serial_for_state`),
    /// unless `--macro-state per-worker` keeps it to the worker, or the diagnostic
    /// `TEQ_STATE_GIVEWAY=off` lets it through; either counts it by its reason. The recursive
    /// signature's fallback (`need_serial`) stays whichever. `module` is the undeclared module whose
    /// state it is, by name and class.
    pub fn shared_state_changed(&self, why: impl FnOnce() -> String, module: Option<(std::rc::Rc<str>, ClassId)>) {
        if self.macro_state_per_worker || crate::measure::state_giveway_off() {
            if self.walk_by_size {
                crate::measure::state_bypassed(why());
            }
            return;
        }
        self.need_serial_for_state(why, module);
    }

    /// Whether `f` is a file of the program: its definitions' lazy work (a signature, a body,
    /// a class check) is done outside the loader's lock, into the doing worker's chunk; the
    /// std's and the libraries' (a jar's entries, the pseudo files of their bodies) under it,
    /// into the shared region.
    pub fn program_file(&self, f: FileId) -> bool {
        !self.source(f).is_std && !self.in_jar(f)
    }

    /// Whether `f` is a file of teq's own standard library, `std/`: neither the program's nor a
    /// library's, whose pseudo files of bodies are marked std too.
    pub fn std_source(&self, f: FileId) -> bool {
        self.files.as_slice().get(f.0 as usize).map_or(false, |s| s.is_std) && !self.in_jar(f)
    }

    /// Whether `sym` is a top-level definition of a product's `<file>$package`.
    pub fn is_product_top_def(&self, sym: SymId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.product_package_members.contains_key(&sym))
    }

    /// Changes the record of `sym` through `f`: under the loader's lock when the record is the
    /// shared region's, where the change is a copy published when the hold ends.
    pub fn shared_write<R>(&mut self, sym: SymId, f: impl FnOnce(&mut Self) -> R) -> R {
        if self.shared_sym(sym) {
            self.with_loader(f)
        } else {
            f(self)
        }
    }

    /// Whether `c` is a record of the shared region, whose change needs the loader's lock.
    #[inline]
    pub fn shared_class(&self, c: ClassId) -> bool {
        self.forked && c.0 < crate::arena::LOCAL_BASE
    }

    #[inline]
    pub fn shared_sym(&self, s: SymId) -> bool {
        self.forked && s.0 < crate::arena::LOCAL_BASE
    }

    /// Whether `f` is a jar of the classpath rather than a source.
    #[inline]
    pub fn in_jar(&self, f: FileId) -> bool {
        self.loaded.as_ref().map_or(false, |l| l.is_jar(f))
    }

    /// Whether `f` is the pseudo file of a directory of teq's products: another module's program.
    pub fn in_products(&self, f: FileId) -> bool {
        self.loaded.as_ref().is_some_and(|l| l.jar_files.iter().position(|&j| j == f).is_some_and(|i| l.cp.is_products_entry(i)))
    }

    /// Whether the object of the block's top-level definitions is the members of a `package
    /// object` without parents, which zinc knows by scalac's `p.package`; with parents, the
    /// `object package` they make is `p.package` and the members' object keeps its file's name.
    pub fn package_object_holder(&self, f: FileId) -> bool {
        let ast = self.ast(f);
        ast.package_object && !ast.top_level.iter().any(|&d| ast.def(d).name == crate::names::PACKAGE && matches!(ast.def(d).kind, crate::ast::DefKind::Class(_)))
    }

    /// Marks what the loader finished as done for the lookups outside its lock: at the end of
    /// the outermost hold, once what it made is published (`with_loader`), or at once before
    /// the fork.
    pub fn loader_done(&mut self, done: LoaderDone) {
        if self.forked {
            self.loader_done_pending.push(done);
        } else if let Some(loaded) = &self.loaded {
            loaded.mark_done(done);
        }
    }

    /// The std table for a change: before the fork, or under the loader's lock after it.
    pub fn std_mut(&mut self) -> &mut stdlib::StdFiles {
        debug_assert!(!self.forked || crate::shared::lock_depth() > 0, "the std table changed outside the loader's lock");
        unsafe { &mut *self.std.0.get() }
    }

    /// The classpath's tables for a change: the loader's lock holder's once the arenas are
    /// forked.
    pub fn loaded_mut(&mut self) -> &mut loader::Loaded {
        debug_assert!(!self.forked || crate::shared::lock_depth() > 0, "the loader's tables changed outside its lock");
        let cell = self.loaded.as_ref().expect("no classpath");
        unsafe { &mut *cell.0.get() }
    }

    pub fn set_jvm(&mut self) {
        self.jvm = true;
        self.prog.record_types = true;
    }

    pub fn set_main_name(&mut self, name: &str) {
        self.main_name = Some(self.interner.intern(name));
    }

    #[inline]
    pub fn ast(&self, file: FileId) -> &'a Ast {
        let asts: &'a Asts = self.asts;
        &asts[file.0 as usize]
    }

    #[inline]
    pub fn cur_ast(&self) -> &'a Ast {
        self.ast(self.env.file)
    }

    /// Whether the file being typed is a converted library body's, the only kind whose AST holds
    /// the types scalac inferred (`Ast::inferred_types`), whichever worker converted it.
    #[inline]
    pub(super) fn in_converted_body(&self) -> bool {
        self.is_body_file(self.env.file)
    }

    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        let msg = msg.into();
        if self.dependent_checks > 0 {
            self.diags.dependent_error(self.env.file, span, msg);
            return;
        }
        if self.is_body_file(self.env.file) {
            let msg = self.library_body_error(span, msg);
            let place = self.body_error_place(span);
            self.diags.error_placed(self.env.file, span, msg, place);
            return;
        }
        self.diags.error(self.env.file, span, msg);
    }

    /// The place of the call at the expression `e` of a library body, which a diagnostic moved
    /// there from its expansion stands at.
    #[cold]
    pub(super) fn call_place(&mut self, file: FileId, e: crate::ast::ExprId) -> Option<crate::source::Place> {
        let place = self.expr_place(file, e).filter(|p| p.pos.is_some())?;
        let (line, col) = place.line_col.map_or((None, None), |(l, c)| (Some(l), Some(c)));
        Some(crate::source::Place { source: place.source(), line, col, line_text: place.line_text })
    }

    /// Where an error of a library body stands in the library's source: the selection or the
    /// operator being typed, at its name's point as scalac reports a missing member, else the
    /// member the span is the line of; the pseudo file's line where neither has a position.
    #[cold]
    fn body_error_place(&mut self, span: Span) -> Option<crate::source::Place> {
        let file = self.env.file;
        let place = match self.body_node {
            Some(n) if n.file == file => self.expr_place(file, n.selection),
            _ => None,
        };
        let place = match place {
            Some(p) if p.pos.is_some() => p,
            _ => self.member_place(file, span).filter(|p| p.pos.is_some())?,
        };
        let (line, col) = place.line_col.map_or((None, None), |(l, c)| (Some(l), Some(c)));
        Some(crate::source::Place { source: place.source(), line, col, line_text: place.line_text })
    }

    /// An error about the types `about`, marked `unknown` where one of them holds the error type,
    /// which stands for what malformed syntax left unknown: a program with a syntax error does
    /// not present it (`Diagnostic::unknown`).
    pub(super) fn error_unless_unknown(&mut self, span: Span, msg: String, about: &[TypeId]) {
        let unknown = self.recovered && about.iter().any(|&t| self.types.contains_error(t));
        self.error(span, msg);
        if unknown {
            if let Some(d) = self.diags.items.last_mut() {
                d.unknown = true;
            }
        }
    }

    /// That no given of the type `wanted` was found: dependent where the type names a class whose
    /// header, or an ancestor's, the parser could not complete (its implicit scope is unknown),
    /// or holds the error type in a program with a syntax error.
    pub(super) fn given_failure_error(&mut self, span: Span, msg: String, wanted: TypeId) {
        let mut classes = Vec::new();
        self.classes_of_type(wanted, &mut classes);
        let incomplete = classes.into_iter().any(|c| {
            let t = self.syms.class(c).base_types.first().map(|&(_, t)| t);
            t.is_some_and(|t| self.incomplete_ancestry(t))
        });
        if incomplete {
            self.dependent_error(span, msg);
        } else {
            self.error_unless_unknown(span, msg, &[wanted]);
        }
    }

    /// That `name` is not found: dependent where an import in scope whose path did not resolve
    /// could bind it (`ImportTarget::Unresolved`, a wildcard but for the names its clause leaves
    /// out), as scalac resolves it through the import to its error.
    pub(super) fn not_found_error(&mut self, name: crate::intern::Name, span: Span, msg: String) {
        let file_imports = self.file_imports[self.env.file.0 as usize].as_ref().map_or(0, |l| l.len());
        let unknown = (0..self.env.imports.len() + file_imports).any(|i| {
            let imp = self.import_at(i);
            matches!(imp.target, ImportTarget::Unresolved) && imp.name.map_or_else(|| !self.import_hides(imp, name), |n| n == name)
        });
        if unknown {
            self.dependent_error(span, msg);
        } else {
            self.error(span, msg);
        }
    }

    /// Where the failures of a typing stand: the errors reported, presented or dependent, and the
    /// error nodes typed. Whether a typing failed is read from these facts and never from what
    /// is presented, so that a suppression changes no decision.
    pub(super) fn failure_mark(&self) -> (usize, u32) {
        (self.diags.items.len(), self.error_nodes)
    }

    pub(super) fn failed_since(&self, mark: (usize, u32)) -> bool {
        self.error_nodes > mark.1 || self.diags.items.get(mark.0..).is_some_and(|items| items.iter().any(|d| !d.is_warning))
    }

    /// Whether the class of `t`, or one of its base classes, is one whose header the parser
    /// could not complete.
    pub(super) fn incomplete_ancestry(&mut self, t: TypeId) -> bool {
        let t = self.dealias(t);
        let Some(c) = self.class_of(t) else { return false };
        self.complete_class(c);
        let info = self.syms.class(c);
        info.mods & crate::ast::mods::INCOMPLETE != 0
            || info.base_types.iter().any(|&(b, _)| self.syms.class(b).mods & crate::ast::mods::INCOMPLETE != 0)
    }

    /// An error that depends on what malformed syntax left unknown: recorded, never presented
    /// (`Diagnostic::dependent`).
    pub fn dependent_error(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.dependent_error(self.env.file, span, msg);
    }

    /// An error inside a library body names the definition it stands in; a member the std's
    /// class lacks is a miss of the census.
    fn library_body_error(&mut self, span: Span, msg: String) -> String {
        let file = self.env.file;
        let path = self.source(file).path.clone();
        let class = path.rsplit('!').next().unwrap_or(&path).to_string();
        let member = crate::source::locate(&self.source(file).text, span.start as usize).2.to_string();
        let member = member.rsplit(' ').next().unwrap_or(&member).to_string();
        let needed_by = if member.is_empty() { class } else { format!("{}.{}", class, member) };
        let body = self.library_body_text(file, span);
        if let Some((m, t)) = msg.split_once(" is not a member of ") {
            let (t, _) = t.split_once('\n').unwrap_or((t, ""));
            let miss = format!("{} of {}", m, t);
            if self.loaded.is_some() {
                self.with_loader(|w| {
                    let loaded = w.loaded_mut();
                    match loaded.bodies.misses.iter_mut().find(|(d, _)| *d == miss) {
                        Some((_, n)) => *n += 1,
                        None => loaded.bodies.misses.push((miss.clone(), 1)),
                    }
                });
            }
            return format!("not supported yet: {} (needed by {}){}", miss, needed_by, body);
        }
        format!("{} (in the body of {}){}", msg, needed_by, body)
    }

    /// Drops the diagnostics from `from` on, with the misses of the census they counted, and the
    /// promoted ranges' positions past `from` (`state::diags_cut_at`).
    pub(super) fn discard_diagnostics(&mut self, from: usize) {
        self.diags_cut_at(from);
        self.discard_counted(from);
    }

    /// `discard_diagnostics` leaving the promoted ranges to the caller: a retraction that keeps
    /// promoted work relocates them itself.
    pub(super) fn discard_counted(&mut self, from: usize) {
        let dropped: Vec<crate::source::Diagnostic> = self.diags.items.drain(from..).collect();
        for d in dropped {
            let Some(miss) = d.msg.strip_prefix("not supported yet: ").and_then(|m| m.split_once(" (needed by ")).map(|(m, _)| m) else { continue };
            if self.loaded.is_none() {
                continue;
            }
            self.with_loader(|w| {
                let loaded = w.loaded_mut();
                if let Some(i) = loaded.bodies.misses.iter().position(|(m, _)| m == miss) {
                    loaded.bodies.misses[i].1 -= 1;
                    if loaded.bodies.misses[i].1 == 0 {
                        loaded.bodies.misses.remove(i);
                    }
                }
            });
        }
    }

    /// The body of the library member whose definition spans `span` in the pseudo file `file`,
    /// printed from its TASTy as the diagnostics' context; empty when nothing is known.
    fn library_body_text(&mut self, file: FileId, span: Span) -> String {
        let Some(sym) = self.def_syms.entries_in(file.0 as usize).into_iter().find(|(d, _)| self.ast(file).def(*d).span == span).map(|(_, s)| s) else { return String::new() };
        let Some(ls) = self.loaded.as_ref().and_then(|l| l.syms.get(&sym).copied()) else { return String::new() };
        let tasty = self.loaded.as_ref().unwrap().file(ls.file).tasty.clone();
        let mut decoder = crate::tasty::tree::Decoder::new(&tasty);
        let mut terms = crate::tasty::terms::TermDecoder::new(&mut decoder);
        let (_, rhs) = terms.def_with_body(ls.addr);
        let Some(rhs) = rhs else { return String::new() };
        let text = crate::tasty::show::Printer::new(&tasty, false).term(&rhs, 2);
        let mut shown: String = text.chars().take(600).collect();
        if shown.len() < text.len() {
            shown.push_str(" ...");
        }
        format!("\n  body: {}", shown)
    }

    /// Warnings concern the program, not the std it is compiled with or the library bodies
    /// scalac checked.
    pub fn warn(&mut self, span: Span, msg: impl Into<String>) {
        if self.nowarn == 0 && !self.source(self.env.file).is_std && !self.is_body_file(self.env.file) {
            if self.dependent_checks > 0 {
                self.diags.dependent_warn(self.env.file, span, msg);
            } else {
                self.diags.warn(self.env.file, span, msg);
            }
        }
    }

    /// The source of a file: one of the program's, or the pseudo file of a library body.
    pub fn source(&self, f: FileId) -> &crate::source::SourceFile {
        let n = self.files.len();
        if (f.0 as usize) < n {
            &self.files[f.0 as usize]
        } else {
            &self.loaded.as_ref().unwrap().body_sources[f.0 as usize - n]
        }
    }

    /// A pseudo file for a library body or class converted into an AST, in package `pkg`, read
    /// from the jar of the pseudo file `jar`.
    pub fn new_body_file(&mut self, jar: FileId, path: String, key: String, ast: Ast, text: String, pkg: PkgId) -> FileId {
        self.new_body_file_tagged(jar, path, key, ast, text, pkg, None)
    }

    /// `new_body_file`, the file's tag the token `recorded` where one is given: a product's file
    /// of top-level definitions, whose initialiser is named by its source's token.
    pub fn new_body_file_tagged(&mut self, jar: FileId, path: String, key: String, ast: Ast, text: String, pkg: PkgId, recorded: Option<u64>) -> FileId {
        let replays = ast.reader.as_ref().map_or(false, |r| !r.replay.is_empty());
        let id = FileId(self.asts.push(ast) as u32);
        #[cfg(debug_assertions)]
        loader::compile::body_crossings::converted(id, self.worker);
        if replays {
            self.loaded_mut().replay_files.insert(id, ());
        }
        match recorded {
            Some(token) => self.prog.push_recorded_tag(token),
            None => self.prog.push_file_tag(&key),
        }
        self.file_pkgs.push(pkg);
        self.file_imports.push(Some(std::sync::Arc::new(Vec::new())));
        self.file_opaques.push(Vec::new());
        self.file_chains.push(None);
        self.loaded_mut().add_body_file(id, jar, path, key, text);
        id
    }

    /// Whether `f`, a jar's pseudo file or one of a body converted from a jar, comes from an
    /// artifact of the standard library (`classpath::is_compiler_library`).
    pub fn from_compiler_library(&self, f: FileId) -> bool {
        let n = self.files.len();
        let mut jar = f;
        while jar.0 as usize >= n {
            let origin = self.loaded.as_ref().and_then(|l| {
                let i = jar.0 as usize - n;
                (i < l.body_jars.len()).then(|| *l.body_jars.get(i))
            });
            match origin {
                Some(origin) if origin != jar => jar = origin,
                _ => return false,
            }
        }
        crate::classpath::is_compiler_library(&self.files[jar.0 as usize].path)
    }

    /// The program's sources followed by the pseudo files of the library bodies, for the
    /// diagnostics renderer.
    pub fn all_sources(&self) -> Vec<crate::source::SourceFile> {
        let mut out: Vec<crate::source::SourceFile> = self.files.as_slice().iter().map(crate::source::SourceFile::copy).collect();
        if let Some(loaded) = &self.loaded {
            out.extend(loaded.body_sources.as_slice().iter().map(crate::source::SourceFile::copy));
        }
        out
    }

    /// Gives the pseudo files of library bodies their place in the canonical order: after the
    /// program's files, by key (`Symbols::file_ranks`), since they are made in the order the
    /// bodies are demanded, which is not one across builds. The sources of the products'
    /// definitions stand among the program's files by their keys, each
    /// before the first program file of a greater key, so that a program built whole and built
    /// over its products order their definitions alike.
    pub fn rank_files(&mut self) {
        let n = self.files.len();
        let Some(loaded) = &self.loaded else {
            self.syms.file_ranks = (0..n as u32).collect();
            return;
        };
        // The products' sources merged with the program's files by key, and their package
        // blocks (a package object's, `<file>#<n>` as `frontend::lay_out` keys it) with the
        // program's package blocks, which follow every file.
        let block = |key: &str| key.contains('#');
        // A block's key in its file's order: `A.scala#10` after `A.scala#2`.
        let order = |key: &str| -> (String, u32) {
            match key.rsplit_once('#') {
                Some((file, n)) => (file.to_string(), n.parse().unwrap_or(u32::MAX)),
                None => (key.to_string(), 0),
            }
        };
        let mut sources: Vec<(&str, usize)> = loaded.product_sources.as_slice().iter().enumerate().filter(|(_, (key, _))| !block(key)).map(|(i, (key, _))| (key.as_str(), i)).collect();
        let mut block_sources: Vec<(&str, usize)> = loaded.product_sources.as_slice().iter().enumerate().filter(|(_, (key, _))| block(key)).map(|(i, (key, _))| (key.as_str(), i)).collect();
        sources.sort();
        block_sources.sort_by_key(|&(key, i)| (order(key), i));
        let mut source_ranks = vec![0u32; loaded.product_sources.len()];
        let mut ranks: Vec<u32> = vec![0; n];
        let mut next = 0u32;
        let mut pending = sources.iter().peekable();
        let mut pending_blocks = block_sources.iter().peekable();
        let last_program = self.files.as_slice().iter().rposition(|f| !f.is_std && !block(&f.key));
        let last_block = self.files.as_slice().iter().rposition(|f| !f.is_std && block(&f.key));
        for (i, f) in self.files.as_slice().iter().enumerate() {
            if !f.is_std {
                let is_block = block(&f.key);
                let queue = if is_block { &mut pending_blocks } else { &mut pending };
                while let Some(&&(_, s)) = queue.peek().filter(|(key, _)| if is_block { order(key) < order(&f.key) } else { *key < f.key.as_str() }) {
                    source_ranks[s] = next;
                    next += 1;
                    queue.next();
                }
            }
            ranks[i] = next;
            next += 1;
            if Some(i) == last_program {
                for &(_, s) in pending.by_ref() {
                    source_ranks[s] = next;
                    next += 1;
                }
            }
            if Some(i) == last_block {
                for &(_, s) in pending_blocks.by_ref() {
                    source_ranks[s] = next;
                    next += 1;
                }
            }
        }
        for &(_, s) in pending.chain(pending_blocks) {
            source_ranks[s] = next;
            next += 1;
        }
        let mut pseudo: Vec<(&str, usize)> = loaded.body_sources.as_slice().iter().enumerate().map(|(i, s)| (s.key.as_str(), i)).collect();
        pseudo.sort();
        ranks.resize(n + pseudo.len(), 0);
        for (rank, (_, i)) in pseudo.into_iter().enumerate() {
            ranks[n + i] = next + rank as u32;
        }
        for (&f, &source) in &loaded.product_files {
            if let Some(r) = ranks.get_mut(f.0 as usize) {
                *r = source_ranks[source as usize];
            }
        }
        self.syms.product_places = loaded.product_places.iter().map(|(&k, p)| (k, (p.source, p.offset))).collect();
        self.syms.product_sources = loaded.product_sources.as_slice().iter().zip(&source_ranks).map(|((key, token), &rank)| (rank, *token, key.clone())).collect();
        self.syms.product_files = loaded.product_files.clone();
        self.syms.product_callee_sources = loaded.product_callee_sources.clone();
        self.syms.product_synthetics = loaded.product_synthetics.clone();
        self.syms.product_class_places = loaded.product_class_places.clone();
        for &(s, c) in &loaded.product_mirror_vals {
            let f = self.syms.class(c).file;
            if loaded.product_files.contains_key(&f) {
                self.syms.sym_mut(s).file = f;
            }
        }
        self.syms.file_ranks = ranks;
    }

    /// Prints the info messages the accepted attempt's runs gave, in the canonical order of
    /// their expansions (the files' ranks, then the positions, the message's number within its
    /// run and its text), and prints the ones after it as they come.
    pub fn flush_infos(&mut self) {
        let Some(mut infos) = self.infos.take() else { return };
        if infos.is_empty() {
            return;
        }
        self.rank_files();
        let ranks = &self.syms.file_ranks;
        let key = |m: &InfoMessage| -> (Vec<(u32, u32, u32)>, u32) {
            (m.path.iter().map(|&(f, a, b)| (ranks.get(f.0 as usize).copied().unwrap_or(f.0), a, b)).collect(), m.ordinal)
        };
        infos.sort_by(|a, b| key(a).cmp(&key(b)).then_with(|| a.text.cmp(&b.text)));
        let mut out = String::new();
        for m in infos {
            out.push_str("info: ");
            out.push_str(&m.text);
            out.push('\n');
        }
        eprint!("{}", out);
    }

    /// The diagnostics rendered in the canonical order of the files.
    pub fn render_diags(&mut self) -> String {
        self.rank_files();
        self.diags.render_ranked(&self.all_sources(), &self.syms.file_ranks)
    }

    /// Owned so that messages can be built while the typer is borrowed mutably.
    pub fn name_str(&self, n: Name) -> String {
        self.interner.get(n).to_string()
    }

    pub fn name_ref(&self, n: Name) -> &str {
        self.interner.get(n)
    }

    /// The signature phase: the syntax scanned, every definition entered, the std classes
    /// found, the classes and aliases of the program completed, then the tables of the
    /// program's scopes the bodies read (`build_scope_tables`). What the bodies then demand of
    /// signatures is still completed on the way (`sig_of`); the phase is what the parallel
    /// typer keeps on the main thread.
    pub fn signature_phase(&mut self) {
        let wildcards = self.scan_wildcards();
        self.wildcards_used.store(wildcards, std::sync::atomic::Ordering::Release);
        let p = self.phase(profile::Phase::Enter);
        self.enter_all();
        self.phase_end(p);
        self.entering_done = true;
        let p = self.phase(profile::Phase::StdClasses);
        // Under `--std=scala-library` the builtin `Array` meets `scala.Array` for what the
        // builtin layer's extensions (`length`, `apply`, `update`) leave: its companion and
        // base types.
        if self.scala_library_std() {
            self.link_library_class(self.b.array);
        }
        self.find_std_classes();
        self.phase_end(p);
        self.complete_program();
        self.build_scope_tables();
    }

    /// Whether the files name a wildcard type argument anywhere.
    pub fn scan_wildcards(&self) -> bool {
        (0..self.asts.len()).any(|i| self.ast(FileId(i as u32)).tys.iter().any(|t| matches!(t, crate::ast::TyExpr::Wildcard | crate::ast::TyExpr::BoundedWildcard(..))))
    }

    /// The std classes the typer refers to by field, among the std files entered so far;
    /// run again whenever a std file enters later (`refresh_std_classes`). Nothing here enters
    /// a file: what the typer needs without a name in the program asks through the accessors
    /// below (`seq_class`, `throwable_class`, ...).
    fn find_std_classes(&mut self) {
        self.b.seq = self.entered_std_class("Seq");
        self.b.string_context = self.entered_std_class("StringContext");
        self.b.partial_function = self.entered_std_class("PartialFunction");
        self.b.conversion = self.entered_std_class("Conversion");
        self.prog.partial_function = self.b.partial_function;
        self.b.sub_evidence = self.entered_std_class("<:<");
        self.b.eq_evidence = self.entered_std_class("=:=");
        self.b.option = self.entered_std_class("Option");
        self.b.named_tuple = self.find_named_tuple();
        self.b.either = self.entered_std_class("Either");
        self.b.set = self.entered_std_class("Set");
        self.b.map = self.entered_std_class("Map");
        self.b.can_equal = self.entered_std_class("CanEqual");
        self.b.value_of = self.entered_std_class("ValueOf");
        self.b.selectable = self.entered_std_class("Selectable");
        self.b.number = self.entered_java_lang_class("Number");
        self.b.js_dynamic = self.scalajs_class(crate::names::DYNAMIC);
        self.b.throwable = self.entered_java_lang_class("Throwable");
        self.b.js_exception = self.entered_js_class("JavaScriptException");
        self.prog.throwable = self.b.throwable;
        self.prog.js_exception = self.b.js_exception;
        self.b.cons_tuple = self.entered_std_class("*:");
        self.b.product = self.entered_std_class("Product");
        self.b.t_product = self.b.product.map_or(ERROR, |p| self.types.class(p, &[]));
        self.note_singleton_class();
        self.b.t_equals = self.entered_std_class("Equals").map_or(ERROR, |e| self.types.class(e, &[]));
        self.b.reflect_enum = self.entered_scala_reflect_class("Enum");
        self.b.t_enum = self.b.reflect_enum.map_or(ERROR, |e| self.types.class(e, &[]));
        self.b.tuple_members = crate::typer::arity::TUPLE_MEMBERS.iter().map(|n| self.interner.intern(n)).collect();
        self.b.tuple_member_bits = [0; 4];
        for n in self.b.tuple_members.clone() {
            self.b.tuple_member_bits[(n.0 / 64 % 4) as usize] |= 1 << (n.0 % 64);
        }
        self.b.predef_names = crate::stdindex::STD_INDEX.iter().flat_map(|f| f.predef).map(|n| self.interner.intern(n)).collect();
        self.b.product_helpers = [("productIterator", "productIteratorImpl"), ("productElementNames", "productElementNamesImpl"), ("canEqual", "canEqualImpl")]
            .iter()
            .filter_map(|&(member, helper)| Some((self.interner.intern(member), self.entered_std_def(helper)?)))
            .collect();
        self.b.tuple_trait = self.entered_std_class("Tuple");
        self.b.non_empty_tuple = self.entered_std_class("NonEmptyTuple");
        self.b.empty_tuple = self.empty_tuple_class();
        self.mark_reducible_classes();
        self.find_site_classes();
        self.find_mirror_classes();
    }

    pub fn refresh_std_classes(&mut self) {
        let p = self.phase(profile::Phase::StdClasses);
        self.find_std_classes();
        self.phase_end(p);
    }

    /// Acts on the std files entered late by any worker since this worker last looked: the
    /// memos of the packages they touched go, and the std classes are found again.
    pub fn refresh_std_if_stale(&mut self) {
        let n = self.std.late_pkgs.len();
        if n <= self.std_seen {
            return;
        }
        for i in self.std_seen..n {
            let p = *self.std.late_pkgs.get(i);
            self.given_indexes.remove(&p);
            self.conversion_indexes.remove(&conversions::ScopeKey::Pkg(p));
            self.pkg_exports.remove(&p);
        }
        self.std_seen = n;
        self.refresh_std_classes();
    }

    /// The std class a construct of the language needs whether or not the program names it:
    /// each accessor enters the class's std file on the first ask; a file another worker entered
    /// first is found, and the std's classes are found again in this worker's table, which the
    /// entering worker's finding (`find_std_classes`) did not fill.
    pub fn seq_class(&mut self) -> Option<ClassId> {
        if self.b.seq.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.seq.is_none() && self.std_class("Seq").is_some() && self.b.seq.is_none() {
            self.refresh_std_classes();
        }
        // A varargs literal is wrapped in the std's `ArraySeq`, which the backends ask for.
        if !self.array_seq_asked {
            self.array_seq_asked = true;
            self.std_class("ArraySeq");
        }
        self.b.seq
    }

    pub fn string_context_class(&mut self) -> Option<ClassId> {
        if self.b.string_context.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.string_context.is_none() && self.std_class("StringContext").is_some() && self.b.string_context.is_none() {
            self.refresh_std_classes();
        }
        self.b.string_context
    }

    /// `ValueOf`: the std's, or scala-library's in link mode, where the std leaves it out.
    pub fn value_of_class(&mut self) -> Option<ClassId> {
        if self.b.value_of.is_none() {
            self.b.value_of = self.std_class("ValueOf");
        }
        self.b.value_of
    }

    /// Whether `c` is the std's `PartialFunction`: in a worker also where another worker entered
    /// its file, whose finding of the std's classes this worker's table lacks until it finds them
    /// again (`refresh_std_classes`).
    pub fn is_partial_function(&mut self, c: ClassId) -> bool {
        if Some(c) == self.b.partial_function {
            return true;
        }
        if self.forked && self.b.partial_function.is_none() && self.name_ref(self.syms.class(c).name) == "PartialFunction" {
            self.refresh_std_classes();
        }
        Some(c) == self.b.partial_function
    }

    pub fn partial_function_class(&mut self) -> Option<ClassId> {
        if self.b.partial_function.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.partial_function.is_none() && self.std_class("PartialFunction").is_some() && self.b.partial_function.is_none() {
            self.refresh_std_classes();
        }
        self.b.partial_function
    }

    pub fn can_equal_class(&mut self) -> Option<ClassId> {
        if self.b.can_equal.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.can_equal.is_none() && self.std_class("CanEqual").is_some() && self.b.can_equal.is_none() {
            self.refresh_std_classes();
        }
        self.b.can_equal
    }

    pub fn throwable_class(&mut self) -> Option<ClassId> {
        if self.b.throwable.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.throwable.is_none() && self.java_lang_class("Throwable").is_some() && self.b.throwable.is_none() {
            self.refresh_std_classes();
        }
        self.b.throwable
    }

    pub fn js_exception_class(&mut self) -> Option<ClassId> {
        if self.b.js_exception.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.js_exception.is_none() && self.js_class("JavaScriptException").is_some() && self.b.js_exception.is_none() {
            self.refresh_std_classes();
        }
        self.b.js_exception
    }

    pub fn named_tuple_class(&mut self) -> Option<ClassId> {
        if self.b.named_tuple.is_none() {
            self.refresh_std_if_stale();
        }
        if self.b.named_tuple.is_none() {
            let name = self.interner.intern("NamedTuple");
            let scala = self.b.scala_pkg;
            self.demand_term(scala, name);
            // scala-library's object, which enters no std file.
            if self.b.named_tuple.is_none() {
                self.b.named_tuple = self.find_named_tuple();
            }
        }
        self.b.named_tuple
    }

    /// The package `scala.scalajs.js` of the Scala.js library, when it is compiled in.
    pub fn scalajs_pkg(&self) -> Option<PkgId> {
        let scalajs = self.syms.pkg(self.b.scala_pkg).entries.get(&crate::names::SCALAJS).and_then(|e| e.pkg)?;
        self.syms.pkg(scalajs).entries.get(&crate::names::JS).and_then(|e| e.pkg)
    }

    fn scalajs_class(&self, name: Name) -> Option<ClassId> {
        let js = self.scalajs_pkg()?;
        self.syms.pkg(js).entries.get(&name).and_then(|e| e.class)
    }

    pub fn java_lang_pkg(&self) -> Option<PkgId> {
        self.java_sub_pkg(crate::names::LANG)
    }

    pub fn java_util_pkg(&self) -> Option<PkgId> {
        self.java_sub_pkg(crate::names::UTIL)
    }

    fn java_sub_pkg(&self, name: Name) -> Option<PkgId> {
        let java = self.syms.pkg(ROOT_PKG).entries.get(&crate::names::JAVA).and_then(|e| e.pkg)?;
        self.syms.pkg(java).entries.get(&name).and_then(|e| e.pkg)
    }

    /// Whether the macro runs' changes of the state of object `c` are a cache's: `c` declared
    /// with `--cacheable-state`, or one of the libraries' objects teq knows to be caches.
    pub fn cacheable_object(&self, c: ClassId) -> bool {
        self.cacheable_state_classes.contains(&c) || self.known_cache_object(c)
    }

    /// Whether `c` is one of the libraries' objects teq knows to be caches (`KNOWN_CACHES`).
    pub fn known_cache_object(&self, c: ClassId) -> bool {
        self.known_caches && self.is_library_class(c) && KNOWN_CACHES.iter().any(|&(path, _)| self.class_path(c) == path)
    }

    /// Resolves the names of `cacheable_state` once the program's classes are entered: each names
    /// an object, top level or nested in objects, of the program, of a jar or of the std; a name
    /// that matches nothing, or matches a class, a trait or an object nested in a class, is an
    /// error of the build, which says what the name found.
    pub fn resolve_cacheable_state(&mut self) {
        self.cacheable_state_classes.clear();
        for name in self.cacheable_state.clone() {
            let msg = match self.object_by_path(&name) {
                PathFound::Object(c) => {
                    if !self.cacheable_state_classes.contains(&c) {
                        self.cacheable_state_classes.push(c);
                    }
                    continue;
                }
                PathFound::Type(ClassKind::Trait) => format!("--cacheable-state: {} is a trait; cacheable state names an object", name),
                PathFound::Type(_) => format!("--cacheable-state: {} is a class; cacheable state names an object", name),
                PathFound::NestedIn(kind) => {
                    let enclosing = if kind == ClassKind::Trait { "trait" } else { "class" };
                    format!("--cacheable-state: {} is an object nested in a {}, which has an instance per enclosing instance", name, enclosing)
                }
                PathFound::Nothing => format!("--cacheable-state {}: no such object", name),
            };
            self.diags.error(crate::source::NO_FILE, Span::default(), msg);
        }
    }

    /// Whether `--cacheable-state <path>` declares the module `c`: a path that names another class or
    /// none (a module whose name holds a dot of its own) is not one to suggest.
    pub fn declared_by(&mut self, path: &str, c: ClassId) -> bool {
        matches!(self.object_by_path(path), PathFound::Object(found) if found == c)
    }

    /// What a qualified path (`a.b.Outer.Inner`) names: packages first, then objects, classes
    /// and traits, each a member of the one before.
    fn object_by_path(&mut self, path: &str) -> PathFound {
        let mut pkg = ROOT_PKG;
        let mut owner: Option<ClassId> = None;
        let mut enclosing: Option<ClassKind> = None;
        let mut found = PathFound::Nothing;
        for segment in path.split('.') {
            if segment.is_empty() {
                return PathFound::Nothing;
            }
            let name = self.interner.intern(segment);
            let (object, class) = match owner {
                None => {
                    if let Some(sub) = self.demand_pkg(pkg, name) {
                        pkg = sub;
                        found = PathFound::Nothing;
                        continue;
                    }
                    let term = self.pkg_term(pkg, name);
                    let object = term.and_then(|t| t.sym()).and_then(|s| self.object_of(s));
                    let class = match term {
                        Some(TermRef::Class(c)) => Some(c),
                        _ => None,
                    };
                    (object, class)
                }
                Some(outer) => {
                    self.complete_class(outer);
                    let info = self.syms.class(outer);
                    let object = info.members.get(&name).copied();
                    let class = info.nested.get(&name).copied();
                    (object.and_then(|s| self.object_of(s)), class)
                }
            };
            found = match (object, class) {
                (Some(c), _) => {
                    owner = Some(c);
                    match enclosing {
                        Some(kind) => PathFound::NestedIn(kind),
                        None => PathFound::Object(c),
                    }
                }
                (None, Some(c)) => {
                    let kind = self.syms.class(c).kind;
                    owner = Some(c);
                    enclosing = Some(kind);
                    PathFound::Type(kind)
                }
                (None, None) => return PathFound::Nothing,
            };
        }
        found
    }

    /// The class of the object `s` names: a top-level object's or an object's member, or the
    /// val that holds the one instance of an object nested in a class.
    fn object_of(&self, s: SymId) -> Option<ClassId> {
        match self.syms.sym(s).kind {
            SymKind::Object(c) => Some(c),
            _ => self.inner_object_of_sym(s),
        }
    }

    /// The class `name` of package `p`, entering the std file that defines it when none of the
    /// entered ones does.
    pub fn demand_class(&mut self, p: PkgId, name: Name) -> Option<ClassId> {
        if let Some(c) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.class) {
            return Some(c);
        }
        if self.demand_std(p, name, crate::stdindex::TYPE) {
            return self.syms.pkg(p).entries.get(&name).and_then(|e| e.class);
        }
        None
    }

    /// The term `name` of package `p`, entering the std file that defines it when needed.
    pub fn demand_term(&mut self, p: PkgId, name: Name) -> Option<SymId> {
        if let Some(s) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.term) {
            return Some(s);
        }
        if self.demand_std(p, name, crate::stdindex::TERM) {
            return self.syms.pkg(p).entries.get(&name).and_then(|e| e.term);
        }
        None
    }

    /// The package `name` under `p`, which a locked std layer may bring with it.
    pub fn demand_pkg(&mut self, p: PkgId, name: Name) -> Option<PkgId> {
        if let Some(sub) = self.syms.pkg(p).entries.get(&name).and_then(|e| e.pkg) {
            return Some(sub);
        }
        self.demand_std_package(p, name)
    }

    pub(super) fn java_lang_class(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        let lang = self.java_lang_pkg()?;
        self.demand_class(lang, name)
    }

    pub(super) fn entered_java_lang_class(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        let lang = self.java_lang_pkg()?;
        self.syms.pkg(lang).entries.get(&name).and_then(|e| e.class)
    }

    pub(super) fn java_lang_object(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        let lang = self.java_lang_pkg()?;
        let sym = self.demand_term(lang, name)?;
        match self.syms.sym(sym).kind {
            SymKind::Object(c) => Some(c),
            _ => None,
        }
    }

    /// A class of teq's own `js` package.
    fn js_class(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        let js = self.syms.pkg(ROOT_PKG).entries.get(&crate::names::JS).and_then(|e| e.pkg)?;
        self.demand_class(js, name)
    }

    fn entered_js_class(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        let js = self.syms.pkg(ROOT_PKG).entries.get(&crate::names::JS).and_then(|e| e.pkg)?;
        self.syms.pkg(js).entries.get(&name).and_then(|e| e.class)
    }

    /// A term of package `scala` as the standard library defines it: a def, a val or an object.
    pub(super) fn std_def(&mut self, name: &str) -> Option<SymId> {
        let name = self.interner.intern(name);
        let scala = self.b.scala_pkg;
        self.demand_term(scala, name)
    }

    fn entered_std_def(&mut self, name: &str) -> Option<SymId> {
        let name = self.interner.intern(name);
        self.syms.pkg(self.b.scala_pkg).entries.get(&name).and_then(|e| e.term)
    }

    pub(super) fn std_object(&mut self, name: &str) -> Option<ClassId> {
        let sym = self.std_def(name)?;
        match self.syms.sym(sym).kind {
            SymKind::Object(c) => Some(c),
            _ => None,
        }
    }

    /// A class of package `scala` among the std files entered so far, through an alias of the
    /// package object when a library defines it that way (`scala.Seq` for
    /// `scala.collection.immutable.Seq`).
    /// `Singleton` once its std file is entered: until a program or a library names it, no
    /// bound asks for a singleton type.
    pub(super) fn note_singleton_class(&mut self) {
        if self.b.t_singleton == ERROR {
            self.b.t_singleton = self.entered_std_class("Singleton").map_or(ERROR, |c| self.types.class(c, &[]));
        }
    }

    fn entered_std_class(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        if let Some(c) = self.syms.pkg(self.b.scala_pkg).entries.get(&name).and_then(|e| e.class) {
            return Some(c);
        }
        if !self.scala_library_std() {
            return None;
        }
        self.std_class_through_alias(name)
    }

    /// A class of package `scala`, entering its std file on the first ask, or through an alias
    /// of the package object when a library defines it that way. teq's products hold none, so
    /// over them alone the class is found as in the whole build.
    pub fn std_class(&mut self, name: &str) -> Option<ClassId> {
        let name = self.interner.intern(name);
        let scala = self.b.scala_pkg;
        if let Some(c) = self.demand_class(scala, name) {
            return Some(c);
        }
        if self.loaded.as_ref().map_or(true, |l| l.cp.only_products()) {
            return None;
        }
        self.std_class_through_alias(name)
    }

    fn std_class_through_alias(&mut self, name: Name) -> Option<ClassId> {
        let scala = self.b.scala_pkg;
        match self.pkg_type(scala, name)? {
            resolve::TypeRef::Class(c) => Some(c),
            resolve::TypeRef::Member(..) | resolve::TypeRef::ValueMember(..) => None,
            resolve::TypeRef::Alias(a) => {
                self.complete_alias(a);
                let rhs = self.syms.aliases[a.idx()].rhs;
                let head = match self.types.get(rhs) {
                    Type::Lambda(_, body) => body,
                    _ => rhs,
                };
                match self.types.get(head) {
                    Type::Class(c, _) | Type::Ctor(c) => Some(c),
                    _ => None,
                }
            }
            resolve::TypeRef::Param(_) => None,
        }
    }

    /// The class a varargs literal is wrapped in: the std's `ArraySeq`, or scala-library's
    /// `immutable.ArraySeq.ofRef` over the JavaScript array under `--std=scala-library`.
    pub fn array_seq_class(&mut self) -> Option<ClassId> {
        if !self.scala_library_std() {
            // Entered with `Seq` when a varargs literal was typed; absent otherwise, with nothing
            // to wrap.
            return self.entered_std_class("ArraySeq");
        }
        let mut pkg = ROOT_PKG;
        for seg in ["scala", "collection", "immutable"] {
            let n = self.interner.intern(seg);
            pkg = self.syms.pkg(pkg).entries.get(&n).and_then(|e| e.pkg)?;
        }
        let name = self.interner.intern("ArraySeq");
        let resolve::TermRef::Global(sym) = self.pkg_term(pkg, name)? else { return None };
        let SymKind::Object(obj) = self.syms.sym(sym).kind else { return None };
        self.complete_class(obj);
        let of_ref = self.interner.intern("ofRef");
        match self.module_type(obj, of_ref)? {
            resolve::TypeRef::Class(c) => Some(c),
            _ => None,
        }
    }

    pub fn fun_type(&mut self, params: &[TypeId], ret: TypeId) -> TypeId {
        let c = self.function_class(params.len());
        let mut args = params.to_vec();
        args.push(ret);
        self.types.class(c, &args)
    }

    /// `A ?=> B`: a `ContextFunctionN` over the parameter and result types.
    pub fn ctx_fun_type(&mut self, params: &[TypeId], ret: TypeId) -> TypeId {
        let c = self.context_function_class(params.len());
        let mut args = params.to_vec();
        args.push(ret);
        self.types.class(c, &args)
    }

    /// For a context function type returns its parameter types and result type.
    pub fn as_context_function(&mut self, t: TypeId) -> Option<(Vec<TypeId>, TypeId)> {
        self.sync_arity_if_stale();
        let mut t = self.deref_alias(t);
        if let Some(parent) = self.named_fun_parent(t) {
            t = self.deref_alias(parent);
        }
        if let Type::Class(c, args) = self.types.get(t) {
            if !self.is_context_function_class(c) {
                return None;
            }
            let items = self.types.items(args);
            let arity = items.len().checked_sub(1)?;
            return Some((items[..arity].to_vec(), items[arity]));
        }
        None
    }

    pub fn tuple_type(&mut self, elems: &[TypeId]) -> TypeId {
        let c = self.tuple_class(elems.len());
        self.types.class(c, elems)
    }

    /// For a function type returns its parameter types and result type.
    pub fn as_function(&mut self, t: TypeId) -> Option<(Vec<TypeId>, TypeId)> {
        self.sync_arity_if_stale();
        let mut t = self.deref_alias(t);
        if let Some(parent) = self.named_fun_parent(t) {
            t = self.deref_alias(parent);
        }
        if let Type::Class(c, args) = self.types.get(t) {
            let items = self.types.items(args);
            if Some(c) == self.b.sub_evidence || Some(c) == self.b.eq_evidence {
                return Some((vec![items[0]], items[1]));
            }
            let arity = items.len().checked_sub(1)?;
            if self.b.functions.get(arity) != Some(&Some(c)) {
                return None;
            }
            return Some((items[..arity].to_vec(), items[arity]));
        }
        None
    }

    /// The function type among the base types of a class type.
    pub fn function_base(&mut self, t: TypeId) -> Option<(Vec<TypeId>, TypeId)> {
        self.sync_arity_if_stale();
        let c = self.class_of(t)?;
        self.complete_class(c);
        let bases: Vec<ClassId> = self.syms.class(c).base_types.iter().map(|&(b, _)| b).collect();
        let f = bases.into_iter().find(|b| self.b.functions.contains(&Some(*b)))?;
        let bt = self.base_type(t, f)?;
        self.as_function(bt)
    }

    /// `scala.Equals`, the parent of `Product`.
    pub fn equals_class(&self) -> Option<ClassId> {
        match self.types.get(self.b.t_equals) {
            Type::Class(c, _) => Some(c),
            _ => None,
        }
    }

    /// For `PartialFunction[A, B]` returns `A` and `B`.
    pub fn as_partial_function(&mut self, t: TypeId) -> Option<(TypeId, TypeId)> {
        let t = self.deref_alias(t);
        match self.types.get(t) {
            Type::Class(c, args) if self.is_partial_function(c) => {
                let items = self.types.items(args);
                Some((items[0], items[1]))
            }
            _ => None,
        }
    }

    /// The function type that a lambda or eta-expansion of `arity` parameters is expected to
    /// have: a union serves as expected type through its one function member of that arity, or
    /// its one function member if the arity fits none (`onChange: (String => Unit) | Unit`).
    pub fn expected_function(&mut self, t: TypeId, arity: usize) -> Option<(Vec<TypeId>, TypeId)> {
        let t = self.function_bound(t, arity);
        if !matches!(self.types.get(t), Type::Union(..)) {
            return self.as_function(t);
        }
        let mut members = Vec::new();
        self.union_alternatives(t, &mut members);
        let mut functions = members.into_iter().filter_map(|m| self.as_function(m)).collect::<Vec<_>>();
        if functions.len() > 1 {
            let fitting = functions.iter().filter(|(ps, _)| ps.len() == arity).count();
            if fitting == 1 {
                functions.retain(|(ps, _)| ps.len() == arity);
            }
        }
        (functions.len() == 1).then(|| functions.pop().unwrap())
    }

    /// An unsolved variable whose bounds hold one function or partial function type stands for
    /// that type when a lambda is typed against it, as scalac instantiates `A` of `List[A]` to
    /// `Int => Int` before it types the lambda in `val fs: List[Int => Int] = List(x => x + 1)`.
    pub fn function_bound(&mut self, t: TypeId, arity: usize) -> TypeId {
        self.function_bound_through(t, arity, 4)
    }

    fn function_bound_through(&mut self, t: TypeId, arity: usize, depth: u32) -> TypeId {
        let t = self.deref_alias(t);
        let Type::Var(v) = self.types.get(t) else { return t };
        let info = &self.tvars[v];
        let bounds: Vec<TypeId> = info.upper.iter().chain(&info.lower).copied().collect();
        let mut found = None;
        for bound in bounds {
            let mut bound = self.deref(bound);
            if matches!(self.types.get(bound), Type::Var(_)) {
                if depth == 0 {
                    continue;
                }
                bound = self.function_bound_through(bound, arity, depth - 1);
                if matches!(self.types.get(bound), Type::Var(_)) {
                    continue;
                }
            }
            if self.as_partial_function(bound).is_some() || self.expected_function(bound, arity).is_some() {
                match found {
                    Some(f) if f != bound => return t,
                    _ => found = Some(bound),
                }
            }
        }
        found.unwrap_or(t)
    }

    /// Runs `f`, which resolves something once for the session (a signature written out, a
    /// class's parents, a file's imports): what it reports is not reported again when the
    /// bodies of the file are typed again, so a retype keeps it (`Diagnostic::of_body`). A
    /// diagnostic is kept or not by the life of what produced it: `again` is the other case.
    pub fn once<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let outer = std::mem::replace(&mut self.diags.of_bodies, false);
        let r = f(self);
        self.diags.of_bodies = outer;
        r
    }

    /// Runs `f`, which builds what a retype of the file builds again, whatever asked for it
    /// first (the imports in a class's body, resolved for a signature of the class): what it
    /// reports the retype reports again, so it goes with the old typing.
    pub fn again<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let outer = std::mem::replace(&mut self.diags.of_bodies, true);
        let r = f(self);
        self.diags.of_bodies = outer;
        r
    }

    pub fn with_env<R>(&mut self, env: Env, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = std::mem::replace(&mut self.env, env);
        let saved_transparent = std::mem::take(&mut self.transparent);
        let saved_ext_scope = self.ext_scope.take();
        self.compute_transparent();
        let r = f(self);
        self.env = saved;
        self.transparent = saved_transparent;
        self.ext_scope = saved_ext_scope;
        r
    }

    /// Builds the environment in which the members of `owner` are typed, with every import of
    /// the enclosing bodies.
    pub fn env_for(&mut self, file: FileId, owner: Owner) -> Env {
        self.env_at(file, owner, u32::MAX)
    }

    /// The environment at the offset `pos` of the body of `owner`: an import counts from where
    /// it stands.
    pub fn env_at(&mut self, file: FileId, owner: Owner, pos: u32) -> Env {
        let mut chain = Vec::new();
        let mut o = owner;
        let mut at = pos;
        while let Owner::Class(c) = o {
            chain.push((c, at));
            let info = self.syms.class(c);
            at = info.span.start;
            o = info.owner;
        }
        let mut env = match chain.last() {
            Some(&(c, _)) if o == Owner::Local => self.anon_env(c),
            _ => Env { file, frames: Vec::with_capacity(chain.len() + 4), imports: Vec::new() },
        };
        for &(c, at) in chain.iter().rev() {
            env.frames.push(Frame::Class(c));
            if self.syms.class(c).has_imports {
                let scope = env.imports.len();
                for &(start, imp) in self.class_imports(c).iter() {
                    if start < at {
                        env.push_import(scope, imp);
                    }
                }
            }
        }
        env
    }

    fn compute_transparent(&mut self) {
        let file = self.env.file;
        let enclosing: Vec<ClassId> = self
            .env
            .frames
            .iter()
            .filter_map(|f| if let Frame::Class(c) = f { Some(*c) } else { None })
            .collect();
        // A top-level opaque type is transparent to the file's top-level definitions and its
        // companion object, an object's to the object's body.
        let outermost = enclosing.iter().copied().find(|&c| self.syms.class(c).owner != Owner::Local);
        for &id in &self.file_opaques[file.0 as usize] {
            let info = self.syms.class(id);
            let visible = match info.owner {
                Owner::Package(_) => outermost.map_or(true, |c| {
                    let o = self.syms.class(c);
                    o.kind == ClassKind::Object && o.name == info.name && o.owner == info.owner
                }),
                Owner::Class(o) => enclosing.contains(&o),
                Owner::Local => false,
            };
            if visible {
                self.transparent.push(id);
            }
        }
    }
}

/// What a qualified path of `--cacheable-state` names (`Worker::object_by_path`).
enum PathFound {
    Object(ClassId),
    /// A class or a trait, of that kind, that no object of the name stands beside.
    Type(ClassKind),
    /// An object inside a class or a trait, of the innermost's kind.
    NestedIn(ClassKind),
    Nothing,
}
