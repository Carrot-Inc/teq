use crate::arena::{worker_base, worker_of, SharedMap, LOCAL_BASE, WORKER_SPAN};
use crate::intern::Name;
use crate::shared::{hash_of, lock_depth, Region, Serial, SlabVec, Table};
use std::cell::{Cell, UnsafeCell};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod view;

macro_rules! id_type {
    ($($name:ident),*) => {$(
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
        pub struct $name(pub u32);
        impl $name {
            #[inline]
            pub fn idx(self) -> usize { self.0 as usize }
        }
    )*};
}
id_type!(TypeId, TList, ClassId, SymId, TParamId, TVarId, PkgId, AliasId, LitId, BlockedId, RefineId, MatchId);

/// A type variable is a worker's: the worker's tag in the high byte, the variable's index in
/// the worker's table below (`typer::TVars`). Two workers' variables never share a type.
impl TVarId {
    pub const TAG_SHIFT: u32 = 24;

    #[inline]
    pub fn tagged(tag: u32, index: usize) -> TVarId {
        TVarId(tag << Self::TAG_SHIFT | index as u32)
    }

    #[inline]
    pub fn index(self) -> usize {
        (self.0 & ((1 << Self::TAG_SHIFT) - 1)) as usize
    }

    #[inline]
    pub fn tag(self) -> u32 {
        self.0 >> Self::TAG_SHIFT
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    Any,
    Nothing,
    Error,
    Class(ClassId, TList),
    Param(TParamId),
    /// Application of a higher-kinded type parameter, e.g. `F[A]`.
    AppParam(TParamId, TList),
    /// An unapplied class used as a type constructor argument, e.g. `List` for `F[_]`.
    Ctor(ClassId),
    /// A type lambda; the first list holds its parameters as `Param` types.
    Lambda(TList, TypeId),
    /// A polymorphic function type, `[T] => (A, B) => C`: the parameters as `Param` types, then
    /// each one's lower and upper bound (`poly_params`, `poly_bounds`), and the function type over
    /// them. The bounds are the type's own, as a `PolyType`'s parameter infos are (dotty's
    /// `TypeMap.mapOverLambda` maps them): what the parameters' symbols hold may be a former
    /// type's. It erases to the function type on every target.
    Poly(TList, TypeId),
    Var(TVarId),
    AppVar(TVarId, TList),
    Union(TypeId, TypeId),
    Inter(TypeId, TypeId),
    /// A literal type such as `1` or `"fast"`; the value sits in the store, so that `Type`
    /// stays three words wide.
    Lit(LitId),
    /// The wildcard argument of an invariant type parameter, `Box[?]`: some unknown type, read
    /// as `Any` and written as `Nothing`.
    Wild,
    /// A wildcard argument with bounds, `Box[? <: T]` or `Box[? >: L]`, lower bound first: some
    /// unknown type between the two, read as the upper bound and written as the lower one.
    BoundedWild(TypeId, TypeId),
    /// A type read from a library that `Type` has no case for yet (a refinement, a match type,
    /// an abstract type member); the store keeps its shape and spelling for the message a use
    /// site reports. It conforms to nothing and nothing conforms to it.
    Blocked(BlockedId),
    /// `C.this` inside a class or trait `C`, the prefix of its type members; an object stands
    /// for itself as its class type.
    This(ClassId),
    /// `x.type` for a local, a parameter or a top-level val.
    Term(SymId),
    /// `p.x.type` for a val `x` reached through the path `p`.
    Select(TypeId, SymId),
    /// `p.T`, `C.this.T` or `C#T`: the abstract type member `T` seen through the prefix type.
    /// A member that a prefix fixes is replaced by what it equals, so only abstract ones remain.
    Member(TypeId, Name),
    /// A type member applied to arguments, `p.F[A]`; the first field is the `Member` or `Decl`.
    AppMember(TypeId, TList),
    /// An abstract type member of an object or a package, which needs no prefix.
    Decl(AliasId),
    /// `T { type A = X }`: the parent with one refinement; several nest, the last one outermost.
    Refined(TypeId, RefineId),
    /// `S match { case P => T ... }`: the scrutinee and, in the store, the cases and the
    /// declared upper bound. It is reduced by the typer where a use needs it.
    Match(TypeId, MatchId),
    /// `M[A, B]` for an alias `M` whose right-hand side is a match type, kept by name until a
    /// use expands it: a case body may name the alias itself, and an error shows the alias.
    Alias(AliasId, TList),
}

/// One case of a match type, `[binders] =>> pattern => body`; the binders are the `Param`
/// types of the pattern's type variables.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MatchCase {
    pub binders: TList,
    pub pattern: TypeId,
    pub body: TypeId,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MatchInfo {
    pub cases: Box<[MatchCase]>,
    pub bound: TypeId,
}

/// One member of a refinement type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Refinement {
    /// `type A = X`; a parameterized one is a `Lambda`.
    Alias(Name, TypeId),
    /// `type A >: L <: U`
    Bounds(Name, TypeId, TypeId),
    /// `def m(..): R`: the symbol stands for the member, the list holds the signature's types
    /// (the parameters clause by clause, then the result), which a substitution rewrites where
    /// the symbol keeps the declared ones.
    Term(Name, SymId, TList),
    /// `val x: T`: the symbol stands for the member, the type is the refinement's own, which
    /// a substitution rewrites where the symbol keeps the declared one.
    Val(Name, SymId, TypeId),
}

impl Refinement {
    pub fn name(self) -> Name {
        match self {
            Refinement::Alias(n, _) | Refinement::Bounds(n, ..) | Refinement::Term(n, ..) | Refinement::Val(n, ..) => n,
        }
    }
}

/// A `Double` is kept as its bits so that the value can be hashed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LitVal {
    Int(i32),
    Long(i64),
    Double(u64),
    Char(u16),
    Bool(bool),
    Str(Name),
}

pub const ANY: TypeId = TypeId(0);
pub const NOTHING: TypeId = TypeId(1);
pub const ERROR: TypeId = TypeId(2);
pub const WILD: TypeId = TypeId(3);

pub const EMPTY_LIST: TList = TList(0);

/// Per type, what occurs anywhere in it, as bits of `TypeStore::flags`: a `Wild`, an inference
/// variable, a path (`This`, `Term`, `Select`) or a member seen through one (what a selection
/// has to see from its receiver), a match type or an applied type-level operation (what the
/// typer reduces before the type is stored or compared), or an applied higher-kinded type
/// parameter.
const WILD_BIT: u8 = 1;
const VARS_BIT: u8 = 2;
const PATHS_BIT: u8 = 4;
const REDUCIBLE_BIT: u8 = 8;
const APP_PARAM_BIT: u8 = 16;
/// The type mentions an id of a worker's own chunk (`arena::LOCAL_BASE` on), which the merge
/// renumbers: the type is made again under the new ids then (`merge.rs`).
const LOCAL_BIT: u8 = 32;
/// The type is a path kept apart from the index (`TypeStore::term_unshared`): its provenance,
/// read wherever the type is and passed on to nothing made of it (`flags_of`).
const UNSHARED_BIT: u8 = 64;

#[inline]
fn local_id(id: u32) -> u8 {
    if id >= crate::arena::LOCAL_BASE { LOCAL_BIT } else { 0 }
}

pub struct TypeStore {
    /// The body phase's overlays (`TEQ_TYPE_OVERLAYS=1`): set at
    /// the fork before any worker starts, taken at the merge after the join. Declared first,
    /// so that the overlays drop what they hold before the mappings go.
    apart: UnsafeCell<Option<Box<Apart>>>,
    /// Whether the workers' inserts go to their overlays: the fork's to the merge's.
    overlaid: AtomicBool,
    /// Whether the merge gave the overlays back (`reclaim_overlays`): an overlay's entry read
    /// after it is refused in the assertion-enabled builds.
    reclaimed: AtomicBool,
    serial: Serial,
    types: SlabVec<Type>,
    type_index: Table,
    flags: SlabVec<u8>,
    lists: SlabVec<Box<[TypeId]>>,
    list_index: Table,
    lits: SlabVec<LitVal>,
    lit_index: Table,
    blocked: SlabVec<Box<str>>,
    blocked_index: Table,
    refinements: SlabVec<Refinement>,
    refine_index: Table,
    /// Whether a `val` or `def` refinement exists, which a member selection reads.
    term_refinements: AtomicBool,
    matches: SlabVec<MatchInfo>,
    match_index: Table,
    /// Per class, whether it is an operation of `scala.compiletime.ops`, evaluated on literal
    /// arguments; set by the typer once the standard library is entered.
    op_classes: SlabVec<AtomicBool>,
    /// Per class, whether it is an opaque type declared in a class or trait, whose type names
    /// its owner's `this` as a path does: seen from an object it is that object's own type.
    path_classes: SlabVec<AtomicBool>,
    /// The same marks for the classes a worker makes during the bodies, whose tagged ids no
    /// dense table may be sized by (`arena::LOCAL_BASE`): a sparse map each.
    op_local: UnsafeCell<SharedMap<ClassId, ()>>,
    path_local: UnsafeCell<SharedMap<ClassId, ()>>,
}

// The store is shared by reference across the parallel typer's workers: readers take no lock
// (`shared.rs`), writers take `serial`.
unsafe impl Sync for TypeStore {}

thread_local! {
    /// How deeply `apply_ctor` is nested on this thread.
    static APPLY_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub type Subst = Vec<(TParamId, TypeId)>;

impl TypeStore {
    /// The bytes the types hold: the arrays, the tables that find a type again, the lists.
    pub fn held(&self) -> usize {
        self.types.held()
            + self.type_index.held()
            + self.flags.held()
            + self.lists.held()
            + self.lists.as_slice().iter().map(|l| l.len() * std::mem::size_of::<TypeId>()).sum::<usize>()
            + self.list_index.held()
            + self.lits.held()
            + self.lit_index.held()
            + self.blocked.held()
            + self.blocked.as_slice().iter().map(|b| b.len()).sum::<usize>()
            + self.blocked_index.held()
            + self.refinements.held()
            + self.refine_index.held()
            + self.matches.held()
            + self.matches.as_slice().iter().map(|m| m.cases.len() * std::mem::size_of::<MatchCase>()).sum::<usize>()
            + self.match_index.held()
            + self.apart_ref().map_or(0, |a| a.held())
            + self.op_classes.held()
            + self.path_classes.held()
            + self.op_local().held()
            + self.path_local().held()
    }
}

impl TypeStore {
    /// A store with room for a program of some tens of thousands of types (the budget's
    /// variants and the core-only benchmark) before its first resize; a small program touches
    /// the table's tags (32 KB) and few pages of the rest.
    pub fn new() -> TypeStore {
        let s = TypeStore {
            serial: Serial::new(true, crate::measure::Wait::TypeStore),
            types: SlabVec::with_capacity(16384),
            type_index: Table::with_capacity(16384),
            flags: SlabVec::with_capacity(16384),
            lists: SlabVec::with_capacity(4096),
            list_index: Table::with_capacity(4096),
            lits: SlabVec::with_capacity(16),
            lit_index: Table::with_capacity(16),
            blocked: SlabVec::with_capacity(16),
            blocked_index: Table::with_capacity(16),
            refinements: SlabVec::with_capacity(16),
            refine_index: Table::with_capacity(16),
            term_refinements: AtomicBool::new(false),
            matches: SlabVec::with_capacity(16),
            match_index: Table::with_capacity(16),
            op_classes: SlabVec::with_capacity(64),
            path_classes: SlabVec::with_capacity(64),
            op_local: UnsafeCell::new(SharedMap::new()),
            path_local: UnsafeCell::new(SharedMap::new()),
            apart: UnsafeCell::new(None),
            overlaid: AtomicBool::new(false),
            reclaimed: AtomicBool::new(false),
        };
        assert_eq!(s.mk(Type::Any), ANY);
        assert_eq!(s.mk(Type::Nothing), NOTHING);
        assert_eq!(s.mk(Type::Error), ERROR);
        assert_eq!(s.mk(Type::Wild), WILD);
        assert_eq!(s.list(&[]), EMPTY_LIST);
        s
    }

    /// Room for a program of `types` types and `lists` lists: a session's full build sized
    /// from the last build's counts grows through no doubling and retires nothing.
    pub fn reserve(&self, types: usize, lists: usize) {
        self.types.reserve(types);
        self.flags.reserve(types);
        self.type_index.reserve(types);
        self.lists.reserve(lists);
        self.list_index.reserve(lists);
    }

    /// Frees what the stores outgrew: between two typing steps, when no reference into a
    /// retired buffer is alive (a store keeps them for the readers of a shared phase).
    pub fn release_retired(&self) {
        unsafe {
            self.types.release_retired();
            self.flags.release_retired();
            self.lists.release_retired();
            self.lits.release_retired();
            self.blocked.release_retired();
            self.refinements.release_retired();
            self.matches.release_retired();
            self.op_classes.release_retired();
            self.path_classes.release_retired();
            for table in [&self.type_index, &self.list_index, &self.lit_index, &self.blocked_index, &self.refine_index, &self.match_index] {
                table.release_retired();
            }
        }
    }

    /// Whether one thread owns the store, whose inserts then take no lock.
    pub fn set_exclusive(&self, exclusive: bool) {
        self.serial.set_exclusive(exclusive);
    }

    /// Whether one thread owns the store's inserts (`set_exclusive`).
    pub fn is_exclusive(&self) -> bool {
        self.serial.is_exclusive()
    }

    /// How many types and how many lists the store holds.
    pub fn len(&self) -> (usize, usize) {
        (self.types.len(), self.lists.len())
    }

    #[inline]
    fn find_type(&self, h: u64, t: Type) -> Option<TypeId> {
        self.type_index.find(h, #[inline(always)] |i| *self.types.get(i as usize) == t).map(TypeId)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn mk(&self, t: Type) -> TypeId {
        let h = hash_of(&t);
        if let Some(id) = self.find_type(h, t) {
            return id;
        }
        let Ok(held) = self.insert_lock() else { return self.mk_apart(h, t) };
        if held.is_some() {
            if let Some(id) = self.find_type(h, t) {
                return id;
            }
        }
        let flags = self.flags_of(t);
        let id = self.types.push(t);
        self.flags.push(flags);
        self.type_index.insert(h, id as u32);
        TypeId(id as u32)
    }

    /// The store's lock for an insert past a miss of the index: none while one thread owns
    /// the store, `Err` with the overlays on, whose inserts take their own path.
    #[inline(always)]
    fn insert_lock(&self) -> Result<Option<crate::shared::Held<'_>>, ()> {
        if self.serial.is_exclusive() {
            return Ok(None);
        }
        if self.overlaid.load(Ordering::Relaxed) {
            return Err(());
        }
        self.insert_lock_shared()
    }

    #[cold]
    #[inline(never)]
    fn insert_lock_shared(&self) -> Result<Option<crate::shared::Held<'_>>, ()> {
        if self.overlaid.load(Ordering::Relaxed) {
            Err(())
        } else {
            Ok(Some(self.serial.lock()))
        }
    }

    /// What a type mentions anywhere (the bits of `flags`), from its parts' flags and the
    /// class marks of the store it is made in, once, when it is made; a part's provenance
    /// (`UNSHARED_BIT`) is its own.
    #[inline(always)]
    fn flags_of(&self, t: Type) -> u8 {
        self.flags_of_parts(t) & !UNSHARED_BIT
    }

    #[inline(always)]
    fn flags_of_parts(&self, t: Type) -> u8 {
        // A prefix or a refinement passes on what it mentions except a wildcard, which counts
        // only where it is a type argument.
        const THROUGH_PREFIX: u8 = VARS_BIT | PATHS_BIT | REDUCIBLE_BIT | LOCAL_BIT;
        match t {
            Type::Wild => WILD_BIT,
            Type::BoundedWild(lo, hi) => WILD_BIT | self.entry_flag(lo) | self.entry_flag(hi),
            Type::Var(_) => VARS_BIT,
            Type::AppVar(_, args) => VARS_BIT | self.list_flags(args),
            Type::This(c) => PATHS_BIT | local_id(c.0),
            Type::Term(s) => PATHS_BIT | local_id(s.0),
            Type::Select(p, s) => PATHS_BIT | (self.entry_flag(p) & THROUGH_PREFIX) | local_id(s.0),
            Type::Member(p, _) => PATHS_BIT | (self.entry_flag(p) & THROUGH_PREFIX),
            Type::Class(c, args) => {
                let op = if args != EMPTY_LIST && self.is_op_class(c) { REDUCIBLE_BIT } else { 0 };
                let path = if self.is_path_class(c) { PATHS_BIT } else { 0 };
                op | path | self.list_flags(args) | local_id(c.0)
            }
            Type::Ctor(c) => local_id(c.0),
            Type::Param(p) => local_id(p.0),
            Type::AppParam(p, args) => APP_PARAM_BIT | self.list_flags(args) | local_id(p.0),
            Type::Decl(a) => local_id(a.0),
            Type::Alias(a, args) => REDUCIBLE_BIT | self.list_flags(args) | local_id(a.0),
            Type::Lambda(ps, b) => self.entry_flag(b) | (self.list_flags(ps) & LOCAL_BIT),
            Type::Poly(ps, b) => {
                let items = self.entry_items(ps);
                let (params, bounds) = items.split_at(items.len() / 3);
                let local = params.iter().fold(0, |acc, &a| acc | self.entry_flag(a)) & LOCAL_BIT;
                self.entry_flag(b) | bounds.iter().fold(local, |acc, &a| acc | self.entry_flag(a))
            }
            Type::Union(a, b) | Type::Inter(a, b) => self.entry_flag(a) | self.entry_flag(b),
            Type::AppMember(m, args) => {
                let args = self.list_flags(args);
                ((self.entry_flag(m) | args) & THROUGH_PREFIX) | (args & APP_PARAM_BIT)
            }
            Type::Refined(p, r) => (self.entry_flag(p) | self.refinement_flags(r)) & THROUGH_PREFIX,
            Type::Match(s, m) => REDUCIBLE_BIT | self.entry_flag(s) | self.match_parts(m).fold(0, |acc, a| acc | self.entry_flag(a)),
            _ => 0,
        }
    }

    /// A path to `sym` with an id no other path to it has: `Symbol.typeRef` of a val in the
    /// reflect API, which is told from the same val's `termRef` by the id. It is kept out of
    /// the index, and its provenance is the store's (`is_unshared`): an import, an export or
    /// the merge makes it again as a path of its own, never through `mk`.
    pub fn term_unshared(&self, sym: SymId) -> TypeId {
        let flags = PATHS_BIT | UNSHARED_BIT | local_id(sym.0);
        let _held = match self.insert_lock() {
            Ok(held) => held,
            Err(()) => return self.unshared_apart(sym, flags, None),
        };
        let id = self.types.push(Type::Term(sym));
        self.flags.push(flags);
        TypeId(id as u32)
    }

    /// An unshared path made with the overlays on, in this worker's overlay or, for the loader's
    /// lock holder, in the base: an original (`origin` none), which stands for itself in its
    /// namespace, or the copy of `origin` a translation makes, recorded as that namespace's
    /// one representative of it (`unshared_copy`).
    #[cold]
    #[inline(never)]
    fn unshared_apart(&self, sym: SymId, flags: u8, origin: Option<TypeId>) -> TypeId {
        let apart = self.apart_ref().expect("the overlays");
        let id = match self.overlay_here() {
            Some(ov) => {
                let id = ov.push_type(Type::Term(sym), flags);
                ov.owner().unshared_reps.insert(origin.unwrap_or(id).0, id);
                id
            }
            None => {
                let _held = self.serial.lock();
                let id = TypeId(self.types.push(Type::Term(sym)) as u32);
                self.flags.push(flags);
                apart.base_reps.insert(origin.unwrap_or(id), id);
                id
            }
        };
        note_made(Sub::Types);
        if let Some(o) = origin {
            apart.origins.insert(id, o);
        }
        id
    }

    /// The unshared path `t` in the namespace this thread makes types in (its worker's overlay,
    /// or the base for the lock holder): the one representative that namespace holds of `t`'s
    /// origin, the original path that `t` is or that `t` was translated from, made there on the
    /// first translation. So an export and the import of its result give back the original,
    /// and every translation of one original into a namespace, from wherever, gives one path.
    fn unshared_copy(&self, t: TypeId) -> TypeId {
        let Type::Term(sym) = self.entry(t) else { unreachable!("an unshared path is a Term") };
        let Some(apart) = self.apart_ref() else { return self.term_unshared(sym) };
        let origin = apart.origins.get(&t).copied().unwrap_or(t);
        let rep = match self.overlay_here() {
            // A base path is in the worker's view: where the worker holds no representative of
            // its origin, it is the worker's.
            Some(ov) if t.0 < LOCAL_BASE => {
                Some(*ov.owner().unshared_reps.entry(origin.0).or_insert(t))
            }
            Some(ov) => ov.owner().unshared_reps.get(&origin.0).copied(),
            None => apart.base_reps.get(&origin).copied(),
        };
        rep.unwrap_or_else(|| self.unshared_apart(sym, self.entry_flag(t), Some(origin)))
    }

    /// The original unshared path `t` is or was translated from (`unshared_copy`): `t` itself
    /// for an original, and for every path without the overlays.
    pub fn unshared_origin(&self, t: TypeId) -> TypeId {
        self.apart_ref().and_then(|a| a.origins.get(&t).copied()).unwrap_or(t)
    }

    /// The path the workers' merge made for an origin, the base's original of it from here, and
    /// its own representative there (`unshared_copy`).
    pub fn note_unshared_original(&self, t: TypeId) {
        if let Some(apart) = self.apart_ref() {
            apart.base_reps.insert(t, t);
        }
    }

    /// Whether the type is a path kept apart from the index (`term_unshared`).
    #[inline]
    pub fn is_unshared(&self, t: TypeId) -> bool {
        self.flag(t) & UNSHARED_BIT != 0
    }

    #[inline]
    fn entry_unshared(&self, t: TypeId) -> bool {
        self.entry_flag(t) & UNSHARED_BIT != 0
    }

    /// The flags of a list of types, joined.
    #[inline]
    fn list_flags(&self, l: TList) -> u8 {
        self.entry_items(l).iter().fold(0, |acc, &a| acc | self.entry_flag(a))
    }

    /// A polymorphic function type's parameters, the first third of its list.
    pub fn poly_params(&self, l: TList) -> &[TypeId] {
        let items = self.items(l);
        &items[..items.len() / 3]
    }

    /// A polymorphic function type's bounds, each parameter's lower and upper, after them.
    pub fn poly_bounds(&self, l: TList) -> &[TypeId] {
        let items = self.items(l);
        &items[items.len() / 3..]
    }

    /// The match type over `scrutinee` with these cases and upper bound.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn match_type(&self, scrutinee: TypeId, cases: &[MatchCase], bound: TypeId) -> TypeId {
        let info = MatchInfo { cases: cases.into(), bound };
        let h = hash_of(&info);
        let find = || self.match_index.find(h, |i| *self.matches.get(i as usize) == info);
        let m = match find() {
            Some(m) => m,
            None => {
                let Ok(held) = self.insert_lock() else { return self.mk(Type::Match(scrutinee, MatchId(self.match_apart(h, &info)))) };
                let again = if held.is_some() { find() } else { None };
                match again {
                    Some(m) => m,
                    None => {
                        let m = self.matches.push(info) as u32;
                        self.match_index.insert(h, m);
                        m
                    }
                }
            }
        };
        self.mk(Type::Match(scrutinee, MatchId(m)))
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn match_info(&self, m: MatchId) -> &MatchInfo {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Matches, m.0);
        self.entry_match(m)
    }

    #[inline]
    fn entry_match(&self, m: MatchId) -> &MatchInfo {
        self.matches.get(m.idx())
    }

    /// The types a match type's cases and bound mention.
    fn match_parts(&self, m: MatchId) -> impl Iterator<Item = TypeId> + '_ {
        let info = self.entry_match(m);
        info.cases.iter().flat_map(|c| [c.pattern, c.body]).chain([info.bound])
    }

    pub fn mark_op_class(&self, c: ClassId) {
        if c.0 >= LOCAL_BASE {
            self.op_local().insert(c, ());
            return;
        }
        let _held = self.serial.hold();
        Self::mark(&self.op_classes, c);
    }

    #[inline]
    pub fn is_op_class(&self, c: ClassId) -> bool {
        if c.0 >= LOCAL_BASE {
            return self.op_local().contains_key(&c);
        }
        Self::marked(&self.op_classes, c)
    }

    pub fn mark_path_class(&self, c: ClassId) {
        if c.0 >= LOCAL_BASE {
            self.path_local().insert(c, ());
            return;
        }
        let _held = self.serial.hold();
        Self::mark(&self.path_classes, c);
    }

    #[inline]
    pub fn is_path_class(&self, c: ClassId) -> bool {
        if c.0 >= LOCAL_BASE {
            return self.path_local().contains_key(&c);
        }
        Self::marked(&self.path_classes, c)
    }

    fn mark(marks: &SlabVec<AtomicBool>, c: ClassId) {
        while marks.len() <= c.idx() {
            marks.push(AtomicBool::new(false));
        }
        marks.get(c.idx()).store(true, Ordering::Release);
    }

    #[inline]
    fn marked(marks: &SlabVec<AtomicBool>, c: ClassId) -> bool {
        c.idx() < marks.len() && marks.get(c.idx()).load(Ordering::Relaxed)
    }

    /// Whether a `val` or `def` refinement exists, which a member selection reads.
    #[inline]
    pub fn term_refinements(&self) -> bool {
        self.term_refinements.load(Ordering::Relaxed)
    }

    /// Whether a match type or an applied type-level operation occurs anywhere in the type.
    #[inline]
    pub fn is_reducible(&self, t: TypeId) -> bool {
        self.flag(t) & REDUCIBLE_BIT != 0
    }

    /// Whether the head of the type is what `Typer::reduce_head` reduces: a match type, an
    /// alias application, an applied type-level operation or tuple cons.
    #[inline]
    pub fn head_reducible(&self, t: Type) -> bool {
        match t {
            Type::Match(..) | Type::Alias(..) => true,
            Type::Class(c, args) => args != EMPTY_LIST && self.is_op_class(c),
            _ => false,
        }
    }

    /// The literal type of a value.
    pub fn lit(&self, v: LitVal) -> TypeId {
        let h = hash_of(&v);
        let find = || self.lit_index.find(h, |i| *self.lits.get(i as usize) == v);
        let id = match find() {
            Some(id) => id,
            None => {
                let Ok(held) = self.insert_lock() else { return self.mk(Type::Lit(LitId(self.lit_apart(h, v)))) };
                let again = if held.is_some() { find() } else { None };
                match again {
                    Some(id) => id,
                    None => {
                        let id = self.lits.push(v) as u32;
                        self.lit_index.insert(h, id);
                        id
                    }
                }
            }
        };
        self.mk(Type::Lit(LitId(id)))
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn lit_val(&self, l: LitId) -> LitVal {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Lits, l.0);
        self.entry_lit(l)
    }

    #[inline]
    fn entry_lit(&self, l: LitId) -> LitVal {
        *self.lits.get(l.idx())
    }

    /// The type standing for a library type of an unsupported shape, described as
    /// `<shape>: <spelling>`.
    pub fn blocked(&self, description: &str) -> TypeId {
        let h = hash_of(description);
        let find = || self.blocked_index.find(h, |i| &**self.blocked.get(i as usize) == description);
        let id = match find() {
            Some(id) => id,
            None => {
                let Ok(held) = self.insert_lock() else { return self.mk(Type::Blocked(BlockedId(self.blocked_apart(h, description)))) };
                let again = if held.is_some() { find() } else { None };
                match again {
                    Some(id) => id,
                    None => {
                        let id = self.blocked.push(description.into()) as u32;
                        self.blocked_index.insert(h, id);
                        id
                    }
                }
            }
        };
        self.mk(Type::Blocked(BlockedId(id)))
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn blocked_description(&self, b: BlockedId) -> &str {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Blocked, b.0);
        self.entry_blocked(b)
    }

    #[inline]
    fn entry_blocked(&self, b: BlockedId) -> &str {
        self.blocked.get(b.idx())
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn refine(&self, r: Refinement) -> RefineId {
        let h = hash_of(&r);
        let find = || self.refine_index.find(h, |i| *self.refinements.get(i as usize) == r);
        if let Some(id) = find() {
            return RefineId(id);
        }
        let Ok(held) = self.insert_lock() else { return self.refine_apart(h, r) };
        if held.is_some() {
            if let Some(id) = find() {
                return RefineId(id);
            }
        }
        if matches!(r, Refinement::Term(..) | Refinement::Val(..)) {
            self.term_refinements.store(true, Ordering::Relaxed);
        }
        let id = self.refinements.push(r) as u32;
        self.refine_index.insert(h, id);
        RefineId(id)
    }

    /// A `val` refinement of type `ty`, which the symbol of `r` has as its signature.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn refinement(&self, r: RefineId) -> Refinement {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Refinements, r.0);
        self.entry_refinement(r)
    }

    #[inline]
    fn entry_refinement(&self, r: RefineId) -> Refinement {
        *self.refinements.get(r.idx())
    }

    /// The flags of the types a refinement mentions.
    fn refinement_flags(&self, r: RefineId) -> u8 {
        match self.entry_refinement(r) {
            Refinement::Alias(_, t) => self.entry_flag(t),
            Refinement::Val(_, s, t) => self.entry_flag(t) | local_id(s.0),
            Refinement::Bounds(_, lo, hi) => self.entry_flag(lo) | self.entry_flag(hi),
            Refinement::Term(_, s, l) => self.list_flags(l) | local_id(s.0),
        }
    }

    /// Whether the type mentions an id of a worker's own chunk.
    #[inline]
    pub fn mentions_local(&self, t: TypeId) -> bool {
        self.flag(t) & LOCAL_BIT != 0
    }

    /// Whether the base's entry `id` of sub-store `sub`, made after the fork, is not worker `ov`'s
    /// canonical entry of its structure in the shared namespace: canonicality is over canonical
    /// parts, so the entry is not when the worker holds its own equivalent of it or of any base
    /// entry made after the fork among its descendants (the import then rebuilds it over the
    /// worker's parts). The base's entries from before the fork and the worker's own are canonical
    /// by construction (the probes find the former first, the construction check keeps the
    /// latter's parts canonical).
    #[cfg(debug_assertions)]
    fn not_canonical(&self, ov: &Overlay, sub: Sub, id: u32) -> bool {
        let late = |sub: Sub, id: u32| id >= self.bound(sub) && id < LOCAL_BASE;
        let mut todo = vec![(sub, id)];
        let mut seen = crate::intern::FxMap::default();
        while let Some((sub, id)) = todo.pop() {
            if !late(sub, id) || seen.insert((sub as u8, id), ()).is_some() {
                continue;
            }
            if self.shadowed(ov, sub, id) {
                return true;
            }
            todo.extend(view::parts_of(self, sub, id).into_iter().map(|(s, p)| (Sub::ALL[s as usize], p)));
        }
        false
    }

    /// Whether worker `ov`'s overlay holds an entry of sub-store `sub` equal to the base's entry
    /// `id`, part for part, or for an unshared path a representative of its origin other than it.
    #[cfg(debug_assertions)]
    fn shadowed(&self, ov: &Overlay, sub: Sub, id: u32) -> bool {
        let at = |i: u32| (ov.base + i) as usize;
        let index = &ov.index[sub as usize];
        match sub {
            // An unshared path by its origin: another than the worker's representative of it.
            Sub::Types if self.entry_unshared(TypeId(id)) => {
                let origin = self.unshared_origin(TypeId(id));
                ov.owner().unshared_reps.get(&origin.0).is_some_and(|&r| r.0 != id)
            }
            Sub::Types => {
                let t = self.entry(TypeId(id));
                index.find(hash_of(&t), |i| *self.types.get(at(i)) == t).is_some()
            }
            Sub::Lists => {
                let items = self.entry_items(TList(id));
                index.find(hash_of(items), |i| &**self.lists.get(at(i)) == items).is_some()
            }
            Sub::Lits => {
                let v = self.entry_lit(LitId(id));
                index.find(hash_of(&v), |i| *self.lits.get(at(i)) == v).is_some()
            }
            Sub::Blocked => {
                let d = self.entry_blocked(BlockedId(id));
                index.find(hash_of(d), |i| &**self.blocked.get(at(i)) == d).is_some()
            }
            Sub::Refinements => {
                let r = self.entry_refinement(RefineId(id));
                index.find(hash_of(&r), |i| *self.refinements.get(at(i)) == r).is_some()
            }
            Sub::Matches => {
                let m = self.entry_match(MatchId(id));
                index.find(hash_of(m), |i| self.matches.get(at(i)) == m).is_some()
            }
        }
    }

    /// The types a refinement mentions, read raw.
    #[cfg(debug_assertions)]
    fn refinement_types_raw(&self, r: RefineId) -> Vec<TypeId> {
        match self.entry_refinement(r) {
            Refinement::Alias(_, t) | Refinement::Val(_, _, t) => vec![t],
            Refinement::Bounds(_, lo, hi) => vec![lo, hi],
            Refinement::Term(_, _, l) => self.entry_items(l).to_vec(),
        }
    }

    /// The types a refinement mentions.
    fn refinement_types(&self, r: RefineId) -> Vec<TypeId> {
        match self.refinement(r) {
            Refinement::Alias(_, t) | Refinement::Val(_, _, t) => vec![t],
            Refinement::Bounds(_, lo, hi) => vec![lo, hi],
            Refinement::Term(_, _, l) => self.items(l).to_vec(),
        }
    }

    /// The refinements of a type, outermost last, and the parent under them.
    pub fn refinements_of(&self, t: TypeId) -> (TypeId, Vec<RefineId>) {
        let mut t = t;
        let mut out = Vec::new();
        while let Type::Refined(p, r) = self.get(t) {
            out.push(r);
            t = p;
        }
        out.reverse();
        (t, out)
    }

    /// Whether a path or a member seen through one occurs anywhere in the type.
    #[inline]
    pub fn has_paths(&self, t: TypeId) -> bool {
        self.flag(t) & PATHS_BIT != 0
    }

    /// Whether a path in the type is over one of `terms`.
    pub fn names_term(&self, t: TypeId, terms: &[SymId]) -> bool {
        if !self.has_paths(t) {
            return false;
        }
        let list = |l: TList| self.items(l).iter().any(|&x| self.names_term(x, terms));
        match self.get(t) {
            Type::Term(s) => terms.contains(&s),
            Type::Select(p, s) => terms.contains(&s) || self.names_term(p, terms),
            Type::Member(p, _) => self.names_term(p, terms),
            Type::AppMember(m, args) => self.names_term(m, terms) || list(args),
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) | Type::Alias(_, args) => list(args),
            Type::Lambda(ps, b) | Type::Poly(ps, b) => list(ps) || self.names_term(b, terms),
            Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => self.names_term(a, terms) || self.names_term(b, terms),
            Type::Refined(p, r) => {
                self.names_term(p, terms)
                    || match self.refinement(r) {
                        Refinement::Alias(_, x) => self.names_term(x, terms),
                        Refinement::Bounds(_, lo, hi) => self.names_term(lo, terms) || self.names_term(hi, terms),
                        Refinement::Term(_, _, l) => list(l),
                        Refinement::Val(_, _, x) => self.names_term(x, terms),
                    }
            }
            Type::Match(s, m) => {
                let info = self.match_info(m);
                self.names_term(s, terms) || self.names_term(info.bound, terms) || info.cases.iter().any(|c| self.names_term(c.pattern, terms) || self.names_term(c.body, terms))
            }
            _ => false,
        }
    }

    /// Whether the type is a path: a singleton type or `C.this`.
    #[inline]
    pub fn is_path(&self, t: TypeId) -> bool {
        matches!(self.get(t), Type::This(_) | Type::Term(_) | Type::Select(..))
    }

    /// Whether a wildcard argument occurs anywhere in the type.
    #[inline]
    pub fn has_wild(&self, t: TypeId) -> bool {
        self.flag(t) & WILD_BIT != 0
    }

    /// The wildcard `? >: lo <: hi`: without bounds the plain one, with equal bounds the type.
    pub fn bounded_wild(&self, lo: TypeId, hi: TypeId) -> TypeId {
        if lo == hi {
            return lo;
        }
        if lo == NOTHING && hi == ANY {
            return WILD;
        }
        self.mk(Type::BoundedWild(lo, hi))
    }

    /// The bounds of a wildcard argument, plain or bounded.
    pub fn wild_bounds(&self, t: TypeId) -> Option<(TypeId, TypeId)> {
        match self.get(t) {
            Type::Wild => Some((NOTHING, ANY)),
            Type::BoundedWild(lo, hi) => Some((lo, hi)),
            _ => None,
        }
    }

    pub fn is_wild(&self, t: TypeId) -> bool {
        matches!(self.get(t), Type::Wild | Type::BoundedWild(..))
    }

    /// The type at `id`, read in this thread's view (`view`: checked in the assertion-enabled
    /// builds).
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get(&self, id: TypeId) -> Type {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Types, id.0);
        self.entry(id)
    }

    /// The type at `id` wherever it lies: the raw traversal of the import, the export and the
    /// measurement, which read foreign entries by design. Private to the store and its checks
    /// (`view`), as the other raw accessors are; the merge traverses through `rebuild` and
    /// `parts`.
    #[inline]
    fn entry(&self, id: TypeId) -> Type {
        *self.types.get(id.idx())
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn flag(&self, t: TypeId) -> u8 {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Types, t.0);
        self.entry_flag(t)
    }

    #[inline]
    fn entry_flag(&self, t: TypeId) -> u8 {
        self.flags[t.idx()]
    }


    #[cfg_attr(debug_assertions, track_caller)]
    pub fn list(&self, items: &[TypeId]) -> TList {
        let h = hash_of(items);
        let find = || self.list_index.find(h, |i| &**self.lists.get(i as usize) == items);
        if let Some(l) = find() {
            return TList(l);
        }
        let Ok(held) = self.insert_lock() else { return self.list_apart(h, items) };
        if held.is_some() {
            if let Some(l) = find() {
                return TList(l);
            }
        }
        let l = self.lists.push(items.into()) as u32;
        self.list_index.insert(h, l);
        TList(l)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn items(&self, l: TList) -> &[TypeId] {
        #[cfg(debug_assertions)]
        view::read(self, Sub::Lists, l.0);
        self.entry_items(l)
    }

    #[inline]
    fn entry_items(&self, l: TList) -> &[TypeId] {
        self.lists.get(l.idx())
    }

    pub fn class(&self, c: ClassId, args: &[TypeId]) -> TypeId {
        let l = self.list(args);
        self.mk(Type::Class(c, l))
    }

    /// The parts of every union the store holds, which the JVM's erasure of a union reads the
    /// base classes of (`complete_erased_jar_classes`).
    pub fn union_parts(&self) -> Vec<TypeId> {
        let mut out = Vec::new();
        for i in 0..self.types.len() {
            if let Type::Union(a, b) = *self.types.get(i) {
                out.push(a);
                out.push(b);
            }
        }
        out
    }

    /// scalac's `tupleArity` of a `*:` chain, which decides its erasure: its heads and the
    /// elements of the tuple class it ends in, of both sides of a union or an intersection that
    /// agree, `None` where it ends in anything else (a type parameter, `Tuple`, the empty tuple
    /// as a signature names it through scalac's alias). `tuple_arity` gives the elements of a
    /// tuple class, `cons` is `*:`.
    pub fn cons_arity(&self, t: TypeId, cons: ClassId, tuple_arity: &impl Fn(ClassId) -> Option<usize>) -> Option<usize> {
        let mut heads = 0;
        let mut t = t;
        loop {
            match self.get(t) {
                Type::Class(c, args) if c == cons => match *self.items(args) {
                    [_, tail] => {
                        heads += 1;
                        t = tail;
                    }
                    _ => return None,
                },
                Type::Class(c, _) => return tuple_arity(c).map(|n| heads + n),
                Type::Union(a, b) | Type::Inter(a, b) => {
                    let n = self.cons_arity(a, cons, tuple_arity)?;
                    return (self.cons_arity(b, cons, tuple_arity)? == n).then_some(heads + n);
                }
                _ => return None,
            }
        }
    }

    pub fn param(&self, p: TParamId) -> TypeId {
        self.mk(Type::Param(p))
    }

    pub fn union(&self, a: TypeId, b: TypeId) -> TypeId {
        if a == b || b == NOTHING {
            return a;
        }
        if a == NOTHING {
            return b;
        }
        if a == ERROR || b == ERROR {
            return ERROR;
        }
        self.mk(Type::Union(a, b))
    }

    pub fn inter(&self, a: TypeId, b: TypeId) -> TypeId {
        if a == b || b == ANY {
            return a;
        }
        if a == ANY {
            return b;
        }
        if a == ERROR || b == ERROR {
            return ERROR;
        }
        self.mk(Type::Inter(a, b))
    }

    /// Applies a type constructor (`Ctor`, higher-kinded `Param`, `Lambda` or `Var`) to arguments.
    /// A lambda applied to itself (`type L[F[_]] = F[F]` at `L[L]`) would expand without end;
    /// the nesting is bounded, and what exceeds it is an error.
    pub fn apply_ctor(&self, ctor: TypeId, args: &[TypeId]) -> TypeId {
        if APPLY_DEPTH.with(|d| d.get()) >= 64 {
            return ERROR;
        }
        APPLY_DEPTH.with(|d| d.set(d.get() + 1));
        let t = self.apply_ctor_inner(ctor, args);
        APPLY_DEPTH.with(|d| d.set(d.get() - 1));
        t
    }

    fn apply_ctor_inner(&self, ctor: TypeId, args: &[TypeId]) -> TypeId {
        match self.get(ctor) {
            Type::Ctor(c) => self.class(c, args),
            Type::Param(p) => {
                let l = self.list(args);
                self.mk(Type::AppParam(p, l))
            }
            Type::Var(v) => {
                let l = self.list(args);
                self.mk(Type::AppVar(v, l))
            }
            Type::Alias(a, given) => {
                let mut all = self.items(given).to_vec();
                all.extend_from_slice(args);
                let l = self.list(&all);
                self.mk(Type::Alias(a, l))
            }
            Type::Lambda(params, body) => {
                let ps: Vec<TypeId> = self.items(params).to_vec();
                let mut subst: Subst = Vec::with_capacity(ps.len());
                for (p, &a) in ps.iter().zip(args) {
                    if let Type::Param(id) = self.get(*p) {
                        subst.push((id, a));
                    }
                }
                self.subst(body, &subst)
            }
            Type::Member(..) | Type::Decl(_) => {
                let l = self.list(args);
                self.mk(Type::AppMember(ctor, l))
            }
            _ => ERROR,
        }
    }

    /// Whether a higher-kinded type parameter is applied anywhere inside `t`.
    pub fn contains_app_param(&self, t: TypeId) -> bool {
        self.flag(t) & APP_PARAM_BIT != 0
    }

    /// Whether an unresolved type occurs anywhere inside `t`.
    pub fn contains_error(&self, t: TypeId) -> bool {
        match self.get(t) {
            Type::Error | Type::Blocked(_) => true,
            Type::Any
            | Type::Nothing
            | Type::Ctor(_)
            | Type::Var(_)
            | Type::Param(_)
            | Type::Lit(_)
            | Type::Wild
            | Type::This(_)
            | Type::Term(_)
            | Type::Decl(_) => false,
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) => {
                self.items(args).iter().any(|&a| self.contains_error(a))
            }
            Type::AppMember(m, args) => self.contains_error(m) || self.items(args).iter().any(|&a| self.contains_error(a)),
            Type::Lambda(_, body) => self.contains_error(body),
            Type::Poly(ps, body) => self.contains_error(body) || self.poly_bounds(ps).iter().any(|&a| self.contains_error(a)),
            Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => self.contains_error(a) || self.contains_error(b),
            Type::Select(p, _) | Type::Member(p, _) => self.contains_error(p),
            Type::Refined(p, r) => self.contains_error(p) || self.refinement_types(r).into_iter().any(|a| self.contains_error(a)),
            Type::Match(s, m) => self.contains_error(s) || self.match_parts(m).collect::<Vec<_>>().into_iter().any(|a| self.contains_error(a)),
            Type::Alias(_, args) => self.items(args).iter().any(|&a| self.contains_error(a)),
        }
    }

    pub fn subst_list(&self, l: TList, s: &Subst) -> TList {
        if l == EMPTY_LIST {
            return l;
        }
        let n = self.items(l).len();
        let mut out: Option<Vec<TypeId>> = None;
        for i in 0..n {
            let item = self.items(l)[i];
            let new = self.subst(item, s);
            if new != item && out.is_none() {
                out = Some(self.items(l).to_vec());
            }
            if let Some(o) = &mut out {
                o[i] = new;
            }
        }
        match out {
            Some(o) => self.list(&o),
            None => l,
        }
    }

    pub fn subst(&self, t: TypeId, s: &Subst) -> TypeId {
        if s.is_empty() {
            return t;
        }
        match self.get(t) {
            Type::Any
            | Type::Nothing
            | Type::Error
            | Type::Ctor(_)
            | Type::Var(_)
            | Type::Lit(_)
            | Type::Wild
            | Type::Blocked(_)
            | Type::This(_)
            | Type::Term(_)
            | Type::Decl(_) => t,
            Type::Param(p) => s.iter().find(|(q, _)| *q == p).map_or(t, |&(_, r)| r),
            Type::Select(p, sym) => {
                let np = self.subst(p, s);
                if np == p {
                    t
                } else {
                    self.mk(Type::Select(np, sym))
                }
            }
            Type::Member(p, name) => {
                let np = self.subst(p, s);
                if np == p {
                    t
                } else {
                    self.mk(Type::Member(np, name))
                }
            }
            Type::AppMember(m, args) => {
                let (nm, n) = (self.subst(m, s), self.subst_list(args, s));
                if nm == m && n == args {
                    t
                } else {
                    let items: Vec<TypeId> = self.items(n).to_vec();
                    self.apply_ctor(nm, &items)
                }
            }
            Type::Refined(p, r) => {
                let np = self.subst(p, s);
                let nr = self.subst_refinement(r, s);
                if np == p && nr == r {
                    t
                } else {
                    self.mk(Type::Refined(np, nr))
                }
            }
            Type::Class(c, args) => {
                let n = self.subst_list(args, s);
                if n == args {
                    t
                } else {
                    self.mk(Type::Class(c, n))
                }
            }
            Type::AppParam(p, args) => {
                let n = self.subst_list(args, s);
                match s.iter().find(|(q, _)| *q == p) {
                    Some(&(_, ctor)) => {
                        let items: Vec<TypeId> = self.items(n).to_vec();
                        self.apply_ctor(ctor, &items)
                    }
                    None if n == args => t,
                    None => self.mk(Type::AppParam(p, n)),
                }
            }
            Type::AppVar(v, args) => {
                let n = self.subst_list(args, s);
                if n == args {
                    t
                } else {
                    self.mk(Type::AppVar(v, n))
                }
            }
            Type::Lambda(ps, body) => {
                let b = self.subst(body, s);
                if b == body {
                    t
                } else {
                    self.mk(Type::Lambda(ps, b))
                }
            }
            Type::Poly(ps, body) => {
                let b = self.subst(body, s);
                let items = self.items(ps).to_vec();
                let k = items.len() / 3;
                let mapped: Vec<TypeId> = items.iter().enumerate().map(|(i, &x)| if i < k { x } else { self.subst(x, s) }).collect();
                let nps = if mapped == items { ps } else { self.list(&mapped) };
                if b == body && nps == ps {
                    t
                } else {
                    self.mk(Type::Poly(nps, b))
                }
            }
            Type::Union(a, b) => {
                let (na, nb) = (self.subst(a, s), self.subst(b, s));
                if na == a && nb == b {
                    t
                } else {
                    self.union(na, nb)
                }
            }
            Type::Inter(a, b) => {
                let (na, nb) = (self.subst(a, s), self.subst(b, s));
                if na == a && nb == b {
                    t
                } else {
                    self.inter(na, nb)
                }
            }
            Type::BoundedWild(lo, hi) => {
                let (nl, nh) = (self.subst(lo, s), self.subst(hi, s));
                if nl == lo && nh == hi {
                    t
                } else {
                    self.bounded_wild(nl, nh)
                }
            }
            Type::Alias(a, args) => {
                let n = self.subst_list(args, s);
                if n == args {
                    t
                } else {
                    self.mk(Type::Alias(a, n))
                }
            }
            Type::Match(scrut, m) => {
                let ns = self.subst(scrut, s);
                let (cases, bound) = {
                    let info = self.match_info(m);
                    (info.cases.to_vec(), info.bound)
                };
                let nb = self.subst(bound, s);
                let mut changed = ns != scrut || nb != bound;
                let ncases: Vec<MatchCase> = cases
                    .iter()
                    .map(|c| {
                        let (p, b) = (self.subst(c.pattern, s), self.subst(c.body, s));
                        changed |= p != c.pattern || b != c.body;
                        MatchCase { binders: c.binders, pattern: p, body: b }
                    })
                    .collect();
                if changed {
                    self.match_type(ns, &ncases, nb)
                } else {
                    t
                }
            }
        }
    }

    /// `t` with each type of `pairs` replaced by its partner, wherever it stands as a part.
    pub fn replace(&self, t: TypeId, pairs: &[(TypeId, TypeId)]) -> TypeId {
        if let Some(&(_, to)) = pairs.iter().find(|&&(from, _)| from == t) {
            return to;
        }
        if !self.has_paths(t) {
            return t;
        }
        let list = |ts: &Self, l: TList| {
            let items: Vec<TypeId> = ts.items(l).to_vec();
            let out: Vec<TypeId> = items.iter().map(|&a| ts.replace(a, pairs)).collect();
            if out == items { l } else { ts.list(&out) }
        };
        match self.get(t) {
            Type::Class(c, args) => {
                let n = list(self, args);
                if n == args { t } else { self.mk(Type::Class(c, n)) }
            }
            Type::AppParam(p, args) => {
                let n = list(self, args);
                if n == args { t } else { self.mk(Type::AppParam(p, n)) }
            }
            Type::AppMember(m, args) => {
                let (nm, n) = (self.replace(m, pairs), list(self, args));
                if nm == m && n == args { t } else { let items = self.items(n).to_vec(); self.apply_ctor(nm, &items) }
            }
            Type::Alias(a, args) => {
                let n = list(self, args);
                if n == args { t } else { self.mk(Type::Alias(a, n)) }
            }
            Type::Lambda(ps, body) => {
                let b = self.replace(body, pairs);
                if b == body { t } else { self.mk(Type::Lambda(ps, b)) }
            }
            Type::Union(a, b) => {
                let (na, nb) = (self.replace(a, pairs), self.replace(b, pairs));
                if na == a && nb == b { t } else { self.union(na, nb) }
            }
            Type::Inter(a, b) => {
                let (na, nb) = (self.replace(a, pairs), self.replace(b, pairs));
                if na == a && nb == b { t } else { self.inter(na, nb) }
            }
            Type::BoundedWild(lo, hi) => {
                let (nl, nh) = (self.replace(lo, pairs), self.replace(hi, pairs));
                if nl == lo && nh == hi { t } else { self.bounded_wild(nl, nh) }
            }
            _ => t,
        }
    }

    fn subst_refinement(&self, r: RefineId, s: &Subst) -> RefineId {
        let refined = match self.refinement(r) {
            Refinement::Alias(n, t) => Refinement::Alias(n, self.subst(t, s)),
            Refinement::Bounds(n, lo, hi) => Refinement::Bounds(n, self.subst(lo, s), self.subst(hi, s)),
            Refinement::Val(n, sym, t) => Refinement::Val(n, sym, self.subst(t, s)),
            Refinement::Term(n, sym, l) => Refinement::Term(n, sym, self.subst_list(l, s)),
        };
        self.refine(refined)
    }

    /// Whether the type mentions any inference variable.
    #[inline]
    pub fn has_vars(&self, t: TypeId) -> bool {
        self.flag(t) & VARS_BIT != 0
    }
}

/// The six sub-stores of `TypeStore`, each with an index of its own and, at the fork, a bound of
/// its own: a newly grown base entry of any of them reused after an
/// import would give one type two keys.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sub {
    Types,
    Lists,
    Lits,
    Blocked,
    Refinements,
    Matches,
}

impl Sub {
    pub const ALL: [Sub; 6] = [Sub::Types, Sub::Lists, Sub::Lits, Sub::Blocked, Sub::Refinements, Sub::Matches];

    pub fn name(self) -> &'static str {
        match self {
            Sub::Types => "types",
            Sub::Lists => "lists",
            Sub::Lits => "literals",
            Sub::Blocked => "blocked descriptions",
            Sub::Refinements => "refinements",
            Sub::Matches => "match records",
        }
    }
}

/// What the store is from the fork to the merge with the overlays on.
/// Every sub-store's entries live in one mapping of the system's pages (`SlabVec::map_regions`):
/// the base's at their ids from 0, at most `LOCAL_BASE` of them, and worker `k`'s overlay at
/// `worker_base(k)`, at most `WORKER_SPAN - 1`, each written by its worker alone at its offset
/// from there. An id is a handle whose value says where it is, so `get` reads any id without
/// asking whose it is, and the plain path's reads cost what they did. A worker's canonical
/// namespace is the base below each bound and its own overlay: the base's index as the fork left
/// it is frozen (its probes find ids below the bound alone), and what the base grows by after the
/// fork, under the loader's lock, is indexed apart (`late`), by the position less the bound.
/// Nothing in an overlay is found by another worker's probe: an id outside a worker's namespace
/// is foreign, read as it is, and made the reader's by `import`.
struct Apart {
    workers: Vec<Box<Overlay>>,
    /// Per sub-store, the base's length at the fork.
    bounds: [u32; 6],
    /// Per sub-store, the base's entries made after the fork, by their id less the bound.
    late: [Table; 6],
    /// Per sub-store, the fork's index taken out of the store's probe (`TEQ_WORKERS_TYPES=1`):
    /// every probe then misses there and the overlay's path probes it, which counts the base's
    /// hits below the bound.
    frozen: Option<[Table; 6]>,
    /// The provenance of the unshared paths translations made: each copy's origin, the
    /// original path it stands for, and the base's one representative of each origin
    /// (`TypeStore::unshared_copy`).
    origins: SharedMap<TypeId, TypeId>,
    base_reps: SharedMap<TypeId, TypeId>,
    /// The measurement's exports into the base, every worker's (`TypeStore::note_export`).
    export_shadow: std::sync::Mutex<Shadow>,
}

impl Apart {
    fn held(&self) -> usize {
        self.workers.iter().map(|o| o.held()).sum::<usize>() + self.late.iter().map(|t| t.held()).sum::<usize>()
    }
}

/// A worker's overlay: its sub-stores' regions in the store's
/// mappings, its indexes by offset, and what the owner alone keeps (the import and export memos,
/// the counts).
pub struct Overlay {
    worker: usize,
    base: u32,
    /// The store the overlay is in, which outlives it (it owns it).
    store: *const TypeStore,
    types: Region<Type>,
    flags: Region<u8>,
    lists: Region<Box<[TypeId]>>,
    lits: Region<LitVal>,
    blocked: Region<Box<str>>,
    refinements: Region<Refinement>,
    matches: Region<MatchInfo>,
    index: [Table; 6],
    own: UnsafeCell<OwnerSide>,
}

unsafe impl Sync for Overlay {}
unsafe impl Send for Overlay {}

/// The owner's side of an overlay, touched by its worker's thread alone.
#[derive(Default)]
pub struct OwnerSide {
    /// A foreign type's id in this overlay's namespace, by the foreign id: immutable structural
    /// translation alone (an id never changes what it names before the merge).
    pub imported: crate::intern::FxMap<u32, TypeId>,
    /// An overlay type's id in the base, by the overlay id, for this worker's exports.
    pub exported: crate::intern::FxMap<u32, TypeId>,
    /// The measurement's imports into this worker's namespace, which make their entries in
    /// the shadow (`ShadowNs`).
    pub import_shadow: Shadow,
    /// This overlay's one representative of each unshared path's origin (`unshared_copy`).
    pub unshared_reps: crate::intern::FxMap<u32, TypeId>,
    /// The peer class records and the signatures already read once (`note_record`'s memo
    /// per record).
    pub records_seen: crate::intern::FxMap<(u8, u32), ()>,
    /// The base's entries made after the fork that the worker's probes met, per sub-store: in
    /// the frozen namespace each is made again in the overlay once.
    pub late_met: [crate::intern::FxMap<u32, ()>; 6],
    pub counts: OverlayCounts,
}

/// What a worker's views of the shared records and its signatures' copies held at the end of its
/// work (`TEQ_WORKERS_TYPES=1`, `TypeStore::count_views`).
#[derive(Default, Clone, Copy)]
pub struct ViewsHeld {
    /// Signatures, classes, type parameters, aliases.
    pub records: [crate::arena::ViewCounts; 4],
}

/// What a worker's overlay counted (`TEQ_WORKERS_TYPES=1`).
#[derive(Default, Clone)]
pub struct OverlayCounts {
    /// Per sub-store: a hit in the base below the bound, a hit in the base above it (counted
    /// as the miss it is in the frozen namespace), a hit in the overlay, an insert.
    pub probes: [[u64; 4]; 6],
    /// Per route (`Route`), what its imports or exports did.
    pub routes: [RouteCounts; Route::COUNT],
    /// The signature reads the typer hooked (`Worker::completed_sig_hooked`).
    pub sigs: SigCounts,
    pub views: ViewsHeld,
}

/// What a worker's hooked signature reads did (`TEQ_WORKERS_TYPES=1`).
#[derive(Default, Clone, Copy)]
pub struct SigCounts {
    /// The reads hooked with the views on.
    pub reads: u64,
}

/// The routes a type crosses between stores by, the ones counted here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route {
    /// A symbol's signature read (`sig_of`, `sig_arc`): the base's past the bound, a peer's.
    Signature,
    /// An expression's recorded type read from another worker's body.
    ExprType,
    /// A class record of another worker's read (its parents, base types, constructor, self).
    PeerClass,
    /// A signature published into a shared symbol (`publish_sig`): an export.
    Publish,
    /// A class check's writes into a shared class's info: an export.
    ClassCheck,
}

impl Route {
    pub const COUNT: usize = 5;
    pub const ALL: [Route; Route::COUNT] = [Route::Signature, Route::ExprType, Route::PeerClass, Route::Publish, Route::ClassCheck];

    pub fn name(self) -> &'static str {
        match self {
            Route::Signature => "a signature read (sig_of, sig_arc)",
            Route::ExprType => "a peer's recorded expression type",
            Route::PeerClass => "a peer's class record",
            Route::Publish => "a signature published (publish_sig)",
            Route::ClassCheck => "a class check's writes",
        }
    }
}

/// What the imports or exports of one route did.
#[derive(Default, Clone, Copy)]
pub struct RouteCounts {
    /// The records the route read or wrote (a signature, a class record, an entry), and of them
    /// the ones read again, which the memo per record answers.
    pub records: u64,
    pub records_again: u64,
    /// The types handed over, and of them the foreign ones (outside the reader's namespace, or
    /// outside the base for an export).
    pub types: u64,
    pub foreign: u64,
    /// The foreign ones by where they are: the base above the bound, another worker's overlay.
    pub from_base: u64,
    pub from_peer: u64,
    /// The types walked (each foreign type once per walk that met it unmemoised), the parts
    /// followed, the memo's hits, the entries the walks made.
    pub nodes: u64,
    pub edges: u64,
    pub memo_hits: u64,
    /// The entries the route's translations appended, counted where each is made: types, and
    /// the other sub-stores' entries.
    pub made: u64,
    pub made_other: u64,
    /// The translations whose result is another id than their source: every import in the
    /// frozen namespace, a reconciled one in the shared.
    pub moved: u64,
}

thread_local! {
    /// The overlay the worker on this thread makes its types in, between `enter_overlay` and
    /// `leave_overlay`.
    static HERE: Cell<*const Overlay> = const { Cell::new(std::ptr::null()) };
}

/// Whether this thread makes its types in a worker's overlay.
pub fn in_overlay() -> bool {
    HERE.with(|h| !h.get().is_null())
}

/// The overlay this thread makes its types in, how deeply `apply_ctor` is nested and whether a
/// translation's route is under way, for `TEQ_SESSION_INVENTORY`.
pub fn inventory() -> Vec<(&'static str, usize)> {
    vec![
        ("in an overlay", in_overlay() as usize),
        ("apply depth", APPLY_DEPTH.with(|d| d.get() as usize)),
        ("route under way", ROUTE.with(|r| r.get().is_some() as usize)),
    ]
}

/// Where a probe of the overlay's path found the entry.
#[derive(Clone, Copy)]
pub enum Probe {
    BaseBelow = 0,
    BaseAbove = 1,
    Own = 2,
    Made = 3,
}

impl Overlay {
    fn new(store: &TypeStore, worker: usize) -> Overlay {
        let base = worker_base(worker);
        let (at, cap) = (base as usize, WORKER_SPAN as usize);
        Overlay {
            worker,
            base,
            store: store as *const TypeStore,
            types: store.types.region(at, cap),
            flags: store.flags.region(at, cap),
            lists: store.lists.region(at, cap),
            lits: store.lits.region(at, cap),
            blocked: store.blocked.region(at, cap),
            refinements: store.refinements.region(at, cap),
            matches: store.matches.region(at, cap),
            index: std::array::from_fn(|_| Table::with_capacity(16)),
            own: UnsafeCell::new(OwnerSide::default()),
        }
    }

    /// The owner's side: its worker's thread alone calls this.
    #[allow(clippy::mut_from_ref)]
    pub fn owner(&self) -> &mut OwnerSide {
        unsafe { &mut *self.own.get() }
    }

    #[inline]
    fn count(&self, sub: Sub, p: Probe) {
        if crate::measure::types_wanted() {
            self.owner().counts.probes[sub as usize][p as usize] += 1;
        }
    }

    /// The first id of the overlay's region.
    #[cfg(test)]
    pub fn base_id(&self) -> u32 {
        self.base
    }

    /// How many entries of each sub-store the overlay holds.
    pub fn lens(&self) -> [usize; 6] {
        [self.types.len(), self.lists.len(), self.lits.len(), self.blocked.len(), self.refinements.len(), self.matches.len()]
    }

    fn push_type(&self, t: Type, flags: u8) -> TypeId {
        let i = self.types.push(t);
        self.flags.push(flags);
        TypeId(self.base + i as u32)
    }

    /// The bytes the overlay holds: its entries and their boxed payloads, its indexes, its
    /// provenance and the owner's memos.
    pub fn held(&self) -> usize {
        let payload = (0..self.lists.len()).map(|i| self.lists.get(i).len() * 4).sum::<usize>()
            + (0..self.blocked.len()).map(|i| self.blocked.get(i).len()).sum::<usize>()
            + (0..self.matches.len()).map(|i| self.matches.get(i).cases.len() * std::mem::size_of::<MatchCase>()).sum::<usize>();
        let own = unsafe { &*self.own.get() };
        self.types.held()
            + self.flags.held()
            + self.lists.held()
            + self.lits.held()
            + self.blocked.held()
            + self.refinements.held()
            + self.matches.held()
            + payload
            + self.index.iter().map(|t| t.held()).sum::<usize>()
            + (own.imported.capacity() + own.exported.capacity()) * 12
            + own.import_shadow.held()
            + own.records_seen.capacity() * 16
    }
}

/// What `TEQ_TYPE_OVERLAYS` asks of a forked build: unset or `shared`, the overlays over the
/// shared base; `off`, the store shared by the workers as it was, the
/// control; `None` for any other value, which the driver refuses (`overlays_switch_error`).
fn overlays_switch() -> Option<bool> {
    match std::env::var("TEQ_TYPE_OVERLAYS").ok().as_deref() {
        None | Some("shared") => Some(true),
        Some("off") => Some(false),
        Some(_) => None,
    }
}

/// The message for a value of `TEQ_TYPE_OVERLAYS` no build takes.
pub fn overlays_switch_error() -> Option<String> {
    overlays_switch().is_none().then(|| "TEQ_TYPE_OVERLAYS is shared (the default, the overlays) or off (the store shared by the workers)".to_string())
}

/// The mappings a fork's overlays live in, one per sub-store (`TypeStore::overlay_mappings`).
pub struct OverlayMappings(Vec<crate::shared::Mapping>);

/// The body phase's workers make their types in overlays of their own,
/// read by every worker through the ids, unless `TEQ_TYPE_OVERLAYS=off`, where the store is shared
/// as before.
/// What a build says where the system gave no mapping for the overlays (`Typer::run`).
pub const MAPPING_REFUSED: &str = "no memory mapping for the type store's overlays; one worker types the build";

static REFUSED: AtomicBool = AtomicBool::new(false);

/// The overlays' mappings were refused, which the build's answer tells (`take_mapping_refused`).
pub fn note_mapping_refused() {
    REFUSED.store(true, Ordering::Relaxed);
}

/// Whether a build since the last ask had its mappings refused: a session's answer carries the
/// note (`notes`), which a resident's stderr, kept for its end, would not bring to the user.
pub fn take_mapping_refused() -> bool {
    REFUSED.swap(false, Ordering::Relaxed)
}

pub fn overlays_wanted() -> bool {
    static STATE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let on = overlays_switch() == Some(true);
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

impl TypeStore {
    /// The types a type is made of, in the order its fields name them: a list's items, a
    /// refinement's and a match record's types (a case's binders, pattern and body, then the
    /// bound).
    pub fn parts(&self, t: TypeId, out: &mut Vec<TypeId>) {
        match self.entry(t) {
            Type::Class(_, args) | Type::AppParam(_, args) | Type::AppVar(_, args) | Type::Alias(_, args) => out.extend_from_slice(self.entry_items(args)),
            Type::Select(p, _) | Type::Member(p, _) => out.push(p),
            Type::Lambda(ps, b) | Type::Poly(ps, b) => {
                out.extend_from_slice(self.entry_items(ps));
                out.push(b);
            }
            Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => out.extend([a, b]),
            Type::AppMember(m, args) => {
                out.push(m);
                out.extend_from_slice(self.entry_items(args));
            }
            Type::Refined(p, r) => {
                out.push(p);
                match self.entry_refinement(r) {
                    Refinement::Alias(_, x) | Refinement::Val(_, _, x) => out.push(x),
                    Refinement::Bounds(_, lo, hi) => out.extend([lo, hi]),
                    Refinement::Term(_, _, l) => out.extend_from_slice(self.entry_items(l)),
                }
            }
            Type::Match(s, m) => {
                out.push(s);
                let info = self.entry_match(m);
                for c in info.cases.iter() {
                    out.extend_from_slice(self.entry_items(c.binders));
                    out.extend([c.pattern, c.body]);
                }
                out.push(info.bound);
            }
            _ => {}
        }
    }

    /// The marks of the classes the workers made, which their ids key (`mark_op_class`).
    #[inline]
    fn op_local(&self) -> &SharedMap<ClassId, ()> {
        unsafe { &*self.op_local.get() }
    }

    #[inline]
    fn path_local(&self) -> &SharedMap<ClassId, ()> {
        unsafe { &*self.path_local.get() }
    }

    /// The marks of the classes the workers made, carried to the ids the workers' merge gives
    /// them (`new_class`) before it makes a type over them again, a type's flags reading the
    /// marks once, when it is made; and the workers' ids of them forgotten, since a later build's
    /// workers give the same ids to other classes. The owner's, at the workers' merge, no worker
    /// running.
    #[cold]
    pub fn carry_local_marks(&self, new_class: impl Fn(ClassId) -> ClassId) {
        let (op, path) = unsafe { (std::mem::take(&mut *self.op_local.get()), std::mem::take(&mut *self.path_local.get())) };
        for (local, dense, sparse) in [(op, &self.op_classes, self.op_local()), (path, &self.path_classes, self.path_local())] {
            for &c in local.keys() {
                let moved = new_class(c);
                if moved.0 < LOCAL_BASE {
                    let _held = self.serial.hold();
                    Self::mark(dense, moved);
                } else {
                    sparse.insert(moved, ());
                }
            }
        }
    }

    /// A copy of the base as it stands, for the workers' merge's order check:
    /// the six sub-stores' entries in their order with the types'
    /// flags, their indexes made again as the join makes the base's later growth's
    /// (`join_overlays`), the class marks; no overlay, no provenance of an unshared path. The
    /// check's promotion interns into it, apart from this store, and reads the types made after the
    /// fork from this one (`merge::Remap::source`). The owner's, no worker running.
    #[cold]
    pub fn shadow(&self) -> TypeStore {
        let s = TypeStore {
            serial: Serial::new(true, crate::measure::Wait::TypeStore),
            types: SlabVec::with_capacity(self.types.len() + 16384),
            type_index: Table::with_capacity(self.types.len() + 16384),
            flags: SlabVec::with_capacity(self.types.len() + 16384),
            lists: SlabVec::with_capacity(self.lists.len() + 4096),
            list_index: Table::with_capacity(self.lists.len() + 4096),
            lits: SlabVec::with_capacity(self.lits.len() + 16),
            lit_index: Table::with_capacity(self.lits.len() + 16),
            blocked: SlabVec::with_capacity(self.blocked.len() + 16),
            blocked_index: Table::with_capacity(self.blocked.len() + 16),
            refinements: SlabVec::with_capacity(self.refinements.len() + 16),
            refine_index: Table::with_capacity(self.refinements.len() + 16),
            term_refinements: AtomicBool::new(self.term_refinements.load(Ordering::Relaxed)),
            matches: SlabVec::with_capacity(self.matches.len() + 16),
            match_index: Table::with_capacity(self.matches.len() + 16),
            op_classes: SlabVec::with_capacity(self.op_classes.len()),
            path_classes: SlabVec::with_capacity(self.path_classes.len()),
            op_local: UnsafeCell::new(SharedMap::new()),
            path_local: UnsafeCell::new(SharedMap::new()),
            apart: UnsafeCell::new(None),
            overlaid: AtomicBool::new(false),
            reclaimed: AtomicBool::new(false),
        };
        for i in 0..self.types.len() {
            let t = *self.types.get(i);
            s.types.push(t);
            s.flags.push(*self.flags.get(i));
            if !self.entry_unshared(TypeId(i as u32)) {
                s.type_index.insert(hash_of(&t), i as u32);
            }
        }
        for i in 0..self.lists.len() {
            let l = self.lists.get(i).clone();
            s.list_index.insert(hash_of(&*l), i as u32);
            s.lists.push(l);
        }
        for i in 0..self.lits.len() {
            let l = self.lits.get(i).clone();
            s.lit_index.insert(hash_of(&l), i as u32);
            s.lits.push(l);
        }
        for i in 0..self.blocked.len() {
            let b = self.blocked.get(i).clone();
            s.blocked_index.insert(hash_of(&*b), i as u32);
            s.blocked.push(b);
        }
        for i in 0..self.refinements.len() {
            let r = self.refinements.get(i).clone();
            s.refine_index.insert(hash_of(&r), i as u32);
            s.refinements.push(r);
        }
        for i in 0..self.matches.len() {
            let m = self.matches.get(i).clone();
            s.match_index.insert(hash_of(&m), i as u32);
            s.matches.push(m);
        }
        for (from, to) in [(&self.op_classes, &s.op_classes), (&self.path_classes, &s.path_classes)] {
            for i in 0..from.len() {
                to.push(AtomicBool::new(from.get(i).load(Ordering::Relaxed)));
            }
        }
        for (from, to) in [(self.op_local(), s.op_local()), (self.path_local(), s.path_local())] {
            for &c in from.keys() {
                to.insert(c, ());
            }
        }
        s
    }

    fn apart_ref(&self) -> Option<&Apart> {
        unsafe { (*self.apart.get()).as_deref() }
    }

    /// Whether the body phase's workers make their types in overlays.
    pub fn overlays_on(&self) -> bool {
        self.overlaid.load(Ordering::Relaxed)
    }

    /// The overlay of the worker this thread types for, between `enter_overlay` and
    /// `leave_overlay`.
    #[inline]
    fn overlay_of_thread(&self) -> Option<&Overlay> {
        if !self.overlaid.load(Ordering::Relaxed) {
            return None;
        }
        let here = HERE.with(|h| h.get());
        (!here.is_null()).then(|| unsafe { &*here })
    }

    /// The overlay this thread makes its types in: its worker's, not while it holds the
    /// loader's lock, whose types are the base's.
    #[inline]
    fn overlay_here(&self) -> Option<&Overlay> {
        self.overlay_of_thread().filter(|_| lock_depth() == 0)
    }

    /// The overlays, for the merge and the measurement.
    pub fn overlays(&self) -> &[Box<Overlay>] {
        self.apart_ref().map_or(&[], |a| &a.workers)
    }

    /// The base's length of a sub-store at the fork, with the overlays on.
    pub fn bound(&self, sub: Sub) -> u32 {
        self.apart_ref().map_or(u32::MAX, |a| a.bounds[sub as usize])
    }

    /// The store's part of the fork for `workers` workers: the overlays in `mappings`
    /// (`overlay_mappings`, taken when `TEQ_TYPE_OVERLAYS` asks for them), the routes' notes and the
    /// memory's first sample. Whether the overlays are on, whose signature reads the typer then
    /// hooks (`Typer::sig_read_noted`).
    #[cold]
    #[inline(never)]
    pub fn fork_for(&self, workers: usize, mappings: Option<OverlayMappings>) -> bool {
        if let Some(maps) = mappings {
            self.adopt_overlays(maps, workers);
        }
        self.note_routes();
        self.note_memory("the fork");
        self.overlays_on()
    }

    /// The overlays for `workers` workers, from the fork (the owner's, on the main thread,
    /// before any worker starts): every sub-store mapped with room for the base and a region
    /// per worker, the bounds taken, the base's later growth indexed apart. `false`, and the
    /// store as it was, when the system gives no mapping. The tests'; a build takes the mappings
    /// first (`overlay_mappings`) and adopts them at the fork (`fork_for`).
    #[cfg(test)]
    pub fn fork_overlays(&self, workers: usize) -> bool {
        match Self::overlay_mappings(workers) {
            Some(maps) => {
                self.adopt_overlays(maps, workers);
                true
            }
            None => false,
        }
    }

    /// The mappings of the overlays for `workers` workers, every one taken or none: a refusal
    /// gives back the ones taken.
    #[cold]
    pub fn overlay_mappings(workers: usize) -> Option<OverlayMappings> {
        let entries = LOCAL_BASE as usize + workers * WORKER_SPAN as usize;
        let sizes = [
            SlabVec::<Type>::mapping_bytes(entries),
            SlabVec::<u8>::mapping_bytes(entries),
            SlabVec::<Box<[TypeId]>>::mapping_bytes(entries),
            SlabVec::<LitVal>::mapping_bytes(entries),
            SlabVec::<Box<str>>::mapping_bytes(entries),
            SlabVec::<Refinement>::mapping_bytes(entries),
            SlabVec::<MatchInfo>::mapping_bytes(entries),
        ];
        let mut maps = Vec::with_capacity(sizes.len());
        for bytes in sizes {
            maps.push(crate::shared::Mapping::new(bytes)?);
        }
        Some(OverlayMappings(maps))
    }

    /// The overlays for `workers` workers in `maps` (`overlay_mappings`, taken for as many).
    #[cold]
    fn adopt_overlays(&self, maps: OverlayMappings, workers: usize) {
        let own = LOCAL_BASE as usize;
        let mut maps = maps.0.into_iter();
        let mut next = || maps.next().expect("a mapping per vector");
        self.types.adopt(next(), own);
        self.flags.adopt(next(), own);
        self.lists.adopt(next(), own);
        self.lits.adopt(next(), own);
        self.blocked.adopt(next(), own);
        self.refinements.adopt(next(), own);
        self.matches.adopt(next(), own);
        let bounds = [self.types.len(), self.lists.len(), self.lits.len(), self.blocked.len(), self.refinements.len(), self.matches.len()].map(|n| n as u32);
        let frozen = crate::measure::types_wanted().then(|| {
            std::array::from_fn(|i| {
                let out = Table::with_capacity(16);
                unsafe { out.exchange(self.index_of(Sub::ALL[i])) };
                out
            })
        });
        let apart = Apart { workers: (0..workers).map(|k| Box::new(Overlay::new(self, k))).collect(), bounds, late: std::array::from_fn(|_| Table::with_capacity(1024)), frozen, origins: SharedMap::new(), base_reps: SharedMap::new(), export_shadow: Default::default() };
        unsafe { *self.apart.get() = Some(Box::new(apart)) };
        self.overlaid.store(true, Ordering::Release);
    }

    fn index_of(&self, sub: Sub) -> &Table {
        match sub {
            Sub::Types => &self.type_index,
            Sub::Lists => &self.list_index,
            Sub::Lits => &self.lit_index,
            Sub::Blocked => &self.blocked_index,
            Sub::Refinements => &self.refine_index,
            Sub::Matches => &self.match_index,
        }
    }

    /// This thread types for `worker`: its types go to the worker's overlay from here on.
    pub fn enter_overlay(&self, worker: usize) {
        if let Some(o) = self.apart_ref().and_then(|a| a.workers.get(worker)) {
            HERE.with(|h| h.set(&**o as *const Overlay));
        }
    }

    pub fn leave_overlay(&self) {
        HERE.with(|h| h.set(std::ptr::null()));
    }

    /// The overlays' end as a place types are made (the merge's, after the join, the workers'
    /// threads gone): the base's later growth indexed with the rest, the store one index per
    /// sub-store again. The overlays stay, read by their ids until the store goes.
    #[cold]
    pub fn join_overlays(&self) {
        if !self.overlaid.swap(false, Ordering::AcqRel) {
            return;
        }
        self.leave_overlay();
        let apart = unsafe { (*self.apart.get()).as_mut().expect("the overlays") };
        if let Some(frozen) = apart.frozen.take() {
            for (i, t) in frozen.iter().enumerate() {
                unsafe { t.exchange(self.index_of(Sub::ALL[i])) };
            }
        }
        // Nothing probes an overlay from here on: its entries stay, read by their ids. Nor
        // does anything translate into one: the memos go with the indexes (the merge reads the
        // provenance of the unshared paths from `origins` and `base_reps`), the counts stay.
        let mut retired = Vec::with_capacity(apart.workers.len());
        for o in apart.workers.iter_mut() {
            let index = std::mem::replace(&mut o.index, std::array::from_fn(|_| Table::with_capacity(16)));
            let own = o.own.get_mut();
            let counts = std::mem::take(&mut own.counts);
            let memos = std::mem::replace(own, OwnerSide { counts, ..Default::default() });
            retired.push((index, memos));
        }
        // The overlays' indexes and memos, which nothing reads from here, freed off the merge's main
        // thread.
        crate::crew::give_back(crate::crew::Unshared(retired));
        let bounds = apart.bounds;
        for i in bounds[0]..self.types.len() as u32 {
            if !self.entry_unshared(TypeId(i)) {
                self.type_index.insert(hash_of(self.types.get(i as usize)), i);
            }
        }
        for i in bounds[1]..self.lists.len() as u32 {
            self.list_index.insert(hash_of(&**self.lists.get(i as usize)), i);
        }
        for i in bounds[2]..self.lits.len() as u32 {
            self.lit_index.insert(hash_of(self.lits.get(i as usize)), i);
        }
        for i in bounds[3]..self.blocked.len() as u32 {
            self.blocked_index.insert(hash_of(&**self.blocked.get(i as usize)), i);
        }
        for i in bounds[4]..self.refinements.len() as u32 {
            self.refine_index.insert(hash_of(self.refinements.get(i as usize)), i);
        }
        for i in bounds[5]..self.matches.len() as u32 {
            self.match_index.insert(hash_of(self.matches.get(i as usize)), i);
        }
        let late = std::mem::replace(&mut apart.late, std::array::from_fn(|_| Table::with_capacity(16)));
        crate::crew::give_back(late);
        view::summary(apart.workers.len());
    }

    /// The overlays' end, once the merge promoted what its
    /// records reach into the base and its check found no record naming an overlay's entry: every
    /// overlay's entries dropped and their pages given back to the system (`Region`), the unshared
    /// paths' provenance with them, the store the base alone from here on. In the assertion-enabled
    /// builds an overlay's entry read after it is refused (`view::read`).
    #[cold]
    pub fn reclaim_overlays(&self) {
        debug_assert!(!self.overlaid.load(Ordering::Relaxed), "the overlays reclaimed before the join");
        let apart = unsafe { (*self.apart.get()).take() };
        if apart.is_some() {
            self.reclaimed.store(true, Ordering::Release);
            // Nothing refers to the overlays from here: their entries and pages are freed off the
            // merge's main thread.
            crate::crew::give_back(crate::crew::Unshared(apart));
        }
    }

    /// Whether the merge gave the overlays back (`reclaim_overlays`).
    #[cfg(debug_assertions)]
    pub fn overlays_reclaimed(&self) -> bool {
        self.reclaimed.load(Ordering::Acquire)
    }

    /// The base's one representative of an unshared path's origin (`unshared_copy`): the origin
    /// itself when it is the base's, the path a translation into the base made for it, or none.
    /// What the merge gives every copy of the origin when its symbol keeps its id.
    pub fn base_representative(&self, origin: TypeId) -> Option<TypeId> {
        match self.apart_ref() {
            Some(apart) => apart.base_reps.get(&origin).copied().or((origin.0 < LOCAL_BASE).then_some(origin)),
            None => (origin.0 < LOCAL_BASE).then_some(origin),
        }
    }

    /// The entries of the sub-stores other than the types' that `t` names itself, each with its
    /// sub-store: its lists, its refinement and the list in it, its match record and its cases'
    /// binders, its literal, its blocked description. What the merge's check walks beside the
    /// parts (`parts`), read raw.
    pub fn sub_entries(&self, t: TypeId, f: &mut dyn FnMut(Sub, u32)) {
        match self.entry(t) {
            Type::Class(_, l) | Type::AppParam(_, l) | Type::AppVar(_, l) | Type::Alias(_, l) | Type::AppMember(_, l) | Type::Lambda(l, _) | Type::Poly(l, _) => f(Sub::Lists, l.0),
            Type::Refined(_, r) => {
                f(Sub::Refinements, r.0);
                if let Refinement::Term(_, _, l) = self.entry_refinement(r) {
                    f(Sub::Lists, l.0);
                }
            }
            Type::Match(_, m) => {
                f(Sub::Matches, m.0);
                for c in self.entry_match(m).cases.iter() {
                    f(Sub::Lists, c.binders.0);
                }
            }
            Type::Lit(l) => f(Sub::Lits, l.0),
            Type::Blocked(b) => f(Sub::Blocked, b.0),
            _ => {}
        }
    }

    /// The entries a type about to be made names, each checked as a part of what this thread
    /// makes (`view::built`): closure at construction.
    #[cfg(debug_assertions)]
    #[track_caller]
    fn built_parts(&self, t: Type) {
        let ty = |x: TypeId| view::built(self, Sub::Types, x.0);
        let list = |l: TList| view::built(self, Sub::Lists, l.0);
        match t {
            Type::Class(_, a) | Type::AppParam(_, a) | Type::AppVar(_, a) | Type::Alias(_, a) => list(a),
            Type::Lambda(ps, b) | Type::Poly(ps, b) => {
                list(ps);
                ty(b);
            }
            Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => {
                ty(a);
                ty(b);
            }
            Type::Select(p, _) | Type::Member(p, _) => ty(p),
            Type::AppMember(m, a) => {
                ty(m);
                list(a);
            }
            Type::Refined(p, r) => {
                ty(p);
                view::built(self, Sub::Refinements, r.0);
            }
            Type::Match(sc, m) => {
                ty(sc);
                view::built(self, Sub::Matches, m.0);
            }
            Type::Lit(l) => view::built(self, Sub::Lits, l.0),
            Type::Blocked(b) => view::built(self, Sub::Blocked, b.0),
            _ => {}
        }
    }

    /// `mk` past a miss of the fork's index, with the overlays on (`probe_apart`).
    #[cold]
    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn mk_apart(&self, h: u64, t: Type) -> TypeId {
        #[cfg(debug_assertions)]
        self.built_parts(t);
        let id = self.probe_apart(
            Sub::Types,
            h,
            |id| *self.types.get(id as usize) == t,
            |ov| {
                let id = ov.push_type(t, self.flags_of(t));
                ov.index[0].insert(h, id.0 - ov.base);
                id.0
            },
            || {
                let flags = self.flags_of(t);
                let id = self.types.push(t) as u32;
                self.flags.push(flags);
                id
            },
        );
        TypeId(id)
    }

    #[cold]
    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn list_apart(&self, h: u64, items: &[TypeId]) -> TList {
        #[cfg(debug_assertions)]
        for &x in items {
            view::built(self, Sub::Types, x.0);
        }
        TList(self.intern_apart(Sub::Lists, h, |o| &o.lists, &self.lists, |l| &**l == items, || items.into()))
    }

    #[cold]
    #[inline(never)]
    fn lit_apart(&self, h: u64, v: LitVal) -> u32 {
        self.intern_apart(Sub::Lits, h, |o| &o.lits, &self.lits, |x| *x == v, || v)
    }

    #[cold]
    #[inline(never)]
    fn blocked_apart(&self, h: u64, description: &str) -> u32 {
        self.intern_apart(Sub::Blocked, h, |o| &o.blocked, &self.blocked, |b| &**b == description, || description.into())
    }

    #[cold]
    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn refine_apart(&self, h: u64, r: Refinement) -> RefineId {
        #[cfg(debug_assertions)]
        match r {
            Refinement::Alias(_, t) | Refinement::Val(_, _, t) => view::built(self, Sub::Types, t.0),
            Refinement::Bounds(_, lo, hi) => {
                view::built(self, Sub::Types, lo.0);
                view::built(self, Sub::Types, hi.0);
            }
            Refinement::Term(_, _, l) => view::built(self, Sub::Lists, l.0),
        }
        if matches!(r, Refinement::Term(..) | Refinement::Val(..)) {
            self.term_refinements.store(true, Ordering::Relaxed);
        }
        RefineId(self.intern_apart(Sub::Refinements, h, |o| &o.refinements, &self.refinements, |x| *x == r, || r))
    }

    #[cold]
    #[inline(never)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn match_apart(&self, h: u64, info: &MatchInfo) -> u32 {
        #[cfg(debug_assertions)]
        {
            for c in info.cases.iter() {
                view::built(self, Sub::Lists, c.binders.0);
                view::built(self, Sub::Types, c.pattern.0);
                view::built(self, Sub::Types, c.body.0);
            }
            view::built(self, Sub::Types, info.bound.0);
        }
        self.intern_apart(Sub::Matches, h, |o| &o.matches, &self.matches, |m| m == info, || info.clone())
    }

    /// An entry of a sub-store other than the types past a miss of the fork's index, with the
    /// overlays on (`probe_apart`).
    #[inline(always)]
    fn intern_apart<T>(&self, sub: Sub, h: u64, region: impl Fn(&Overlay) -> &Region<T>, slab: &SlabVec<T>, same: impl Fn(&T) -> bool, make: impl FnOnce() -> T) -> u32 {
        let make = std::cell::Cell::new(Some(make));
        let take = || (make.take().expect("made once"))();
        self.probe_apart(
            sub,
            h,
            |id| same(slab.get(id as usize)),
            |ov| {
                let i = region(ov).push(take()) as u32;
                ov.index[sub as usize].insert(h, i);
                ov.base + i
            },
            || slab.push(take()) as u32,
        )
    }

    /// An entry of sub-store `sub` with hash `h` past a miss of the fork's index, with the
    /// overlays on: `same` tells the entry at an id. A worker outside the loader's lock looks in
    /// its overlay and makes the entry there (`make_own`); the loader's lock holder, or a
    /// thread with no overlay, looks in what the base grew by after the fork and makes it
    /// there (`make_base`), under the store's lock, which this path alone takes after the fork.
    /// The frozen namespace (`TEQ_TYPE_OVERLAYS=1`) hides the base's later entries from a
    /// worker outside the lock and the worker's overlay from its own holds; the open one
    /// (`TEQ_TYPE_OVERLAYS=open`, the measurement's) lets each see both, its overlay first, so
    /// that one worker's view has one id per entry without the imports;
    /// the shared one (`TEQ_TYPE_OVERLAYS=shared`) lets a worker see
    /// the base's later entries after its overlay and keeps the worker's overlay from its holds.
    #[inline(always)]
    fn probe_apart(&self, sub: Sub, h: u64, same: impl Fn(u32) -> bool, make_own: impl FnOnce(&Overlay) -> u32, make_base: impl FnOnce() -> u32) -> u32 {
        let apart = self.apart_ref().expect("the overlays");
        let k = sub as usize;
        let ov = self.overlay_of_thread();
        let holder = ov.is_none() || lock_depth() > 0;
        let count = |p: Probe| {
            if let Some(o) = ov.filter(|_| !holder) {
                o.count(sub, p);
            }
        };
        if let Some(frozen) = &apart.frozen {
            if let Some(id) = frozen[k].find(h, |i| same(i)) {
                count(Probe::BaseBelow);
                return id;
            }
        }
        if let Some(o) = ov.filter(|_| !holder) {
            if let Some(i) = o.index[k].find(h, |i| same(o.base + i)) {
                count(Probe::Own);
                return o.base + i;
            }
        }
        let bound = apart.bounds[k];
        let late = || apart.late[k].find(h, |p| same(bound + p)).map(|p| bound + p);
        let met_late = |id: u32| {
            if let Some(o) = ov.filter(|_| !holder && crate::measure::types_wanted()) {
                o.owner().late_met[k].insert(id, ());
            }
        };
        if let Some(id) = late() {
            count(Probe::BaseAbove);
            met_late(id);
            return id;
        }
        if let Some(o) = ov.filter(|_| !holder) {
            count(Probe::Made);
            note_made(sub);
            return make_own(o);
        }
        let _held = self.serial.lock();
        if let Some(id) = late() {
            return id;
        }
        let id = make_base();
        apart.late[k].insert(h, id - bound);
        note_made(sub);
        id
    }
}


/// Whether the measurement notes the routes' crossings (`TEQ_WORKERS_TYPES=1` with the
/// overlays on), which the forked reads of the arenas ask before they call in here.
static NOTING: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn noting() -> bool {
    NOTING.load(Ordering::Relaxed)
}

/// A read of another worker's recorded expression type `t` (of the expression `e`), by the
/// worker on this thread: counted as the import it needs (`Route::ExprType`).
#[cold]
pub fn note_peer_expr_type(e: u32, t: TypeId) {
    let Some(ov) = thread_overlay() else { return };
    if worker_of(e).is_none_or(|k| k == ov.worker) || t == crate::tir::NO_TYPE {
        return;
    }
    let store = unsafe { &*ov.store };
    store.note_record(ov, Route::ExprType, 0, e, &[t]);
}

/// A recorded expression type `t` of expression `e` as the reader on this thread reads it
/// (`Program::expr_types`' reads while the workers share the table): in its view, a worker's import or the loader's lock holder's
/// export, memoised per foreign type by the translations' memos; the entry itself unchanged, and
/// `NO_TYPE` as it is. Counted under `Route::ExprType` while the routes are noted.
pub fn expr_type_in_view(e: u32, t: TypeId) -> TypeId {
    if t == crate::tir::NO_TYPE {
        return t;
    }
    let here = HERE.with(|h| h.get());
    if here.is_null() {
        return t;
    }
    let store = unsafe { &*(*here).store };
    if noting() {
        note_peer_expr_type(e, t);
    }
    let out = store.translate_at(store.view_here(), Route::ExprType, t);
    #[cfg(debug_assertions)]
    if out != t {
        view::crossed(view::Crossing::ExprType, 1);
    }
    out
}

/// `expr_type_in_view`'s measurement without the views (`TEQ_VIEW_READS=0`): the read noted, the
/// entry as it is.
pub fn expr_type_noted(e: u32, t: TypeId) -> TypeId {
    note_peer_expr_type(e, t);
    t
}

/// A read of another worker's class record `c` (`Arena::peer_reads`): its types counted as the
/// imports they need (`note_peer_class`).
#[cold]
pub fn note_peer_class_record(c: u32, info: &crate::symbols::ClassInfo) {
    let mut types: Vec<TypeId> = info.parents.clone();
    types.extend(info.base_types.iter().map(|&(_, t)| t));
    types.extend([info.underlying, info.declared_self, info.this_type].into_iter().flatten());
    types.extend(info.ctor.iter().flat_map(|cl| cl.params.iter().map(|p| p.ty)));
    note_peer_class(c, &types);
}

/// A read of another worker's class record, by the worker on this thread: its types counted as
/// the imports they need (`Route::PeerClass`), once per record and worker.
#[cold]
pub fn note_peer_class(c: u32, types: &[TypeId]) {
    let Some(ov) = thread_overlay() else { return };
    if worker_of(c).is_none_or(|k| k == ov.worker) {
        return;
    }
    let store = unsafe { &*ov.store };
    store.note_record(ov, Route::PeerClass, 1, c, types);
}

fn thread_overlay() -> Option<&'static Overlay> {
    if !noting() || lock_depth() > 0 {
        return None;
    }
    let here = HERE.with(|h| h.get());
    (!here.is_null()).then(|| unsafe { &*here })
}

impl TypeStore {
    /// Starts noting the routes' crossings (`noting`), with the overlays on.
    pub fn note_routes(&self) {
        NOTING.store(self.overlays_on() && crate::measure::types_wanted(), Ordering::Relaxed);
    }

    pub fn stop_noting(&self) {
        NOTING.store(false, Ordering::Relaxed);
    }

    /// A signature read by the worker on this thread (`Route::Signature`): its types counted as
    /// the imports the frozen namespace needs, once per symbol and worker.
    pub fn note_signature(&self, sym: u32, types: &[TypeId]) {
        if let Some(ov) = thread_overlay() {
            self.note_record(ov, Route::Signature, 2, sym, types);
        }
    }

    fn note_record(&self, ov: &Overlay, route: Route, kind: u8, id: u32, types: &[TypeId]) {
        let c = &mut ov.owner().counts.routes[route as usize];
        c.records += 1;
        if ov.owner().records_seen.insert((kind, id), ()).is_some() {
            c.records_again += 1;
            return;
        }
        let c = &mut ov.owner().counts.routes[route as usize];
        for &t in types {
            c.types += 1;
            if !self.in_view(ov, Sub::Types, t.0) {
                c.foreign += 1;
                if t.0 >= LOCAL_BASE { c.from_peer += 1 } else { c.from_base += 1 }
            }
        }
        let own = ov.owner();
        let mut ns = ShadowNs { store: self, into: Some(ov), shadow: &mut own.import_shadow, counts: &mut own.counts.routes[route as usize] };
        for &t in types {
            ns.ty(t);
        }
    }

    /// A publication's types (`Route::Publish`, `Route::ClassCheck`), counted as the exports
    /// into the base, through the base's shadow (`ShadowNs`): what the
    /// base holds already, or an export made before, is found; the rest is appended to the shadow.
    pub fn note_export(&self, route: Route, types: &[TypeId]) {
        if !noting() {
            return;
        }
        let Some(ov) = self.overlay_of_thread() else { return };
        ov.owner().counts.routes[route as usize].records += 1;
        for &t in types {
            let c = &mut ov.owner().counts.routes[route as usize];
            c.types += 1;
            if t.0 >= LOCAL_BASE {
                c.foreign += 1;
                c.from_peer += (t.0 < ov.base || t.0 - ov.base >= WORKER_SPAN) as u64;
            }
        }
        let apart = self.apart_ref().expect("the overlays");
        let mut shadow = apart.export_shadow.lock().unwrap_or_else(|e| e.into_inner());
        let mut ns = ShadowNs { store: self, into: None, shadow: &mut shadow, counts: &mut ov.owner().counts.routes[route as usize] };
        for &t in types {
            ns.ty(t);
        }
    }

}

/// The store's memory by component (`TypeStore::memory`).
#[derive(Default, Clone, Copy)]
pub struct StoreMemory {
    /// The base's entries and their boxed payloads (the lists' items, the blocked descriptions'
    /// text, the match records' cases).
    pub base_entries: usize,
    pub base_payload: usize,
    /// The base's indexes: the fork's and the later growth's.
    pub base_indexes: usize,
    /// The buffers and tables the base outgrew, kept for their readers.
    pub retired: usize,
    /// The overlays: entries, payloads, indexes, memos.
    pub overlays: usize,
    /// The class marks and the provenance bits' share (a byte of flags per type is counted with
    /// the entries).
    pub marks: usize,
}

impl StoreMemory {
    pub fn total(&self) -> usize {
        self.base_entries + self.base_payload + self.base_indexes + self.retired + self.overlays + self.marks
    }
}

impl TypeStore {
    /// The store's memory by component, for its owner at a barrier: no worker writes meanwhile
    /// (`Table::held` reads the writer's cells).
    #[cold]
    #[inline(never)]
    pub fn memory(&self) -> StoreMemory {
        fn own<T>(v: &SlabVec<T>) -> usize {
            v.held() - v.retired_bytes()
        }
        let base_entries = own(&self.types) + own(&self.flags) + own(&self.lists) + own(&self.lits) + own(&self.blocked) + own(&self.refinements) + own(&self.matches);
        let base_payload = self.lists.as_slice().iter().map(|l| l.len() * 4).sum::<usize>()
            + self.blocked.as_slice().iter().map(|b| b.len()).sum::<usize>()
            + self.matches.as_slice().iter().map(|m| m.cases.len() * std::mem::size_of::<MatchCase>()).sum::<usize>();
        let tables = [&self.type_index, &self.list_index, &self.lit_index, &self.blocked_index, &self.refine_index, &self.match_index];
        let apart = self.apart_ref();
        let late = apart.map_or(0, |a| a.late.iter().map(|t| t.held()).sum::<usize>() + a.frozen.as_ref().map_or(0, |f| f.iter().map(|t| t.held()).sum::<usize>()));
        let base_indexes = tables.iter().map(|t| t.held() - t.retired_bytes()).sum::<usize>() + late;
        let retired = self.types.retired_bytes() + self.flags.retired_bytes() + self.lists.retired_bytes() + self.lits.retired_bytes() + self.blocked.retired_bytes() + self.refinements.retired_bytes() + self.matches.retired_bytes() + tables.iter().map(|t| t.retired_bytes()).sum::<usize>();
        let overlays = self.overlays().iter().map(|o| o.held()).sum();
        let marks = self.op_classes.held() + self.path_classes.held() + self.op_local().held() + self.path_local().held();
        StoreMemory { base_entries, base_payload, base_indexes, retired, overlays, marks }
    }

    /// The memory row of a barrier (`TEQ_WORKERS_MEMORY=1`): the store's components and the
    /// process's footprint.
    #[inline(never)]
    pub fn note_memory(&self, barrier: &str) {
        if !crate::measure::memory_wanted() {
            return;
        }
        let m = self.memory();
        let b = |n: usize| crate::report::bytes(n as u64);
        crate::measure::overlays_measured(vec![crate::measure::OverlayRow {
            label: format!("memory at {}", barrier),
            count: None,
            time: None,
            note: format!(
                "store {}: base entries {} + payload {} + indexes {}, retired {}, overlays {}, marks {}; process footprint {}",
                b(m.total()),
                b(m.base_entries),
                b(m.base_payload),
                b(m.base_indexes),
                b(m.retired),
                b(m.overlays),
                b(m.marks),
                b(crate::alloc::footprint())
            ),
        }]);
    }

    /// The overlays' counts as rows (`TEQ_WORKERS_TYPES=1`), summed over the workers: per
    /// sub-store what the probes found and what the base grew by after the fork, per route what
    /// its imports or exports did, and the memos and marks.
    #[cold]
    #[inline(never)]
    pub fn note_overlay_counts(&self) {
        let Some(apart) = self.apart_ref() else { return };
        if !crate::measure::types_wanted() {
            return;
        }
        let mut probes = [[0u64; 4]; 6];
        let mut routes = [RouteCounts::default(); Route::COUNT];
        let mut entries = [0u64; 6];
        let (mut memo, mut dry) = (0u64, 0u64);
        let mut unshared = 0u64;
        let mut late_met = [0u64; 6];
        let mut sigs = SigCounts::default();
        for o in &apart.workers {
            let c = unsafe { &*o.own.get() }.counts.sigs;
            for (a, b) in [(&mut sigs.reads, c.reads)] {
                *a += b;
            }
            let own = unsafe { &*o.own.get() };
            for (i, m) in own.late_met.iter().enumerate() {
                late_met[i] += m.len() as u64;
            }
            for (i, p) in own.counts.probes.iter().enumerate() {
                for k in 0..4 {
                    probes[i][k] += p[k];
                }
            }
            for (i, r) in own.counts.routes.iter().enumerate() {
                let t = &mut routes[i];
                for (a, b) in [(&mut t.records, r.records), (&mut t.records_again, r.records_again), (&mut t.types, r.types), (&mut t.foreign, r.foreign), (&mut t.from_base, r.from_base), (&mut t.from_peer, r.from_peer), (&mut t.nodes, r.nodes), (&mut t.edges, r.edges), (&mut t.memo_hits, r.memo_hits), (&mut t.made, r.made), (&mut t.made_other, r.made_other), (&mut t.moved, r.moved)] {
                    *a += b;
                }
            }
            for (i, n) in o.lens().iter().enumerate() {
                entries[i] += *n as u64;
            }
            memo += (own.imported.len() + own.exported.len()) as u64;
            dry += (own.import_shadow.memo.len() + own.records_seen.len()) as u64;
            unshared += (0..o.types.len()).filter(|&i| *o.flags.get(i) & UNSHARED_BIT != 0).count() as u64;
        }
        let lens = [self.types.len(), self.lists.len(), self.lits.len(), self.blocked.len(), self.refinements.len(), self.matches.len()];
        let mut rows = Vec::new();
        let row = |label: String, count: Option<u64>, note: String| crate::measure::OverlayRow { label, count, time: None, note };
        rows.push(row(format!("overlays: {} workers", apart.workers.len()), None, String::new()));
        for sub in Sub::ALL {
            let i = sub as usize;
            let p = probes[i];
            rows.push(row(
                format!("  {}: made in the overlays", sub.name()),
                Some(entries[i]),
                format!("probes past the fork's index: base below the bound {}, base above it {} (a miss when frozen: {} distinct per worker), own overlay {}, made {}; the base {} at the fork, {} made after it", p[0], p[1], late_met[i], p[2], p[3], apart.bounds[i], lens[i] as u64 - apart.bounds[i] as u64),
            ));
        }
        for r in Route::ALL {
            let c = routes[r as usize];
            rows.push(row(
                format!("  route: {}", r.name()),
                Some(c.records),
                format!("records ({} again); types {}, foreign {} (base past the bound {}, a peer's {}); walked {} nodes, {} edges, memo hits {}; appended: {} types, {} other entries; {} translated to another id", c.records_again, c.types, c.foreign, c.from_base, c.from_peer, c.nodes, c.edges, c.memo_hits, c.made, c.made_other, c.moved),
            ));
        }
        rows.push(row("  signature reads hooked".to_string(), Some(sigs.reads), "read through the views of the records".to_string()));
        for (k, what) in ["signatures", "classes", "type parameters", "aliases"].iter().enumerate() {
            let per: Vec<crate::arena::ViewCounts> = apart.workers.iter().map(|o| unsafe { &*o.own.get() }.counts.views.records[k]).collect();
            let sum = |f: fn(&crate::arena::ViewCounts) -> usize| per.iter().map(f).sum::<usize>();
            let max = |f: fn(&crate::arena::ViewCounts) -> usize| per.iter().map(f).max().unwrap_or(0);
            rows.push(row(
                format!("  views of the shared {}: records read", what),
                Some(sum(|c| c.records) as u64),
                format!("versions translated {}; copies {} ({} retired); per worker at most: reach {} ids, capacity {} bytes, copies' payload {} bytes, peak {} bytes; summed: capacity {}, payload {}, peaks {}", per.iter().map(|c| c.translated).sum::<u64>(), sum(|c| c.copies), sum(|c| c.retired), max(|c| c.reach), max(|c| c.capacity), max(|c| c.payload), max(|c| c.peak), sum(|c| c.capacity), sum(|c| c.payload), sum(|c| c.peak)),
            ));
            rows.push(row(
                format!("  views of a peer's {}: reads", what),
                Some(per.iter().map(|c| c.peer_reads).sum()),
                format!("records kept {} (per worker at most {}), translated {}, of them copies {}", sum(|c| c.peer_records), max(|c| c.peer_records), per.iter().map(|c| c.peer_translated).sum::<u64>(), sum(|c| c.peer_copies)),
            ));
        }
        rows.push(row("  memo entries (imports, exports; the measurement's)".to_string(), Some(memo + dry), format!("{} made, {} counted without making", memo, dry)));
        rows.push(row("  unshared paths in the overlays".to_string(), Some(unshared), format!("class marks {} bytes", self.op_classes.held() + self.path_classes.held() + self.op_local().held() + self.path_local().held())));
        crate::measure::overlays_measured(rows);
    }
}


/// The entries the measurement's translations would append to one namespace, made apart from
/// it: a translation of a foreign type walks its parts, finds each in the namespace (the
/// receiving worker's view, or the base) by its value, the literals, descriptions, refinements
/// and match records as the types, or in the shadow, where what an earlier translation would
/// have made is found again, and appends the rest to the shadow. The appends are what the
/// real translations (`TypeStore::import`, `export_at`) would make at those crossings, but for
/// what the worker makes itself after an import: a real import's entries meet the worker's
/// later types, the shadow's do not, so a type both imported and then made counts in both.
#[derive(Default)]
pub struct Shadow {
    /// Per sub-store, an entry's shape (its parts as `Part`s) and its shadow id.
    entries: [crate::intern::FxMap<Vec<u64>, u32>; 6],
    /// Per foreign type translated, its part.
    pub memo: crate::intern::FxMap<u32, u64>,
    pub appended: [u64; 6],
}

impl Shadow {
    fn held(&self) -> usize {
        self.entries.iter().map(|m| m.iter().map(|(k, _)| k.len() * 8 + 12).sum::<usize>()).sum::<usize>() + self.memo.capacity() * 12
    }
}

/// A part of a shape the shadow interns: an entry of the namespace (its id) or of the shadow.
const SHADOW: u64 = 1 << 40;

/// A namespace and its shadow, translating into them (`Shadow`): `into` the receiving worker's
/// overlay, or none for the base.
struct ShadowNs<'a> {
    store: &'a TypeStore,
    into: Option<&'a Overlay>,
    shadow: &'a mut Shadow,
    counts: &'a mut RouteCounts,
}

impl ShadowNs<'_> {
    fn in_ns(&self, sub: Sub, id: u32) -> bool {
        match self.into {
            Some(ov) => self.store.in_view(ov, sub, id),
            None => id < LOCAL_BASE,
        }
    }

    /// The entry of sub-store `sub` with hash `h` that `same` tells, in the namespace.
    fn find(&self, sub: Sub, h: u64, same: impl Fn(u32) -> bool) -> Option<u32> {
        let apart = self.store.apart_ref()?;
        let k = sub as usize;
        let fork = apart.frozen.as_ref().map_or(self.store.index_of(sub), |f| &f[k]);
        if let Some(i) = fork.find(h, |i| i < apart.bounds[k] && same(i)) {
            return Some(i);
        }
        match self.into {
            Some(ov) => ov.index[k].find(h, |i| same(ov.base + i)).map(|i| ov.base + i),
            None => apart.late[k].find(h, |p| same(apart.bounds[k] + p)).map(|p| apart.bounds[k] + p),
        }
    }

    /// The shadow's entry of `sub` with shape `key`, appended if new.
    fn intern(&mut self, sub: Sub, key: Vec<u64>) -> u64 {
        let n = self.shadow.entries[sub as usize].len() as u32;
        let id = *self.shadow.entries[sub as usize].entry(key).or_insert_with(|| n);
        if id == n {
            self.shadow.appended[sub as usize] += 1;
            if sub == Sub::Types {
                self.counts.made += 1;
            } else {
                self.counts.made_other += 1;
            }
        }
        SHADOW | id as u64
    }

    fn real(parts: &[u64]) -> bool {
        parts.iter().all(|&p| p & SHADOW == 0)
    }

    fn ty(&mut self, t: TypeId) -> u64 {
        if t == crate::tir::NO_TYPE || self.in_ns(Sub::Types, t.0) {
            return t.0 as u64;
        }
        if let Some(&p) = self.shadow.memo.get(&t.0) {
            self.counts.memo_hits += 1;
            return p;
        }
        self.counts.nodes += 1;
        let store = self.store;
        let out = if store.entry_unshared(t) {
            self.unshared(t)
        } else {
            let shape = store.entry(t);
            let mut key: Vec<u64> = vec![hash_of(&std::mem::discriminant(&shape))];
            let edge = |ns: &mut Self, x: TypeId| {
                ns.counts.edges += 1;
                ns.ty(x)
            };
            let real = match shape {
                Type::Class(c, a) => {
                    let l = self.list(a);
                    key.extend([c.0 as u64, l]);
                    Self::real(&[l]).then(|| Type::Class(c, TList(l as u32)))
                }
                Type::AppParam(p, a) => {
                    let l = self.list(a);
                    key.extend([p.0 as u64, l]);
                    Self::real(&[l]).then(|| Type::AppParam(p, TList(l as u32)))
                }
                Type::AppVar(v, a) => {
                    let l = self.list(a);
                    key.extend([v.0 as u64, l]);
                    Self::real(&[l]).then(|| Type::AppVar(v, TList(l as u32)))
                }
                Type::Alias(x, a) => {
                    let l = self.list(a);
                    key.extend([x.0 as u64, l]);
                    Self::real(&[l]).then(|| Type::Alias(x, TList(l as u32)))
                }
                Type::Lambda(ps, b) | Type::Poly(ps, b) => {
                    let (l, b2) = (self.list(ps), edge(self, b));
                    key.extend([l, b2]);
                    Self::real(&[l, b2]).then(|| if matches!(shape, Type::Lambda(..)) { Type::Lambda(TList(l as u32), TypeId(b2 as u32)) } else { Type::Poly(TList(l as u32), TypeId(b2 as u32)) })
                }
                Type::Union(a, b) | Type::Inter(a, b) | Type::BoundedWild(a, b) => {
                    let (a2, b2) = (edge(self, a), edge(self, b));
                    key.extend([a2, b2]);
                    Self::real(&[a2, b2]).then(|| match shape {
                        Type::Union(..) => Type::Union(TypeId(a2 as u32), TypeId(b2 as u32)),
                        Type::Inter(..) => Type::Inter(TypeId(a2 as u32), TypeId(b2 as u32)),
                        _ => Type::BoundedWild(TypeId(a2 as u32), TypeId(b2 as u32)),
                    })
                }
                Type::Select(p, x) => {
                    let p2 = edge(self, p);
                    key.extend([p2, x.0 as u64]);
                    Self::real(&[p2]).then(|| Type::Select(TypeId(p2 as u32), x))
                }
                Type::Member(p, n) => {
                    let p2 = edge(self, p);
                    key.extend([p2, n.0 as u64]);
                    Self::real(&[p2]).then(|| Type::Member(TypeId(p2 as u32), n))
                }
                Type::AppMember(m, a) => {
                    let (m2, l) = (edge(self, m), self.list(a));
                    key.extend([m2, l]);
                    Self::real(&[m2, l]).then(|| Type::AppMember(TypeId(m2 as u32), TList(l as u32)))
                }
                Type::Refined(p, r) => {
                    let (p2, r2) = (edge(self, p), self.refinement(r));
                    key.extend([p2, r2]);
                    Self::real(&[p2, r2]).then(|| Type::Refined(TypeId(p2 as u32), RefineId(r2 as u32)))
                }
                Type::Match(sc, m) => {
                    let (s2, m2) = (edge(self, sc), self.match_record(m));
                    key.extend([s2, m2]);
                    Self::real(&[s2, m2]).then(|| Type::Match(TypeId(s2 as u32), MatchId(m2 as u32)))
                }
                Type::Lit(l) => {
                    let l2 = self.lit(l);
                    key.push(l2);
                    Self::real(&[l2]).then(|| Type::Lit(LitId(l2 as u32)))
                }
                Type::Blocked(b) => {
                    let b2 = self.blocked(b);
                    key.push(b2);
                    Self::real(&[b2]).then(|| Type::Blocked(BlockedId(b2 as u32)))
                }
                leaf => {
                    key.push(hash_of(&leaf));
                    Some(leaf)
                }
            };
            let found = real.and_then(|shape| self.find(Sub::Types, hash_of(&shape), |i| store.entry(TypeId(i)) == shape));
            match found {
                Some(id) => id as u64,
                None => self.intern(Sub::Types, key),
            }
        };
        self.shadow.memo.insert(t.0, out);
        out
    }

    /// An unshared path: the namespace's representative of its origin, or the shadow's.
    fn unshared(&mut self, t: TypeId) -> u64 {
        let apart = self.store.apart_ref().expect("the overlays");
        let origin = apart.origins.get(&t).copied().unwrap_or(t);
        let rep = match self.into {
            Some(ov) => ov.owner().unshared_reps.get(&origin.0).copied(),
            None => apart.base_reps.get(&origin).copied(),
        };
        match rep {
            Some(r) => r.0 as u64,
            None => self.intern(Sub::Types, vec![u64::MAX, origin.0 as u64]),
        }
    }

    fn list(&mut self, l: TList) -> u64 {
        if self.in_ns(Sub::Lists, l.0) {
            return l.0 as u64;
        }
        let store = self.store;
        let parts: Vec<u64> = store.entry_items(l).iter().map(|&x| {
            self.counts.edges += 1;
            self.ty(x)
        }).collect();
        if Self::real(&parts) {
            let items: Vec<TypeId> = parts.iter().map(|&p| TypeId(p as u32)).collect();
            if let Some(id) = self.find(Sub::Lists, hash_of(&items[..]), |i| store.entry_items(TList(i)) == &items[..]) {
                return id as u64;
            }
        }
        self.intern(Sub::Lists, parts)
    }

    fn lit(&mut self, l: LitId) -> u64 {
        if self.in_ns(Sub::Lits, l.0) {
            return l.0 as u64;
        }
        let store = self.store;
        let v = store.entry_lit(l);
        match self.find(Sub::Lits, hash_of(&v), |i| store.entry_lit(LitId(i)) == v) {
            Some(id) => id as u64,
            None => self.intern(Sub::Lits, vec![hash_of(&v)]),
        }
    }

    fn blocked(&mut self, b: BlockedId) -> u64 {
        if self.in_ns(Sub::Blocked, b.0) {
            return b.0 as u64;
        }
        let store = self.store;
        let text = store.entry_blocked(b);
        match self.find(Sub::Blocked, hash_of(text), |i| store.entry_blocked(BlockedId(i)) == text) {
            Some(id) => id as u64,
            None => self.intern(Sub::Blocked, text.bytes().map(|c| c as u64).collect()),
        }
    }

    /// A refinement translated as the real rebuild translates it (`TypeStore::rebuild`): its
    /// parts first, then found by its value, whoever made it, since the namespace's own may name
    /// another's types.
    fn refinement(&mut self, r: RefineId) -> u64 {
        let store = self.store;
        let (key, real) = match store.entry_refinement(r) {
            Refinement::Alias(n, t) => {
                let t2 = self.ty(t);
                (vec![0, n.0 as u64, t2], Self::real(&[t2]).then(|| Refinement::Alias(n, TypeId(t2 as u32))))
            }
            Refinement::Bounds(n, lo, hi) => {
                let (a, b) = (self.ty(lo), self.ty(hi));
                (vec![1, n.0 as u64, a, b], Self::real(&[a, b]).then(|| Refinement::Bounds(n, TypeId(a as u32), TypeId(b as u32))))
            }
            Refinement::Term(n, x, l) => {
                let l2 = self.list(l);
                (vec![2, n.0 as u64, x.0 as u64, l2], Self::real(&[l2]).then(|| Refinement::Term(n, x, TList(l2 as u32))))
            }
            Refinement::Val(n, x, t) => {
                let t2 = self.ty(t);
                (vec![3, n.0 as u64, x.0 as u64, t2], Self::real(&[t2]).then(|| Refinement::Val(n, x, TypeId(t2 as u32))))
            }
        };
        let found = real.and_then(|v| self.find(Sub::Refinements, hash_of(&v), |i| store.entry_refinement(RefineId(i)) == v));
        match found {
            Some(id) => id as u64,
            None => self.intern(Sub::Refinements, key),
        }
    }

    /// A match record translated as a refinement is (`refinement`).
    fn match_record(&mut self, m: MatchId) -> u64 {
        let store = self.store;
        let info = store.entry_match(m).clone();
        let mut key = Vec::new();
        let mut cases = Vec::new();
        for c in info.cases.iter() {
            let (b, p, x) = (self.list(c.binders), self.ty(c.pattern), self.ty(c.body));
            key.extend([b, p, x]);
            cases.push((b, p, x));
        }
        let bound = self.ty(info.bound);
        key.push(bound);
        let found = Self::real(&key).then(|| MatchInfo { cases: cases.iter().map(|&(b, p, x)| MatchCase { binders: TList(b as u32), pattern: TypeId(p as u32), body: TypeId(x as u32) }).collect(), bound: TypeId(bound as u32) }).and_then(|v| self.find(Sub::Matches, hash_of(&v), |i| *store.entry_match(MatchId(i)) == v));
        match found {
            Some(id) => id as u64,
            None => self.intern(Sub::Matches, key),
        }
    }
}

thread_local! {
    /// Where an import or an export on this thread counts its work, while one runs: the route
    /// and the overlay whose counts it is.
    static ROUTE: Cell<Option<(Route, *const Overlay)>> = const { Cell::new(None) };
}

/// An entry of sub-store `sub` appended by a translation under way on this thread, counted in
/// its route's `made` while the routes are noted (`TEQ_WORKERS_TYPES=1`).
#[inline]
fn note_made(sub: Sub) {
    if !noting() {
        return;
    }
    if let Some((route, ov)) = ROUTE.with(|r| r.get()) {
        let c = &mut unsafe { &*ov }.owner().counts.routes[route as usize];
        if sub == Sub::Types {
            c.made += 1;
        } else {
            c.made_other += 1;
        }
    }
}

/// A worker's import of the types a shared record hands it (`TypeStore::worker_import`).
pub struct WorkerImport<'s> {
    store: &'s TypeStore,
    ov: &'s Overlay,
}

impl WorkerImport<'_> {
    /// `t` in the worker's view: itself where it is the worker's canonical type, else the
    /// worker's own of its structure.
    #[inline]
    pub fn import(&self, t: TypeId) -> TypeId {
        if t == crate::tir::NO_TYPE {
            return t;
        }
        self.store.import_in::<false>(self.ov, t)
    }

    /// `t` handed to the worker in the record `what`, checked in its view (the
    /// assertion-enabled builds' `view::record`).
    #[inline]
    pub fn handed(&self, _what: &'static str, _t: TypeId) {
        #[cfg(debug_assertions)]
        if _t != crate::tir::NO_TYPE {
            view::record(self.store, _what, _t);
        }
    }
}

/// The loader's lock holder's export of the types a peer's record hands it
/// (`TypeStore::holder_export`).
pub struct HolderExport<'s> {
    store: &'s TypeStore,
}

impl HolderExport<'_> {
    /// `t` in the base, the holder's view: itself where it is the base's, else exported.
    #[inline]
    pub fn export(&self, t: TypeId) -> TypeId {
        if t == crate::tir::NO_TYPE {
            return t;
        }
        self.store.export(t)
    }

    /// `t` handed to the holder in the record `what`, checked in its view (the base).
    #[inline]
    pub fn handed(&self, _what: &'static str, _t: TypeId) {
        #[cfg(debug_assertions)]
        if _t != crate::tir::NO_TYPE {
            view::record(self.store, _what, _t);
        }
    }
}

/// Where a type read on this thread is checked against a view (`TypeStore::view_here`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    /// No worker's view: no overlays, a thread with no worker, or the store after the join.
    None,
    /// A worker outside the loader's lock: its imports make a foreign type its own.
    Worker,
    /// The loader's lock holder, whose namespace is the base: its exports make a worker's type
    /// the base's.
    Hold,
}

/// The import on read and the export at publication: the production translations (`import`,
/// `export`), which count nothing, and the measurement's (`import_at`, `export_at`), which count
/// their work under a route while the routes are noted (`TEQ_WORKERS_TYPES=1`). The typer routes
/// the signature reads (`Worker::completed_sig`) and the loader operations' inputs
/// (`merged_overload`, `parameters_of`) through them; a consumer that meets a record's type raw
/// (a quote's, a quote pattern's, a stored inline body's) takes `in_view_here`, which imports it
/// for a worker outside the loader's lock and exports it for the lock's holder.
impl TypeStore {
    /// Whether an id of sub-store `sub` lies in `ov`'s worker's namespace: the base below the
    /// bound or the worker's own overlay.
    #[inline]
    fn in_view(&self, ov: &Overlay, sub: Sub, id: u32) -> bool {
        id < self.bound(sub) || (id >= ov.base && id - ov.base < WORKER_SPAN)
    }

    /// The view the reads of this thread are in: a worker's outside the loader's lock, the
    /// base's under it, none without overlays.
    #[inline]
    pub fn view_here(&self) -> View {
        if self.overlay_of_thread().is_none() {
            return View::None;
        }
        match self.apart_ref() {
            Some(_) => {
                if lock_depth() > 0 {
                    View::Hold
                } else {
                    View::Worker
                }
            }
            _ => View::None,
        }
    }

    /// Whether the reader of view `view` translates `t` before it uses it: for a worker, a type
    /// outside the base below the bound and its own overlay (in the shared namespace the base's
    /// later type too, which the import finds as it is unless the worker made its own before);
    /// for the holder, an overlay's type.
    #[inline]
    #[cfg(test)]
    pub fn outside(&self, view: View, t: TypeId) -> bool {
        match view {
            View::None => false,
            View::Worker => {
                let ov = unsafe { &*HERE.with(|h| h.get()) };
                !self.in_view(ov, Sub::Types, t.0)
            }
            View::Hold => t.0 >= LOCAL_BASE,
        }
    }

    /// `t` in the view of the reader on this thread: imported for a worker outside the loader's
    /// lock, exported for its holder, as it is without a worker's view (the overlays off, after
    /// the join). What a consumer that meets a record's type raw (a quote's,
    /// a pattern's, a stored body's) takes before it substitutes or compares.
    #[inline]
    pub fn in_view_here(&self, t: TypeId) -> TypeId {
        if !self.overlaid.load(Ordering::Relaxed) {
            return t;
        }
        self.in_view_overlaid(t)
    }

    #[inline(never)]
    fn in_view_overlaid(&self, t: TypeId) -> TypeId {
        if t == crate::tir::NO_TYPE {
            return t;
        }
        let out = self.translate(self.view_here(), t);
        #[cfg(debug_assertions)]
        if out != t {
            view::crossed(view::Crossing::Consumer, 1);
        }
        out
    }

    /// `t` in the reader's namespace: imported for a worker, exported for the holder.
    pub fn translate(&self, view: View, t: TypeId) -> TypeId {
        match view {
            View::None => t,
            View::Worker => self.import(t),
            View::Hold => self.export(t),
        }
    }

    /// `translate`, its work counted under `route` while the routes are noted.
    pub fn translate_at(&self, view: View, route: Route, t: TypeId) -> TypeId {
        match view {
            View::None => t,
            View::Worker => self.import_at(route, t),
            View::Hold => self.export_at(route, t),
        }
    }

    /// What the worker on this thread held in its views and its signatures' copies at the end of
    /// its work (`ViewsHeld`).
    pub fn count_views(&self, records: [crate::arena::ViewCounts; 4]) {
        if let Some(ov) = self.overlay_of_thread() {
            ov.owner().counts.views = ViewsHeld { records };
        }
    }

    /// A normalised signature read counted in this thread's overlay (`SigCounts`).
    pub fn count_sig_read(&self, f: impl FnOnce(&mut SigCounts)) {
        if let Some(ov) = self.overlay_of_thread() {
            f(&mut ov.owner().counts.sigs);
        }
    }

    fn count_route(&self, ov: &Overlay, f: impl FnOnce(&mut RouteCounts)) {
        if let Some((r, _)) = ROUTE.with(|r| r.get()) {
            f(&mut ov.owner().counts.routes[r as usize]);
        }
    }

    /// The type `t` in the namespace of this thread's worker (the import on read): itself when
    /// it lies there, else made the worker's by structural
    /// interning, parts first, through `mk` and the sub-stores' constructors as a type made anew
    /// is, so that a type imported and then made, or made and then imported, meets its own index
    /// either way; an unshared path made again as one of the worker's own. The memo per foreign id
    /// (`OwnerSide::imported`) is immutable structural translation alone, a cost saving for the
    /// interned types and the one identity of an unshared path's import; a variable's solution and
    /// an alias's reduction are other operations and never an import. Outside a worker, or under
    /// the loader's lock, whose types are the base's, `t` as it is. Nothing is counted.
    #[inline]
    pub fn import(&self, t: TypeId) -> TypeId {
        if !self.overlaid.load(Ordering::Relaxed) {
            return t;
        }
        self.import_overlaid(t)
    }

    #[inline(never)]
    fn import_overlaid(&self, t: TypeId) -> TypeId {
        match self.overlay_here() {
            Some(ov) => self.import_in::<false>(ov, t),
            None => t,
        }
    }

    /// The list `l` in the namespace of this thread's worker, as `import` has a type: a foreign
    /// list's items imported and made the worker's list, before anything reads them.
    #[inline]
    pub fn import_list(&self, l: TList) -> TList {
        if !self.overlaid.load(Ordering::Relaxed) {
            return l;
        }
        self.import_list_overlaid(l)
    }

    #[inline(never)]
    fn import_list_overlaid(&self, l: TList) -> TList {
        match self.overlay_here() {
            Some(ov) => self.import_list_in::<false>(ov, l),
            None => l,
        }
    }

    /// `f` with the import of the worker this thread types for, outside the loader's lock with
    /// the overlays on and a namespace whose views are checked (not the open one): the
    /// translation of a shared record's types into the worker's view (`Symbols`' record
    /// views). `None` elsewhere.
    pub fn worker_import<R>(f: impl FnOnce(&WorkerImport<'_>) -> R) -> Option<R> {
        let here = HERE.with(|h| h.get());
        if here.is_null() || lock_depth() > 0 {
            return None;
        }
        let ov = unsafe { &*here };
        let store = unsafe { &*ov.store };
        match store.apart_ref() {
            Some(_) if store.overlays_on() => Some(f(&WorkerImport { store, ov })),
            _ => None,
        }
    }

    /// `f` with the export of the loader's lock holder on this thread, the translation of a peer's
    /// record's types into the holder's view, the base (`Symbols`' record views), with the
    /// overlays on in a namespace whose views are checked; `None` elsewhere.
    pub fn holder_export<R>(f: impl FnOnce(&HolderExport<'_>) -> R) -> Option<R> {
        let here = HERE.with(|h| h.get());
        if here.is_null() || lock_depth() == 0 {
            return None;
        }
        let store = unsafe { &*(*here).store };
        match store.apart_ref() {
            Some(_) if store.overlays_on() => Some(f(&HolderExport { store })),
            _ => None,
        }
    }

    /// `import`, its work counted under `route` while the routes are noted.
    pub fn import_at(&self, route: Route, t: TypeId) -> TypeId {
        if !noting() {
            return self.import(t);
        }
        let Some(ov) = self.overlay_here() else { return t };
        ROUTE.with(|r| r.set(Some((route, ov as *const Overlay))));
        self.count_route(ov, |c| c.types += 1);
        if !self.in_view(ov, Sub::Types, t.0) {
            let peer = t.0 >= LOCAL_BASE;
            self.count_route(ov, |c| {
                c.foreign += 1;
                if peer { c.from_peer += 1 } else { c.from_base += 1 }
            });
        }
        let out = self.import_in::<true>(ov, t);
        ROUTE.with(|r| r.set(None));
        out
    }

    fn import_in<const COUNT: bool>(&self, ov: &Overlay, t: TypeId) -> TypeId {
        if self.in_view(ov, Sub::Types, t.0) {
            return t;
        }
        if let Some(&m) = ov.owner().imported.get(&t.0) {
            if COUNT {
                self.count_route(ov, |c| c.memo_hits += 1);
            }
            return m;
        }
        if COUNT {
            self.count_route(ov, |c| c.nodes += 1);
        }
        // The marks precede the first type over their class: the source's flags, cached when it
        // was made, are those its parts and the marks give now, whatever id the import finds for
        // it (in the shared namespace the source itself).
        #[cfg(debug_assertions)]
        if !self.entry_unshared(t) {
            assert_eq!(self.flags_of(self.entry(t)), self.entry_flag(t), "an import's source has flags its class marks no longer give: a class mark set after a type over the class was made");
        }
        let made = self.rebuild(t, &mut |p| {
            if COUNT {
                self.count_route(ov, |c| c.edges += 1);
            }
            self.import_in::<COUNT>(ov, p)
        }, &mut |l| self.import_list_in::<COUNT>(ov, l));
        if COUNT && made != t {
            self.count_route(ov, |c| c.moved += 1);
        }
        ov.owner().imported.insert(t.0, made);
        made
    }

    fn import_list_in<const COUNT: bool>(&self, ov: &Overlay, l: TList) -> TList {
        if self.in_view(ov, Sub::Lists, l.0) {
            return l;
        }
        let items: Vec<TypeId> = self.entry_items(l).iter().map(|&x| {
            if COUNT {
                self.count_route(ov, |c| c.edges += 1);
            }
            self.import_in::<COUNT>(ov, x)
        }).collect();
        self.list(&items)
    }

    /// The type `t` in the base (the export at publication, and the canonicalisation of a loader
    /// operation's inputs), for the loader's lock holder: a base type as
    /// it is, another an overlay's (this worker's or a peer's), interned into the base with its
    /// parts first, memoised per worker (`OwnerSide::exported`); an unshared path made again as the
    /// base's own. The root shortcut (a base id as it is) is sound by closure: no base entry names
    /// an overlay's (`view::built`). Nothing is counted.
    pub fn export(&self, t: TypeId) -> TypeId {
        debug_assert!(lock_depth() > 0, "an export outside the loader's lock");
        let Some(ov) = self.overlay_of_thread() else { return t };
        if t.0 < LOCAL_BASE {
            return t;
        }
        // The base's path whatever the namespace: this thread's overlay out of `probe_apart`'s
        // sight while the export runs.
        let here = HERE.with(|h| h.replace(std::ptr::null()));
        let out = self.export_in::<false>(ov, t);
        HERE.with(|h| h.set(here));
        out
    }

    /// `export`, its work counted under `route` while the routes are noted.
    pub fn export_at(&self, route: Route, t: TypeId) -> TypeId {
        if !noting() {
            return self.export(t);
        }
        debug_assert!(lock_depth() > 0, "an export outside the loader's lock");
        let Some(ov) = self.overlay_of_thread() else { return t };
        ROUTE.with(|r| r.set(Some((route, ov as *const Overlay))));
        self.count_route(ov, |c| {
            c.types += 1;
            if t.0 >= LOCAL_BASE {
                c.foreign += 1;
                c.from_peer += (t.0 < ov.base || t.0 - ov.base >= WORKER_SPAN) as u64;
            }
        });
        let here = HERE.with(|h| h.replace(std::ptr::null()));
        let out = self.export_in::<true>(ov, t);
        HERE.with(|h| h.set(here));
        ROUTE.with(|r| r.set(None));
        out
    }

    fn export_in<const COUNT: bool>(&self, ov: &Overlay, t: TypeId) -> TypeId {
        if t.0 < LOCAL_BASE {
            return t;
        }
        if let Some(&m) = ov.owner().exported.get(&t.0) {
            if COUNT {
                self.count_route(ov, |c| c.memo_hits += 1);
            }
            return m;
        }
        if COUNT {
            self.count_route(ov, |c| c.nodes += 1);
        }
        let made = self.rebuild(t, &mut |p| {
            if COUNT {
                self.count_route(ov, |c| c.edges += 1);
            }
            self.export_in::<COUNT>(ov, p)
        }, &mut |l| {
            if l.0 < LOCAL_BASE {
                return l;
            }
            let items: Vec<TypeId> = self.entry_items(l).iter().map(|&x| {
                if COUNT {
                    self.count_route(ov, |c| c.edges += 1);
                }
                self.export_in::<COUNT>(ov, x)
            }).collect();
            self.list(&items)
        });
        ov.owner().exported.insert(t.0, made);
        made
    }

    /// `t` made again from its fields with each part through `part` and each list through
    /// `list`, in the store this thread makes types in: the literal, blocked, refinement and
    /// match entries interned again with their values; an unshared path made again as an
    /// unshared path.
    pub fn rebuild(&self, t: TypeId, part: &mut dyn FnMut(TypeId) -> TypeId, list: &mut dyn FnMut(TList) -> TList) -> TypeId {
        match self.entry(t) {
            Type::Class(c, args) => {
                let l = list(args);
                self.mk(Type::Class(c, l))
            }
            Type::AppParam(p, args) => {
                let l = list(args);
                self.mk(Type::AppParam(p, l))
            }
            Type::AppVar(v, args) => {
                let l = list(args);
                self.mk(Type::AppVar(v, l))
            }
            Type::Alias(a, args) => {
                let l = list(args);
                self.mk(Type::Alias(a, l))
            }
            Type::Lambda(ps, b) => {
                let ps = list(ps);
                let b = part(b);
                self.mk(Type::Lambda(ps, b))
            }
            Type::Poly(ps, b) => {
                let ps = list(ps);
                let b = part(b);
                self.mk(Type::Poly(ps, b))
            }
            Type::Union(a, b) => {
                let (a, b) = (part(a), part(b));
                self.mk(Type::Union(a, b))
            }
            Type::Inter(a, b) => {
                let (a, b) = (part(a), part(b));
                self.mk(Type::Inter(a, b))
            }
            Type::BoundedWild(lo, hi) => {
                let (lo, hi) = (part(lo), part(hi));
                self.mk(Type::BoundedWild(lo, hi))
            }
            Type::Select(p, s) => {
                let p = part(p);
                self.mk(Type::Select(p, s))
            }
            Type::Member(p, n) => {
                let p = part(p);
                self.mk(Type::Member(p, n))
            }
            Type::AppMember(m, args) => {
                let m = part(m);
                let l = list(args);
                self.mk(Type::AppMember(m, l))
            }
            Type::Refined(p, r) => {
                let p = part(p);
                let r = match self.entry_refinement(r) {
                    Refinement::Alias(n, x) => Refinement::Alias(n, part(x)),
                    Refinement::Bounds(n, lo, hi) => Refinement::Bounds(n, part(lo), part(hi)),
                    Refinement::Term(n, s, l) => Refinement::Term(n, s, list(l)),
                    Refinement::Val(n, s, x) => Refinement::Val(n, s, part(x)),
                };
                let r = self.refine(r);
                self.mk(Type::Refined(p, r))
            }
            Type::Match(s, m) => {
                let s = part(s);
                let info = self.entry_match(m).clone();
                let cases: Vec<MatchCase> = info.cases.iter().map(|c| MatchCase { binders: list(c.binders), pattern: part(c.pattern), body: part(c.body) }).collect();
                let bound = part(info.bound);
                self.match_type(s, &cases, bound)
            }
            Type::Lit(l) => self.lit(self.entry_lit(l)),
            Type::Blocked(b) => self.blocked(self.entry_blocked(b)),
            Type::Term(_) if self.entry_unshared(t) => self.unshared_copy(t),
            leaf => self.mk(leaf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A class a worker made has a tagged id: its mark is sparse, not a table sized by the tag.
    #[test]
    fn a_local_class_is_marked_apart_from_the_dense_marks() {
        let s = TypeStore::new();
        let local = ClassId(LOCAL_BASE + 5);
        let other = ClassId(crate::arena::worker_base(3) + 9);
        s.mark_op_class(local);
        s.mark_path_class(other);
        assert!(s.is_op_class(local));
        assert!(!s.is_op_class(ClassId(LOCAL_BASE + 6)));
        assert!(!s.is_op_class(other));
        assert!(s.is_path_class(other));
        assert!(!s.is_path_class(local));
        assert!(s.held() < 1 << 20, "the marks took {} bytes", s.held());
    }

    use crate::shared::ReentrantLock;

    /// A store past the signature phase (some base types) forked with `workers` overlays, boxed
    /// so that it stays where its overlays point to it (`Overlay::store`), as the build's shared
    /// store does.
    fn forked(workers: usize) -> Box<TypeStore> {
        let s = Box::new(TypeStore::new());
        s.class(ClassId(7), &[ANY]);
        s.set_exclusive(false);
        assert!(s.fork_overlays(workers));
        s
    }

    fn param(s: &TypeStore, p: u32) -> TypeId {
        s.param(TParamId(p))
    }

    /// Without a fork the store is one store: no overlay, every id the base's, an import the
    /// identity (the plain path the instruction rule holds).
    #[test]
    fn the_plain_path_keeps_one_store() {
        let s = TypeStore::new();
        let t = s.class(ClassId(4), &[s.param(TParamId(1))]);
        assert!(t.0 < 64 && s.overlays().is_empty() && !s.overlays_on());
        assert_eq!(s.import(t), t);
        assert_eq!(s.bound(Sub::Types), u32::MAX);
    }

    /// A worker's types are made in its overlay, under its worker's tag; the base's types from
    /// before the fork are found where they are.
    #[test]
    fn a_worker_makes_its_types_in_its_overlay() {
        let s = forked(2);
        let before = s.class(ClassId(7), &[ANY]);
        s.enter_overlay(1);
        assert_eq!(s.class(ClassId(7), &[ANY]), before);
        assert!(before.0 < s.bound(Sub::Types));
        let t = s.class(ClassId(9), &[param(&s, 3)]);
        assert_eq!(worker_of(t.0), Some(1));
        assert_eq!(s.class(ClassId(9), &[param(&s, 3)]), t);
        let Type::Class(c, args) = s.get(t) else { panic!("{:?}", s.get(t)) };
        assert_eq!((c, worker_of(args.0)), (ClassId(9), Some(1)));
        s.leave_overlay();
        s.join_overlays();
        assert_eq!(s.get(t), Type::Class(ClassId(9), args), "an overlay's type is read by its id after the join");
    }

    /// The invariant, for each sub-store: within a worker's view
    /// structurally equal entries have one id whichever order they were imported and made in.
    /// Worker 0 makes a type whose entry of the sub-store is its own; worker 1 imports it and then
    /// makes it, and makes another and then imports worker 0's copy of that.
    #[test]
    fn a_view_has_one_id_per_type_in_either_order() {
        let makers: [(&str, fn(&TypeStore, u32) -> TypeId); 6] = [
            ("types", |s, k| s.class(ClassId(20 + k), &[])),
            ("lists", |s, k| s.class(ClassId(7), &[s.param(TParamId(40 + k)), ANY])),
            ("literals", |s, k| s.lit(LitVal::Int(500 + k as i32))),
            ("blocked descriptions", |s, k| s.blocked(&format!("refinement: T {{ def m{}: Int }}", k))),
            ("refinements", |s, k| {
                let r = s.refine(Refinement::Alias(Name(900 + k), ANY));
                s.mk(Type::Refined(s.class(ClassId(7), &[ANY]), r))
            }),
            ("match records", |s, k| s.match_type(s.param(TParamId(60)), &[MatchCase { binders: EMPTY_LIST, pattern: s.lit(LitVal::Int(k as i32)), body: ANY }], ANY)),
        ];
        for (what, make) in makers {
            let s = forked(2);
            s.enter_overlay(0);
            let (a, b) = (make(&s, 1), make(&s, 2));
            assert_eq!(worker_of(a.0), Some(0), "{}", what);
            s.enter_overlay(1);
            let imported = s.import(a);
            assert_eq!(worker_of(imported.0), Some(1), "{}", what);
            assert_eq!(make(&s, 1), imported, "{}: imported, then made", what);
            let own = make(&s, 2);
            assert_eq!(s.import(b), own, "{}: made, then imported", what);
            assert_eq!(s.import(a), imported, "{}: imported twice", what);
        }
    }

    /// An unshared path's identity is its origin's: exported and imported back it is the
    /// original again, and two peers translating the same original, through each other or
    /// through the base, each hold one path for it.
    #[test]
    fn an_unshared_path_round_trips_and_converges() {
        let s = forked(3);
        let lock = ReentrantLock::new();
        let sym = SymId(worker_base(0) + 7);
        s.enter_overlay(0);
        let u = s.term_unshared(sym);
        lock.lock();
        let e = s.export_at(Route::Publish, u);
        lock.unlock();
        assert!(e != u && s.entry_unshared(e));
        assert_eq!(s.import(e), u, "the export imported back into its origin's namespace");
        s.enter_overlay(1);
        let a = s.import(u);
        assert_eq!(s.import(e), a, "worker 1's copy, through the base");
        assert!(a != u && a != e && s.is_unshared(a));
        lock.lock();
        assert_eq!(s.export_at(Route::Publish, a), e, "worker 1's copy exported is the base's one");
        lock.unlock();
        s.enter_overlay(2);
        let c = s.import(a);
        assert_eq!(s.import(u), c, "worker 2's copy, from the origin");
        assert_eq!(s.import(e), c, "worker 2's copy, through the base");
        assert!(worker_of(c.0) == Some(2) && s.is_unshared(c) && c != s.mk(Type::Term(sym)));
        s.enter_overlay(0);
        assert_eq!(s.import(c), u, "worker 2's copy imported into the origin's namespace");
    }

    /// The measurement's shadow counts what a real import appends, sub-store by sub-store: a
    /// peer's type over every sub-store counted into one fresh namespace appends in the shadow
    /// what the real import appends in another; a type sharing those parts appends only itself;
    /// and a foreign literal the namespace holds already appends nothing.
    #[test]
    fn the_shadow_counts_what_an_import_appends() {
        let s = forked(3);
        s.enter_overlay(0);
        let lit = s.lit(LitVal::Int(501));
        let described = s.blocked("refinement: T { def q: Int }");
        let matched = s.match_type(param(&s, 60), &[MatchCase { binders: EMPTY_LIST, pattern: lit, body: ANY }], ANY);
        let refinement = s.refine(Refinement::Alias(Name(901), lit));
        let t = s.mk(Type::Refined(s.class(ClassId(7), &[described, matched]), refinement));
        let both = s.union(t, param(&s, 61));
        s.enter_overlay(2);
        let before = s.overlays()[2].lens();
        s.import(t);
        let after = s.overlays()[2].lens();
        let real: Vec<u64> = (0..6).map(|i| (after[i] - before[i]) as u64).collect();
        s.enter_overlay(1);
        let ov = &*s.overlays()[1];
        s.note_record(ov, Route::ExprType, 0, 1, &[t]);
        assert_eq!(ov.owner().import_shadow.appended.to_vec(), real, "the shadow's appends against the real import's, per sub-store");
        assert!(real.iter().all(|&n| n > 0), "every sub-store took part: {:?}", real);
        s.note_record(ov, Route::ExprType, 0, 2, &[both]);
        let appended = ov.owner().import_shadow.appended;
        assert_eq!((appended[0], appended[1]), (real[0] + 2, real[1]), "the union and its second parameter alone");
        s.enter_overlay(2);
        let ov2 = &*s.overlays()[2];
        s.note_record(ov2, Route::ExprType, 0, 3, &[lit]);
        assert_eq!(ov2.owner().import_shadow.appended, [0; 6], "a literal the namespace holds");
        let c = ov2.owner().counts.routes[Route::ExprType as usize];
        assert_eq!((c.foreign, c.made, c.made_other), (1, 0, 0));
    }

    /// A record a worker made over another's types (a refinement of worker 1's over worker 0's
    /// parameter, a match record likewise) meets a type of worker 0's over it: the import into
    /// worker 1 rebuilds the record from its translated parts, and the shadow counts the same
    /// appends, sub-store by sub-store.
    #[test]
    fn the_shadow_counts_mixed_records_as_an_import_does() {
        view::unchecked(the_shadow_counts_mixed_records);
    }

    /// Mixed records, which closure at construction refuses (`view::built`) and the translations
    /// meet only in a build that breaks it: the shadow counts them as the real rebuild does.
    fn the_shadow_counts_mixed_records() {
        let s = forked(2);
        s.enter_overlay(0);
        let p = param(&s, 70);
        let q = param(&s, 71);
        s.enter_overlay(1);
        let r = s.refine(Refinement::Alias(Name(902), p));
        let matched = s.match_type(q, &[MatchCase { binders: EMPTY_LIST, pattern: p, body: ANY }], ANY);
        let Type::Match(_, m) = s.get(matched) else { panic!() };
        s.enter_overlay(0);
        let refined = s.mk(Type::Refined(ANY, r));
        let over_match = s.mk(Type::Match(q, m));
        for (what, t) in [("a refinement", refined), ("a match record", over_match)] {
            s.enter_overlay(1);
            let ov = &*s.overlays()[1];
            let shadow_before = ov.owner().import_shadow.appended;
            s.note_record(ov, Route::ExprType, 0, t.0, &[t]);
            let shadow: Vec<u64> = (0..6).map(|i| ov.owner().import_shadow.appended[i] - shadow_before[i]).collect();
            let before = ov.lens();
            s.import(t);
            let after = ov.lens();
            let real: Vec<u64> = (0..6).map(|i| (after[i] - before[i]) as u64).collect();
            assert_eq!(shadow, real, "{}: the shadow's appends against the real import's", what);
            assert!(real[0] >= 2, "{}: {:?}", what, real);
        }
    }

    /// A translation counts each entry it appends once, where it is made: a chain of three
    /// types over three parameters imported into an empty namespace makes six types.
    #[test]
    fn a_translation_counts_each_append_once() {
        NOTING.store(true, Ordering::Relaxed);
        let s = forked(2);
        s.enter_overlay(0);
        let (p1, p2, p3) = (param(&s, 300), param(&s, 301), param(&s, 302));
        let chain = s.union(s.union(s.union(p1, p2), p3), s.param(TParamId(303)));
        s.enter_overlay(1);
        let before = s.overlays()[1].lens()[0];
        let imported = s.import_at(Route::ExprType, chain);
        let made = s.overlays()[1].lens()[0] - before;
        assert_eq!(made, 7);
        let counts = s.overlays()[1].owner().counts.routes[Route::ExprType as usize];
        assert_eq!((counts.made, counts.made_other), (made as u64, 0));
        assert_eq!(s.import_at(Route::ExprType, chain), imported);
        assert_eq!(s.overlays()[1].owner().counts.routes[Route::ExprType as usize].made, made as u64, "a repeat makes nothing");
    }

    /// An export interns an overlay's type into the base, parts first, whoever made them.
    #[test]
    fn an_export_makes_the_base_hold_the_type() {
        let s = forked(2);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        let peer = s.class(ClassId(40), &[param(&s, 8)]);
        s.enter_overlay(1);
        let peer = s.import(peer);
        let t = s.class(ClassId(41), &[peer, s.lit(LitVal::Int(3))]);
        lock.lock();
        let e = s.export_at(Route::Publish, t);
        lock.unlock();
        assert!(e.0 < LOCAL_BASE);
        let Type::Class(c, args) = s.entry(e) else { panic!() };
        assert!(c == ClassId(41) && args.0 < LOCAL_BASE && s.entry_items(args).iter().all(|a| a.0 < LOCAL_BASE));
        assert_eq!(s.import(e), t, "the exported type imported again is the worker's own");
    }

    /// The store's mappings are taken all or none: a refusal at any of the seven gives back the
    /// ones taken before it, changes no vector, and leaves the store shared, whose inserts, growth
    /// and drop go on as without overlays.
    #[test]
    fn a_refused_mapping_leaves_the_shared_store_whole() {
        for refused in 0..7 {
            let before = crate::shared::mapped_here();
            let s = TypeStore::new();
            let early: Vec<TypeId> = (0..100).map(|k| s.class(ClassId(7), &[s.param(TParamId(k))])).collect();
            s.set_exclusive(false);
            crate::shared::refuse_mapping(Some(refused));
            assert!(!s.fork_overlays(4), "mapping {} refused", refused);
            crate::shared::refuse_mapping(None);
            assert_eq!(crate::shared::mapped_here(), before, "mapping {} refused: the ones taken are given back", refused);
            assert!(s.overlays().is_empty() && !s.overlays_on() && !s.types.is_mapped() && !s.matches.is_mapped());
            let late: Vec<TypeId> = (0..40_000).map(|k| s.class(ClassId(8), &[s.param(TParamId(k))])).collect();
            assert!(late.iter().all(|t| t.0 < LOCAL_BASE));
            for (k, &t) in early.iter().enumerate() {
                assert_eq!(s.class(ClassId(7), &[s.param(TParamId(k as u32))]), t);
            }
            assert_eq!(s.class(ClassId(8), &[s.param(TParamId(39_999))]), late[39_999]);
            drop(s);
            assert_eq!(crate::shared::mapped_here(), before);
        }
        let before = crate::shared::mapped_here();
        let s = forked(2);
        assert!(crate::shared::mapped_here() > before);
        drop(s);
        assert_eq!(crate::shared::mapped_here(), before, "a store's mappings go with it");
    }

    /// The first type of a high-numbered worker takes a bounded amount of memory: its overlay's
    /// slabs are regions of the store's mappings, whose pages are the system's until written,
    /// and its indexes are by offset.
    #[test]
    fn the_first_type_of_the_last_worker_is_cheap() {
        let s = forked(16);
        let before = s.held();
        s.enter_overlay(15);
        let t = s.class(ClassId(50), &[ANY]);
        assert_eq!(worker_of(t.0), Some(15));
        assert!(s.held() - before < 1 << 16, "{} bytes for one type", s.held() - before);
        assert!(s.overlays()[15].held() < 1 << 16);
    }

    /// A type's flags read the class marks once, when it is made: a mark set after a type over
    /// its class was made leaves that type's flags stale, which is why a mark is carried before
    /// any type over the class is made in the receiving store, and moved to the class's new id
    /// before the merge makes the types over it again.
    #[test]
    fn a_class_mark_precedes_the_types_over_the_class() {
        let s = TypeStore::new();
        let local = ClassId(worker_base(1) + 3);
        let early = s.class(local, &[]);
        s.mark_path_class(local);
        assert!(!s.has_paths(early), "the type made before the mark keeps the flags it was made with");
        let merged = ClassId(600);
        s.carry_local_marks(|c| if c == local { merged } else { c });
        assert!(s.is_path_class(merged) && s.has_paths(s.class(merged, &[])));
    }

    /// The base grows under the loader's lock while a worker makes types: every type from
    /// before the fork keeps its id for the worker, a later one is the worker's or the base's.
    #[test]
    fn the_base_grows_under_a_worker() {
        use std::sync::Arc;
        let s = TypeStore::new();
        let early: Vec<TypeId> = (0..200).map(|k| s.class(ClassId(7), &[s.param(TParamId(k))])).collect();
        s.set_exclusive(false);
        assert!(s.fork_overlays(2));
        let s = Arc::new(s);
        let grower = {
            let s = s.clone();
            std::thread::spawn(move || {
                let lock = ReentrantLock::new();
                lock.lock();
                let late: Vec<TypeId> = (0..20000).map(|k| s.class(ClassId(8), &[s.param(TParamId(k))])).collect();
                lock.unlock();
                late
            })
        };
        s.enter_overlay(1);
        for round in 0..50 {
            for (k, &t) in early.iter().enumerate() {
                assert_eq!(s.class(ClassId(7), &[s.param(TParamId(k as u32))]), t, "round {}", round);
            }
            let mine = s.class(ClassId(8), &[s.param(TParamId(round))]);
            assert!(worker_of(mine.0) == Some(1) || (mine.0 >= s.bound(Sub::Types) && mine.0 < LOCAL_BASE), "round {}: {}", round, mine.0);
        }
        let late = grower.join().unwrap();
        assert!(late.iter().all(|t| t.0 >= s.bound(Sub::Types) && t.0 < LOCAL_BASE));
        s.leave_overlay();
    }

    /// The enforcement contract's classification (`view::foreign`): outside the loader's lock a
    /// worker's view is the base below the bound and its own overlay, a base id at or past the
    /// bound being the base's later growth; under the lock the holder's view is the base whole,
    /// its own overlay's ids foreign there.
    #[test]
    fn a_foreign_id_is_told_by_the_reader_and_the_bound() {
        use view::{foreign, Foreign};
        let (bound, own, peer) = (100, worker_base(2), worker_base(5) + 7);
        assert_eq!(foreign(bound, own, 99, false), None);
        assert_eq!(foreign(bound, own, 100, false), Some(Foreign::LateBase), "a base id starting at the bound is foreign");
        assert_eq!(foreign(bound, own, LOCAL_BASE - 1, false), Some(Foreign::LateBase));
        assert_eq!(foreign(bound, own, own, false), None);
        assert_eq!(foreign(bound, own, own + WORKER_SPAN - 1, false), None);
        assert_eq!(foreign(bound, own, own + WORKER_SPAN, false), Some(Foreign::Peer));
        assert_eq!(foreign(bound, own, peer, false), Some(Foreign::Peer));
        assert_eq!(foreign(bound, own, 100, true), None, "the holder reads the base's later growth");
        assert_eq!(foreign(bound, own, own + 3, true), Some(Foreign::OwnOverlay));
        assert_eq!(foreign(bound, own, peer, true), Some(Foreign::Peer));
        assert!(view::publishable(LOCAL_BASE - 1) && !view::publishable(own));
    }

    /// The checked reads pass every id of the reader's view and the raw traversal reads any: a
    /// worker reads the base's types from before the fork and its own, an import reads the
    /// base's later type it translates, and after the join nothing is checked.
    #[test]
    fn a_worker_reads_its_view_and_the_import_reads_raw() {
        let s = forked(2);
        let before = s.class(ClassId(7), &[ANY]);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        lock.lock();
        let late = s.class(ClassId(40), &[s.lit(LitVal::Int(3))]);
        assert!(matches!(s.get(late), Type::Class(..)), "the holder reads the base's later type");
        lock.unlock();
        let mine = s.class(ClassId(41), &[before]);
        assert!(matches!(s.get(before), Type::Class(..)) && matches!(s.get(mine), Type::Class(..)));
        let imported = s.import(late);
        let Type::Class(_, args) = s.get(imported) else { panic!() };
        assert!(matches!(s.get(s.items(args)[0]), Type::Lit(_)));
        s.leave_overlay();
        s.join_overlays();
        assert!(matches!(s.get(late), Type::Class(..)), "after the join no bound restricts a read");
    }

    /// The holder's view is the base: its own worker's overlay type read under the lock is
    /// refused (the loader operation's argument, which the canonicalisation rule exports first).
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "view check: the loader's lock holder read types")]
    fn the_holder_reading_its_workers_overlay_is_refused() {
        let s = forked(1);
        // Leaked: the panic unwinds with the lock held, whose guard may not outlive it.
        let lock: &'static ReentrantLock = Box::leak(Box::new(ReentrantLock::new()));
        s.enter_overlay(0);
        let mine = s.class(ClassId(41), &[ANY]);
        lock.lock();
        s.get(mine);
    }

    /// Closure at construction for the base: a base wrapper the holder makes around its worker's
    /// unexported overlay child is refused.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "view check: the loader's lock holder made a part of types")]
    fn a_base_wrapper_around_an_overlay_child_is_refused() {
        let s = forked(1);
        // Leaked: the panic unwinds with the lock held, whose guard may not outlive it.
        let lock: &'static ReentrantLock = Box::leak(Box::new(ReentrantLock::new()));
        s.enter_overlay(0);
        let mine = s.class(ClassId(41), &[ANY]);
        lock.lock();
        s.class(ClassId(42), &[mine]);
    }

    /// Every id reachable from `t` through its parts, `t` among them (raw reads).
    fn reach(s: &TypeStore, t: TypeId) -> Vec<u32> {
        let (mut out, mut todo) = (Vec::new(), vec![t]);
        while let Some(x) = todo.pop() {
            out.push(x.0);
            s.parts(x, &mut todo);
        }
        out
    }

    /// The canonicalisation rule's first clause: a loader operation's inputs are canonicalised
    /// into the base within the hold, so that a worker's type and the base's type of the same
    /// structure compare equal there; outside the hold the same translation imports.
    #[test]
    fn the_holder_canonicalises_a_comparisons_inputs_into_the_base() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        let mine = s.class(ClassId(41), &[s.param(TParamId(3))]);
        assert_eq!(s.view_here(), View::Worker);
        lock.lock();
        assert_eq!(s.view_here(), View::Hold);
        let base = s.class(ClassId(41), &[s.param(TParamId(3))]);
        assert!(base.0 >= s.bound(Sub::Types) && base.0 < LOCAL_BASE);
        assert!(s.outside(View::Hold, mine) && !s.outside(View::Hold, base));
        assert_eq!(s.translate(View::Hold, mine), base, "the worker's input is the base's type within the hold");
        assert_eq!(s.translate(View::Hold, base), base);
        lock.unlock();
        assert!(s.outside(View::Worker, base) && !s.outside(View::Worker, mine));
        assert_eq!(s.translate(View::Worker, base), mine, "outside the hold the base's type is imported as the worker's");
        s.leave_overlay();
        s.join_overlays();
        assert_eq!(s.view_here(), View::None);
    }

    /// Closure at publication: an export makes the base hold a worker's wrapper with every part,
    /// so that nothing the base holds names an overlay's entry, which the export's root shortcut
    /// (a base id as it is) relies on.
    #[test]
    fn an_export_is_closed_over_its_parts() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        let inner = s.class(ClassId(41), &[s.lit(LitVal::Int(7))]);
        let r = s.refine(Refinement::Alias(Name(900), inner));
        let wrapper = s.union(s.mk(Type::Refined(s.class(ClassId(42), &[inner]), r)), s.param(TParamId(5)));
        assert!(reach(&s, wrapper).iter().any(|&id| id >= LOCAL_BASE));
        lock.lock();
        let e = s.export(wrapper);
        assert!(reach(&s, e).iter().all(|&id| id < LOCAL_BASE), "an export's parts are the base's");
        assert_eq!(s.export(e), e, "a base root is taken as it is");
        lock.unlock();
        s.leave_overlay();
    }

    /// Closure at construction, which the import's root shortcut (a type of the reader's view as
    /// it is) relies on: every part of what a worker makes from its imports lies in its view.
    #[test]
    fn an_imported_type_is_closed_in_the_view() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        lock.lock();
        let late = s.union(s.class(ClassId(40), &[s.lit(LitVal::Int(9))]), s.param(TParamId(8)));
        lock.unlock();
        let mine = s.inter(s.import(late), ANY);
        let ov = s.overlays()[0].base_id();
        assert!(reach(&s, mine).iter().all(|&id| id < LOCAL_BASE || id >= ov), "a worker's type names its view alone");
        s.leave_overlay();
    }

    /// The class marks precede the first type made over their class, since an import computes
    /// its flags from the marks and repairs no flag cached before: a mark set after a base type
    /// over its class was made is caught by the import's check of the source's flags (the import
    /// finds the source itself).
    #[cfg(debug_assertions)]
    #[test]
    fn a_class_mark_after_a_type_over_its_class_is_caught_by_the_import() {
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let s = forked(1);
            let lock = ReentrantLock::new();
            s.enter_overlay(0);
            lock.lock();
            let late = s.class(ClassId(43), &[s.param(TParamId(1))]);
            lock.unlock();
            s.mark_path_class(ClassId(43));
            s.import(late);
        }));
        leave_any_overlay();
        let message = caught.err().and_then(|e| e.downcast::<String>().ok()).map(|m| *m).unwrap_or_default();
        assert!(message.contains("a class mark set after a type over the class was made"), "{}", message);
    }

    #[cfg(debug_assertions)]
    fn leave_any_overlay() {
        HERE.with(|h| h.set(std::ptr::null()));
    }

    /// The shared namespace: a worker finds the base's later type as it is, the holder never
    /// finds the worker's overlay, and an import of a base type the worker made its own of
    /// before the base did gives the worker's (the reconciliation).
    #[test]
    fn the_shared_namespace_reconciles_the_workers_earlier_equivalent() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        let mine = s.class(ClassId(30), &[ANY]);
        lock.lock();
        let late = s.class(ClassId(30), &[ANY]);
        let only_late = s.class(ClassId(31), &[ANY]);
        lock.unlock();
        assert_ne!(late, mine, "the holder does not find the worker's overlay");
        assert!(late.0 < LOCAL_BASE);
        assert_eq!(s.class(ClassId(31), &[ANY]), only_late, "a worker finds the base's later type");
        assert_eq!(s.class(ClassId(30), &[ANY]), mine, "its own first");
        assert_eq!(s.import(late), mine, "the base's type reconciled to the worker's earlier one");
        assert_eq!(s.import(only_late), only_late, "the base's type as it is where the worker has none");
        s.leave_overlay();
    }

    /// A list the holder made over a type the worker made its own of first (a quote's type
    /// arguments the copier reads): `import_list` gives the worker's list over its own type
    /// before its items are read, and a list with no such part as it is.
    #[test]
    fn a_list_over_a_shadowed_type_imports_as_the_workers() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        let mine = s.class(ClassId(30), &[ANY]);
        let mine_list = s.list(&[mine]);
        lock.lock();
        let late = s.class(ClassId(30), &[ANY]);
        let late_list = s.list(&[late]);
        let plain_list = s.list(&[ANY, NOTHING]);
        lock.unlock();
        assert_ne!(late_list, mine_list);
        assert_eq!(s.import_list(late_list), mine_list);
        assert_eq!(s.items(s.import_list(late_list)), &[mine]);
        assert_eq!(s.import_list(plain_list), plain_list);
        s.leave_overlay();
    }

    /// The shared namespace's check: a base type made after the fork that the worker made its
    /// own of first is foreign to it (the worker's is the canonical one, which the import gives).
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "view check: a worker read types from the base past the bound, shadowed by the worker's own")]
    fn a_shadowed_base_type_is_refused_in_the_shared_namespace() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        s.class(ClassId(30), &[ANY]);
        lock.lock();
        let late = s.class(ClassId(30), &[ANY]);
        let only_late = s.class(ClassId(31), &[ANY]);
        lock.unlock();
        assert!(matches!(s.get(only_late), Type::Class(..)), "a base type the worker has none of is its view's");
        s.get(late);
    }

    /// The shared namespace's world of the nested cases: worker 0 makes `A`, then the base makes
    /// its own `A` and `C[A]` (the holder's completion), the base's `C[A]` made before any worker's.
    fn shadowed_wrapper(s: &TypeStore, lock: &ReentrantLock) -> (TypeId, TypeId, TypeId) {
        s.enter_overlay(0);
        let a = s.class(ClassId(50), &[ANY]);
        lock.lock();
        let base_a = s.class(ClassId(50), &[ANY]);
        let base_ca = s.class(ClassId(51), &[base_a]);
        lock.unlock();
        (a, base_a, base_ca)
    }

    /// Canonicality is over canonical parts, not over which namespace made the root first: the
    /// base's `C[A]` is not worker 0's canonical wrapper, its `A` being shadowed by the worker's,
    /// so the import and the worker's own construction give one wrapper over the worker's `A`; and
    /// as the base grows over its `C[A]`, what it grows imports over the worker's wrapper.
    #[test]
    fn a_base_wrapper_over_a_shadowed_part_is_rebuilt_over_the_workers_part() {
        let s = forked(2);
        let lock = ReentrantLock::new();
        let (a, base_a, base_ca) = shadowed_wrapper(&s, &lock);
        assert!(base_a != a && base_ca.0 < LOCAL_BASE);
        let ca = s.import(base_ca);
        assert_eq!(worker_of(ca.0), Some(0), "the worker's own wrapper");
        assert_eq!(s.class(ClassId(51), &[s.class(ClassId(50), &[ANY])]), ca, "the worker's construction meets it");
        lock.lock();
        let base_dca = s.class(ClassId(52), &[base_ca]);
        lock.unlock();
        assert_eq!(s.import(base_dca), s.class(ClassId(52), &[ca]), "the base's growth over its wrapper");
        s.leave_overlay();
    }

    /// Every edge of an entry is walked, a term refinement's list among them: a base wrapper
    /// `Refined(Any, Term(_, _, l))` over a base list the worker made its own of first is not its
    /// canonical one, whose representative is the import's, over the worker's list.
    #[test]
    fn a_base_term_refinement_over_a_shadowed_list_is_rebuilt() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        s.enter_overlay(0);
        let own = s.list(&[ANY, NOTHING]);
        lock.lock();
        let base_list = s.list(&[ANY, NOTHING]);
        let r = s.refine(Refinement::Term(Name(901), SymId(7), base_list));
        let wrapper = s.mk(Type::Refined(ANY, r));
        lock.unlock();
        assert!(base_list != own && wrapper.0 < LOCAL_BASE);
        let imported = s.import(wrapper);
        assert_ne!(imported, wrapper, "the import rebuilds the refinement over the worker's list");
        let mine = s.mk(Type::Refined(ANY, s.refine(Refinement::Term(Name(901), SymId(7), own))));
        assert_eq!(imported, mine, "the worker's own construction");
        #[cfg(debug_assertions)]
        {
            let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| view::compared("a retained record", wrapper)));
            assert!(refused.is_err(), "the retained base wrapper is refused");
        }
        s.leave_overlay();
    }

    /// An unshared path's canonicality is by its origin: a worker that holds its own path sees
    /// the base's copy of it (its export) as foreign, alone or inside a wrapper, where a worker
    /// that holds none takes the base's as its representative, the origin's other copies then
    /// translating to it; a distinct original stays distinct.
    #[test]
    fn an_unshared_origin_has_one_representative_in_a_shared_view() {
        let s = forked(2);
        let lock = ReentrantLock::new();
        let sym = SymId(worker_base(0) + 7);
        s.enter_overlay(0);
        let u = s.term_unshared(sym);
        lock.lock();
        let e = s.export(u);
        let wrapper = s.mk(Type::Select(e, SymId(9)));
        lock.unlock();
        assert!(e != u && e.0 < LOCAL_BASE);
        assert_eq!(s.import(e), u, "the export's origin is the worker's own path");
        assert_eq!(s.import(wrapper), s.mk(Type::Select(u, SymId(9))));
        #[cfg(debug_assertions)]
        for t in [e, wrapper] {
            let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| view::compared("a retained record", t)));
            assert!(refused.is_err(), "a foreign representative of an origin the worker holds, or a wrapper over it, is refused");
        }
        s.enter_overlay(1);
        assert_eq!(s.import(e), e, "a worker holding no representative takes the base's");
        #[cfg(debug_assertions)]
        view::compared("a record", e);
        assert_eq!(s.import(u), e, "the origin's other copies translate to it");
        let other = s.term_unshared(sym);
        assert!(other != e && s.import(other) == other, "a distinct original stays");
        s.leave_overlay();
    }

    /// A worker's canonical wrappers and a peer's converge through the base: a peer holding no `A`
    /// of its own takes the base's `C[A]` as its canonical one, its type over it exported, and each
    /// side's import of the other's (or of the export) is its own construction.
    #[test]
    fn peers_translate_to_each_others_canonical_wrappers() {
        let s = forked(2);
        let lock = ReentrantLock::new();
        let (a, _, base_ca) = shadowed_wrapper(&s, &lock);
        let d0 = s.class(ClassId(53), &[s.class(ClassId(51), &[a])]);
        s.enter_overlay(1);
        assert_eq!(s.import(base_ca), base_ca, "a peer without its own `A` keeps the base's wrapper");
        let d1 = s.class(ClassId(53), &[s.class(ClassId(51), &[s.class(ClassId(50), &[ANY])])]);
        assert_eq!(s.class(ClassId(51), &[s.class(ClassId(50), &[ANY])]), base_ca);
        lock.lock();
        let exported = s.export(d1);
        lock.unlock();
        assert!(exported.0 < LOCAL_BASE);
        s.enter_overlay(0);
        assert_eq!(s.import(exported), d0, "the export in worker 0's view");
        assert_eq!(s.import(d1), d0, "the peer's type in worker 0's view");
        s.enter_overlay(1);
        assert_eq!(s.import(d0), d1, "worker 0's type in the peer's view");
        s.leave_overlay();
    }

    /// Canonicality is per worker. After the join nothing translates: the worker's wrapper and the
    /// base's stand side by side, structurally equal, and the base's probe finds its own; making
    /// them one id is the merge's contract (the merge interns the overlays' kept types into the base),
    /// not the workers' views'.
    #[test]
    fn the_join_leaves_a_workers_wrapper_beside_the_bases_for_the_merge() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        let (_, base_a, base_ca) = shadowed_wrapper(&s, &lock);
        let ca = s.import(base_ca);
        s.leave_overlay();
        s.join_overlays();
        assert_ne!(ca, base_ca);
        let (Type::Class(c, l), Type::Class(d, m)) = (s.get(ca), s.get(base_ca)) else { panic!() };
        assert!(c == d && s.items(l).len() == 1 && s.items(m) == [base_a]);
        assert_eq!(s.class(ClassId(51), &[s.class(ClassId(50), &[ANY])]), base_ca, "the base's own after the join");
    }

    /// The shared namespace's check is over the parts: a record that kept the base's `C[A]`
    /// (retained before the reconciliation reached its route) is refused when it is read or
    /// compared, though no entry of the worker's equals the wrapper itself.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "shadowed by the worker's own or over a part that is")]
    fn a_retained_wrapper_over_a_shadowed_part_is_refused() {
        let s = forked(1);
        let lock = ReentrantLock::new();
        let (_, _, base_ca) = shadowed_wrapper(&s, &lock);
        view::compared("a retained record", base_ca);
    }

}
