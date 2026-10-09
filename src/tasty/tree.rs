//! The ASTs section: an index of definitions by address that decodes nothing but names and
//! modifiers, and the decoding of one definition's signature on demand. Bodies are skipped by
//! their length; a signature keeps the address of its body for later.

use super::tags::*;
use super::{NameRef, Reader, TastyFile};

/// A byte position in the ASTs section, which is how trees refer to each other.
pub type Addr = u32;

/// Modifier tags as a bit set; every modifier tag is below 64.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Flags(pub u64);

impl Flags {
    #[inline]
    pub fn has(self, tag: u8) -> bool {
        self.0 & (1u64 << tag) != 0
    }
    #[inline]
    pub fn set(&mut self, tag: u8) {
        self.0 |= 1u64 << tag;
    }
    #[inline]
    pub fn clear(&mut self, tag: u8) {
        self.0 &= !(1u64 << tag);
    }
}

/// A definition as the index knows it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub addr: Addr,
    /// `VALDEF`, `DEFDEF` or `TYPEDEF`.
    pub tag: u8,
    pub name: NameRef,
    pub flags: Flags,
    /// A `TYPEDEF` whose right-hand side is a template.
    pub is_class: bool,
    /// Whether a `VALDEF` or `DEFDEF` has a right-hand side.
    pub has_body: bool,
    /// `private[p]`, which the index reads as public within `p` and hides from outside it.
    pub qualified_private: bool,
    /// The name of the class or package `p` of `private[p]` or `protected[p]`.
    pub access_within: Option<NameRef>,
    /// An annotation of the definition has the simple name of a Scala.js interop annotation.
    pub js_annotated: bool,
}

/// The simple names of the Scala.js annotations the loader reads (`scala.scalajs.js.native` and
/// those of `scala.scalajs.js.annotation`).
pub const JS_ANNOTATION_NAMES: [&str; 8] =
    ["native", "JSType", "JSGlobal", "JSImport", "JSName", "JSBracketAccess", "JSGlobalScope", "JSExportTopLevel"];

/// An annotation with the arguments of its constructor call, as far as they are constants or paths.
#[derive(Clone, Debug)]
pub struct Annot {
    pub class: TType,
    pub args: Vec<AnnotArg>,
}

#[derive(Clone, Debug)]
pub enum AnnotArg {
    Str(NameRef),
    Path(TType),
    Other,
}

pub struct TopLevel {
    /// The qualified name of the enclosing package clause.
    pub package: NameRef,
    pub entry: Entry,
}

/// The members of one class, by address; nothing of them is decoded.
pub struct TemplateIndex {
    pub members: Vec<Entry>,
    pub exports: Vec<Addr>,
    /// Whether the template holds a statement that is no definition, which a construction runs.
    pub statements: bool,
}

#[derive(Clone, Debug)]
pub enum Const {
    Unit,
    Bool(bool),
    Byte(i32),
    Short(i32),
    Char(u32),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    Str(NameRef),
    Null,
    Class(Box<TType>),
}

/// A type as TASTy states it. Type trees (`IDENTtpt`, `APPLIEDtpt`, ...) and types read into the
/// same shape, since a signature needs the type alone.
#[derive(Clone, Debug)]
pub enum TType {
    /// `TYPEREFpkg` / `TERMREFpkg`: a package by its qualified name.
    Package(NameRef),
    /// `prefix.Name`
    TypeRef(Box<TType>, NameRef),
    /// `prefix.name`, a path; as a type it is the singleton `prefix.name.type`.
    TermRef(Box<TType>, NameRef),
    /// A type defined in this file, named by the address of its definition: a type parameter,
    /// a class or a type member. The prefix is absent for a local symbol.
    LocalType(Addr, Option<Box<TType>>),
    LocalTerm(Addr, Option<Box<TType>>),
    /// `C.this`
    This(Box<TType>),
    Applied(Box<TType>, Vec<TType>),
    /// `>: lo <: hi`
    Bounds(Box<TType>, Box<TType>),
    /// `= alias`, the right-hand side of a type alias as a type
    Alias(Box<TType>),
    /// `>: lo <: hi = alias`, the right-hand side of an opaque type with bounds
    BoundedAlias(Box<TType>, Box<TType>),
    And(Box<TType>, Box<TType>),
    Or(Box<TType>, Box<TType>),
    ByName(Box<TType>),
    /// The type and the class of the annotation.
    Annotated(Box<TType>, Box<TType>),
    /// `parent { name: info }`; a refinement written as a tree keeps the names of its members.
    Refined(Box<TType>, Vec<(NameRef, Option<TType>)>),
    Rec(Addr, Box<TType>),
    RecThis(Addr),
    Super(Box<TType>, Box<TType>),
    /// A type lambda, or the method and polymorphic method types of refinements. Parameters of a
    /// lambda written as a tree are referred to by their own address (`LocalType`), those of a
    /// lambda type by the binder's address and their position (`ParamRef`).
    Lambda { kind: LambdaKind, binder: Addr, params: Vec<TParam>, result: Box<TType> },
    ParamRef(Addr, u32),
    /// `S match { case P => T ... }` with its declared or inferred bound.
    Match { scrutinee: Box<TType>, bound: Option<Box<TType>>, cases: Vec<TMatchCase> },
    /// `pattern => body`, the result of the type lambda a match type case is written as.
    MatchCase(Box<TType>, Box<TType>),
    /// A type from Java under explicit nulls, `T | Null` or `T` as needed.
    Flexible(Box<TType>),
    Const(Const),
    /// A tag this reader has no reading for in a type position.
    Unknown(u8),
}

/// One case of a match type, `[binders] =>> pattern => body`. Written as a tree, the pattern
/// binds its variables with `BIND` nodes, listed by address with their names and bounds; as a
/// type, a type lambda over the case binds them, whose binder address and parameters the
/// pattern's `ParamRef`s name.
#[derive(Clone, Debug)]
pub struct TMatchCase {
    pub binders: Vec<(Addr, NameRef, Option<TType>)>,
    pub lambda: Option<(Addr, Vec<TParam>)>,
    pub pattern: TType,
    pub body: TType,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LocalKind {
    TypeParam,
    Class,
    Alias,
    /// `opaque type T = ...`, which teq keeps as a class of its own.
    Opaque,
    /// `type T` or `type T >: L <: H`: a type member that subclasses and refinements fix.
    AbstractMember,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LambdaKind {
    Type,
    Poly,
    Method,
}

#[derive(Clone, Debug)]
pub struct TParam {
    /// The address of a `TYPEPARAM`; 0 for the parameter of a lambda type.
    pub addr: Addr,
    pub name: NameRef,
    /// `Bounds`, or a `Lambda` over bounds for a higher-kinded parameter; for a method type the
    /// type of the term parameter.
    pub info: TType,
    pub flags: Flags,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub addr: Addr,
    pub name: NameRef,
    pub ty: TType,
    /// Whether scalac inferred the type.
    pub inferred: bool,
    pub flags: Flags,
}

#[derive(Clone, Debug)]
pub enum Clause {
    Types(Vec<TParam>),
    Terms(Vec<Param>),
}

#[derive(Clone, Debug, Default)]
pub struct Mods {
    pub flags: Flags,
    /// The classes of the annotations.
    pub annots: Vec<TType>,
    pub within: Option<TType>,
    /// The name `@targetName("name")` gives the definition in the class file.
    pub target_name: Option<NameRef>,
    /// `@scala.volatile`, which a field of the definition carries as `ACC_VOLATILE`.
    pub volatile: bool,
}

#[derive(Clone, Debug)]
pub struct DefSig {
    pub addr: Addr,
    pub tag: u8,
    pub name: NameRef,
    pub clauses: Vec<Clause>,
    pub ret: TType,
    /// Whether scalac inferred the result type.
    pub ret_inferred: bool,
    pub mods: Mods,
    pub body: Option<Addr>,
}

impl DefSig {
    /// The signature as the method type a refinement states.
    pub fn as_type(self) -> TType {
        let parameterless = self.tag == DEFDEF && self.clauses.is_empty();
        let mut t = self.ret;
        for clause in self.clauses.into_iter().rev() {
            let (kind, params) = match clause {
                Clause::Types(ps) => (LambdaKind::Poly, ps),
                Clause::Terms(ps) => (
                    LambdaKind::Method,
                    ps.into_iter().map(|p| TParam { addr: p.addr, name: p.name, info: p.ty, flags: p.flags }).collect(),
                ),
            };
            t = TType::Lambda { kind, binder: 0, params, result: Box::new(t) };
        }
        if parameterless {
            TType::ByName(Box::new(t))
        } else {
            t
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeDefSig {
    pub addr: Addr,
    pub name: NameRef,
    /// `Bounds` for an abstract type, a `Lambda` for a parameterized one, the alias otherwise.
    /// An opaque alias with bounds keeps the alias here and the bounds in `opaque_bounds`.
    pub rhs: TType,
    pub opaque_bounds: Option<TType>,
    pub mods: Mods,
}

pub struct ClassSig {
    pub name: NameRef,
    pub tparams: Vec<TParam>,
    /// The parameter accessors, whose modifiers tell a `val` parameter from a plain one.
    pub accessors: Vec<Param>,
    pub parents: Vec<TType>,
    pub self_type: Option<(NameRef, TType)>,
    pub ctor: Option<DefSig>,
    pub mods: Mods,
    pub index: TemplateIndex,
}

pub struct ExportSig {
    pub path: TType,
    /// (name, renamed to); the name `_` is a wildcard, the empty name a given wildcard.
    pub selectors: Vec<(NameRef, Option<NameRef>)>,
}

pub(super) const MAX_DEPTH: u32 = 200;

pub struct Decoder<'a> {
    pub file: &'a TastyFile,
    pub(super) r: Reader<'a>,
    pub(super) depth: u32,
    /// The shared addresses being decoded. A refinement names its own members through
    /// `THIS(SHAREDtype <the refinement>)`, so a shared address can be an enclosing tree.
    pub(super) open_shared: Vec<Addr>,
}

/// The top-level definitions of a file: classes, the vals of objects, and the package clause
/// each stands in.
pub fn index_top_level(file: &TastyFile) -> Vec<TopLevel> {
    let mut d = Decoder::new(file);
    let mut out = Vec::new();
    let end = d.r.bytes.len();
    d.index_package_stats(end, 0, &mut out);
    out
}

impl<'a> Decoder<'a> {
    pub fn new(file: &'a TastyFile) -> Decoder<'a> {
        Decoder { file, r: file.trees(), depth: 0, open_shared: Vec::new() }
    }

    // ---- skipping and indexing ----

    pub(super) fn skip_tree(&mut self) {
        let tag = self.r.byte();
        if tag >= PACKAGE {
            self.r.pos = self.r.end();
        } else if tag >= IDENT {
            self.r.nat();
            self.skip_tree();
        } else if tag >= THIS {
            self.skip_tree();
        } else if tag >= SHAREDTERM {
            self.r.nat();
        }
    }

    fn index_package_stats(&mut self, end: usize, package: NameRef, out: &mut Vec<TopLevel>) {
        while self.r.pos < end {
            match self.r.peek() {
                PACKAGE => {
                    self.r.byte();
                    let inner_end = self.r.end();
                    let Some(name) = self.package_ref() else {
                        self.r.pos = inner_end;
                        continue;
                    };
                    self.index_package_stats(inner_end, name, out);
                    self.r.pos = inner_end;
                }
                VALDEF | DEFDEF | TYPEDEF => {
                    let entry = self.index_entry();
                    out.push(TopLevel { package, entry });
                }
                _ => self.skip_tree(),
            }
        }
    }

    /// The package a `PACKAGE` clause names, which a nested clause refers to through a shared
    /// tree (`package scala` inside `package <empty>` of `scala/package.tasty`).
    fn package_ref(&mut self) -> Option<NameRef> {
        match self.r.byte() {
            TERMREFPKG | TYPEREFPKG => Some(self.r.nat()),
            SHAREDTYPE | SHAREDTERM => {
                let addr = self.r.nat() as usize;
                let mut at = self.r.at(addr);
                match at.byte() {
                    TERMREFPKG | TYPEREFPKG => Some(at.nat()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Reads name and modifiers of the definition at the cursor and leaves the cursor behind it.
    pub(super) fn index_entry(&mut self) -> Entry {
        let addr = self.r.pos as Addr;
        let tag = self.r.byte();
        let end = self.r.end();
        let name = self.r.nat();
        let (is_class, has_body) = self.skip_to_modifiers(tag, end);
        let mut flags = Flags::default();
        let mut qualified_private = false;
        let mut access_within = None;
        let mut js_annotated = false;
        while self.r.pos < end {
            let m = self.r.byte();
            match m {
                ANNOTATION => {
                    let annot_end = self.r.end();
                    js_annotated |= self.names_js_annotation();
                    self.r.pos = annot_end;
                }
                // `private[p]` and `protected[p]` are reachable from all of `p`, which the
                // library was checked for: neither is the plain modifier.
                PRIVATEQUALIFIED | PROTECTEDQUALIFIED => {
                    qualified_private |= m == PRIVATEQUALIFIED;
                    access_within = self.qualifier_name();
                    self.skip_tree();
                }
                m if m < 64 => flags.set(m),
                _ => break,
            }
        }
        self.r.pos = end;
        Entry { addr, tag, name, flags, is_class, has_body, qualified_private, access_within, js_annotated }
    }

    /// The name of the class or package a qualifier type at the cursor refers to.
    fn qualifier_name(&self) -> Option<NameRef> {
        let mut r = self.r.at(self.r.pos);
        loop {
            match r.byte() {
                SHAREDTYPE => {
                    let to = r.nat() as usize;
                    r = r.at(to);
                }
                TYPEREFPKG | TERMREFPKG | TYPEREF => return Some(r.nat()),
                TYPEREFDIRECT | TYPEREFSYMBOL => {
                    let to = r.nat() as usize;
                    let mut def = r.at(to);
                    def.byte();
                    def.end();
                    return Some(def.nat());
                }
                _ => return None,
            }
        }
    }

    /// Skips the parameters, type and right-hand side of a definition whose name was just read;
    /// whether it is a class, and whether a val or def has a body.
    fn skip_to_modifiers(&mut self, tag: u8, end: usize) -> (bool, bool) {
        match tag {
            DEFDEF => {
                while matches!(self.r.peek(), TYPEPARAM | PARAM | EMPTYCLAUSE | SPLITCLAUSE) && self.r.pos < end {
                    self.skip_tree();
                }
                self.skip_tree();
                (false, self.skip_rhs(end).is_some())
            }
            VALDEF => {
                self.skip_tree();
                (false, self.skip_rhs(end).is_some())
            }
            _ => {
                let is_class = self.r.peek() == TEMPLATE;
                self.skip_tree();
                (is_class, false)
            }
        }
    }

    /// Whether the annotation class at the cursor, a type reference, has the name of a Scala.js
    /// annotation; the cursor is left where it was.
    fn names_js_annotation(&mut self) -> bool {
        let saved = self.r.pos;
        let mut hops = 0;
        let found = loop {
            match self.r.byte() {
                SHAREDTYPE if hops < 8 => {
                    let addr = self.r.nat();
                    self.r.pos = addr as usize;
                    hops += 1;
                }
                TYPEREF => {
                    let n = self.r.nat();
                    break self.file.simple(n).map_or(false, |s| JS_ANNOTATION_NAMES.contains(&s));
                }
                _ => break false,
            }
        };
        self.r.pos = saved;
        found
    }

    /// The annotations of the definition of `e`, with their arguments.
    pub fn annotations(&mut self, e: &Entry) -> Vec<Annot> {
        self.r.pos = e.addr as usize;
        let tag = self.r.byte();
        let end = self.r.end();
        self.r.nat();
        self.skip_to_modifiers(tag, end);
        let mut out = Vec::new();
        while self.r.pos < end {
            match self.r.byte() {
                ANNOTATION => {
                    let annot_end = self.r.end();
                    let class = self.read_type();
                    let args = if self.r.pos < annot_end { self.annotation_args() } else { Vec::new() };
                    out.push(Annot { class, args });
                    self.r.pos = annot_end;
                }
                PRIVATEQUALIFIED | PROTECTEDQUALIFIED => self.skip_tree(),
                m if m < 64 => {}
                _ => break,
            }
        }
        self.r.pos = end;
        out
    }

    /// The arguments of the constructor call `new A(args)` at the cursor.
    fn annotation_args(&mut self) -> Vec<AnnotArg> {
        match self.r.peek() {
            SHAREDTERM => {
                self.r.byte();
                let addr = self.r.nat();
                let saved = self.r.pos;
                self.r.pos = addr as usize;
                let args = self.annotation_args();
                self.r.pos = saved;
                args
            }
            APPLY => {
                self.r.byte();
                let end = self.r.end();
                self.skip_tree();
                let mut args = Vec::new();
                while self.r.pos < end {
                    args.push(self.annotation_arg());
                }
                self.r.pos = end;
                args
            }
            _ => {
                self.skip_tree();
                Vec::new()
            }
        }
    }

    fn annotation_arg(&mut self) -> AnnotArg {
        match self.r.peek() {
            STRINGCONST => {
                self.r.byte();
                AnnotArg::Str(self.r.nat())
            }
            NAMEDARG => {
                self.r.byte();
                self.r.nat();
                self.annotation_arg()
            }
            IDENT | SELECT | SELECTIN | TERMREF | TERMREFPKG | TERMREFDIRECT | TERMREFSYMBOL | SHAREDTERM => match self.read_path() {
                TType::Const(Const::Str(s)) => AnnotArg::Str(s),
                path => AnnotArg::Path(path),
            },
            _ => {
                self.skip_tree();
                AnnotArg::Other
            }
        }
    }

    /// The type parameters of the class at `addr`, decoded with their bounds, and nothing else
    /// of its template.
    pub fn class_tparams(&mut self, addr: Addr) -> Vec<TParam> {
        self.r.pos = addr as usize;
        self.r.byte();
        let end = self.r.end();
        self.r.nat();
        let mut out = Vec::new();
        if self.r.peek() != TEMPLATE {
            self.r.pos = end;
            return out;
        }
        self.r.byte();
        let template_end = self.r.end();
        while self.r.peek() == TYPEPARAM && self.r.pos < template_end {
            out.push(self.tparam());
        }
        self.r.pos = end;
        out
    }

    /// The parents of the class at `addr`, and nothing else of its template decoded.
    pub fn class_parents(&mut self, addr: Addr) -> Vec<TType> {
        self.r.pos = addr as usize;
        self.r.byte();
        let end = self.r.end();
        self.r.nat();
        let mut out = Vec::new();
        if self.r.peek() != TEMPLATE {
            self.r.pos = end;
            return out;
        }
        self.r.byte();
        let template_end = self.r.end();
        while matches!(self.r.peek(), TYPEPARAM | PARAM) && self.r.pos < template_end {
            self.skip_tree();
        }
        while !matches!(self.r.peek(), DEFDEF | SPLITCLAUSE | SELFDEF) && self.r.pos < template_end {
            out.push(self.parent());
        }
        self.r.pos = end;
        out
    }

    /// The type definitions of the template at `addr` (nested classes, aliases, opaque and
    /// abstract types) and the vals of its objects, by their entries; nothing is decoded.
    pub fn nested_types(&mut self, addr: Addr) -> Vec<Entry> {
        self.r.pos = addr as usize;
        self.r.byte();
        let end = self.r.end();
        self.r.nat();
        if self.r.peek() != TEMPLATE {
            self.r.pos = end;
            return Vec::new();
        }
        self.r.byte();
        let template_end = self.r.end();
        let mut out = Vec::new();
        while self.r.pos < template_end {
            match self.r.peek() {
                TYPEDEF => out.push(self.index_entry()),
                VALDEF | DEFDEF => {
                    let e = self.index_entry();
                    if e.tag == VALDEF && e.flags.has(OBJECT) {
                        out.push(e);
                    }
                }
                _ => self.skip_tree(),
            }
        }
        self.r.pos = end;
        out
    }

    pub(super) fn skip_rhs(&mut self, end: usize) -> Option<Addr> {
        if self.r.pos < end && !is_modifier(self.r.peek()) {
            let at = self.r.pos as Addr;
            self.skip_tree();
            return Some(at);
        }
        None
    }

    pub(super) fn index_stats(&mut self, end: usize) -> TemplateIndex {
        let mut index = TemplateIndex { members: Vec::new(), exports: Vec::new(), statements: false };
        while self.r.pos < end {
            match self.r.peek() {
                VALDEF | DEFDEF | TYPEDEF => index.members.push(self.index_entry()),
                EXPORT => {
                    index.exports.push(self.r.pos as Addr);
                    self.skip_tree();
                }
                IMPORT => self.skip_tree(),
                _ => {
                    index.statements = true;
                    self.skip_tree();
                }
            }
        }
        index
    }

    /// The annotations of the definition at `addr`, each as the address of its class and of its
    /// tree, the constructor call, for a reader of the trees.
    pub fn annotation_addrs(&mut self, addr: Addr) -> Vec<(Addr, Addr)> {
        self.r.pos = addr as usize;
        let tag = self.r.byte();
        let end = self.r.end();
        self.r.nat();
        self.skip_to_modifiers(tag, end);
        let mut out = Vec::new();
        while self.r.pos < end {
            match self.r.byte() {
                ANNOTATION => {
                    let annot_end = self.r.end();
                    let class = self.r.pos as Addr;
                    self.skip_tree();
                    out.push((class, self.r.pos as Addr));
                    self.r.pos = annot_end;
                }
                PRIVATEQUALIFIED | PROTECTEDQUALIFIED => self.skip_tree(),
                m if m < 64 => {}
                _ => break,
            }
        }
        self.r.pos = end;
        out
    }

    /// The type of the class a constructor call `new P[Ts](args)` at `addr` constructs.
    pub fn call_type(&mut self, addr: Addr) -> TType {
        self.r.pos = addr as usize;
        self.parent_type()
    }

    /// The type or type tree at `addr`.
    pub fn type_at(&mut self, addr: Addr) -> TType {
        self.r.pos = addr as usize;
        self.read_type()
    }

    /// The address after the tree at `addr`, whatever its kind.
    pub fn skip_at(&mut self, addr: Addr) -> Addr {
        self.r.pos = addr as usize;
        self.skip_tree();
        self.r.pos as Addr
    }

    /// The name of the definition or binding at `addr`, for a reference by address.
    pub fn name_at(&self, addr: Addr) -> Option<NameRef> {
        let mut r = self.r.at(addr as usize);
        match r.byte() {
            TYPEPARAM | PARAM | VALDEF | DEFDEF | TYPEDEF | BIND => {
                r.end();
                Some(r.nat())
            }
            _ => None,
        }
    }

    /// The address after the tree that starts at `addr`.
    pub fn tree_end(&self, addr: Addr) -> Addr {
        let mut r = self.r.at(addr as usize);
        r.byte();
        r.end() as Addr
    }

    /// The type a `BIND` at `addr` gives its symbol: a type pattern's variable has its bounds.
    pub fn bind_type(&self, addr: Addr) -> Option<TType> {
        let mut d = Decoder { file: self.file, r: self.r.at(addr as usize), depth: 0, open_shared: Vec::new() };
        if d.r.byte() != BIND {
            return None;
        }
        d.r.end();
        d.r.nat();
        Some(d.read_type())
    }

    /// The address of the tree at `addr`, or of the tree a shared reference there points to.
    pub fn shared_target(&self, addr: Addr) -> Addr {
        let mut r = self.r.at(addr as usize);
        match r.byte() {
            SHAREDTYPE | SHAREDTERM => r.nat() as Addr,
            _ => addr,
        }
    }

    pub fn tag_at(&self, addr: Addr) -> u8 {
        self.r.at(addr as usize).peek()
    }

    /// The bounds of the type variable a `BIND` at `addr` introduces.
    fn bind_bounds(&self, addr: Addr) -> Option<TType> {
        let mut d = Decoder { file: self.file, r: self.r.at(addr as usize), depth: 0, open_shared: Vec::new() };
        if d.r.byte() != BIND {
            return None;
        }
        let end = d.r.end();
        d.r.nat();
        (d.r.pos < end && d.r.peek() == TYPEBOUNDSTPT).then(|| d.read_tpt())
    }

    /// Whether the definition at `addr` is the val of an object, which names a place in the
    /// program as the object's class does.
    pub fn is_module_val(&self, addr: Addr) -> bool {
        let mut d = Decoder { file: self.file, r: self.r.at(addr as usize), depth: 0, open_shared: Vec::new() };
        d.r.peek() == VALDEF && d.index_entry().flags.has(OBJECT)
    }

    /// What a type referred to by the address of its definition is.
    pub fn local_type_kind(&self, addr: Addr) -> LocalKind {
        let mut d = Decoder { file: self.file, r: self.r.at(addr as usize), depth: 0, open_shared: Vec::new() };
        match d.r.byte() {
            TYPEPARAM => LocalKind::TypeParam,
            BIND => LocalKind::TypeParam,
            TYPEDEF => {
                let end = d.r.end();
                d.r.nat();
                let mut rhs = d.r.peek();
                if rhs == TEMPLATE {
                    return LocalKind::Class;
                }
                let rhs_at = d.r.pos;
                if rhs == LAMBDATPT {
                    d.r.byte();
                    d.r.end();
                    while d.r.peek() == TYPEPARAM {
                        d.skip_tree();
                    }
                    rhs = d.r.peek();
                }
                d.r.pos = rhs_at;
                d.skip_tree();
                let mut opaque = false;
                while d.r.pos < end {
                    match d.r.byte() {
                        OPAQUE => opaque = true,
                        ANNOTATION => d.r.pos = d.r.end(),
                        PRIVATEQUALIFIED | PROTECTEDQUALIFIED => d.skip_tree(),
                        _ => {}
                    }
                }
                if opaque {
                    LocalKind::Opaque
                } else if matches!(rhs, TYPEBOUNDSTPT | TYPEBOUNDS) {
                    LocalKind::AbstractMember
                } else {
                    LocalKind::Alias
                }
            }
            _ => LocalKind::Other,
        }
    }

    // ---- definitions ----

    pub(super) fn modifiers(&mut self, end: usize) -> Mods {
        let mut mods = Mods::default();
        while self.r.pos < end {
            let m = self.r.byte();
            match m {
                ANNOTATION => {
                    let annot_end = self.r.end();
                    let class = self.read_type();
                    let call = self.r.pos;
                    if let TType::TypeRef(prefix, n) = &class {
                        mods.volatile |= self.file.simple(*n) == Some("volatile") && matches!(**prefix, TType::Package(p) if self.file.simple(p) == Some("scala"));
                    }
                    if matches!(&class, TType::TypeRef(_, n) if self.file.simple(*n) == Some("targetName")) && call < annot_end {
                        mods.target_name = match self.annotation_args().first() {
                            Some(AnnotArg::Str(n)) => Some(*n),
                            _ => None,
                        };
                        self.r.pos = call;
                    }
                    // `@Child[C]` keeps its type argument in the constructor call alone.
                    let applied = if self.r.pos < annot_end { self.annotation_class() } else { TType::Unknown(0) };
                    mods.annots.push(if matches!(applied, TType::Applied(..)) { applied } else { class });
                    self.r.pos = annot_end;
                }
                PRIVATEQUALIFIED | PROTECTEDQUALIFIED => {
                    mods.flags.set(if m == PRIVATEQUALIFIED { PRIVATE } else { PROTECTED });
                    mods.within = Some(self.read_type());
                }
                m if m < 64 => mods.flags.set(m),
                _ => break,
            }
        }
        self.r.pos = end;
        mods
    }

    pub(super) fn tparam(&mut self) -> TParam {
        let addr = self.r.pos as Addr;
        self.r.byte();
        let end = self.r.end();
        let name = self.r.nat();
        let info = self.read_tpt();
        let flags = self.modifiers(end).flags;
        TParam { addr, name, info, flags }
    }

    pub(super) fn param(&mut self) -> Param {
        let addr = self.r.pos as Addr;
        self.r.byte();
        let end = self.r.end();
        let name = self.r.nat();
        let inferred = self.tpt_is_inferred();
        let ty = self.read_tpt();
        let mods = self.modifiers(end);
        let mut flags = mods.flags;
        // A `private[p]` parameter is a member outside its class, as the index reads one.
        if mods.within.is_some() {
            flags.clear(PRIVATE);
            flags.clear(PROTECTED);
        }
        Param { addr, name, ty, inferred, flags }
    }

    pub(super) fn clauses(&mut self, end: usize) -> Vec<Clause> {
        let mut out: Vec<Clause> = Vec::new();
        let mut open = false;
        while self.r.pos < end {
            match self.r.peek() {
                TYPEPARAM => {
                    let p = self.tparam();
                    match out.last_mut() {
                        Some(Clause::Types(ps)) if open => ps.push(p),
                        _ => out.push(Clause::Types(vec![p])),
                    }
                    open = true;
                }
                PARAM => {
                    let p = self.param();
                    match out.last_mut() {
                        Some(Clause::Terms(ps)) if open => ps.push(p),
                        _ => out.push(Clause::Terms(vec![p])),
                    }
                    open = true;
                }
                EMPTYCLAUSE => {
                    self.r.byte();
                    out.push(Clause::Terms(Vec::new()));
                    open = false;
                }
                SPLITCLAUSE => {
                    self.r.byte();
                    open = false;
                }
                _ => break,
            }
        }
        out
    }

    /// The signature of the `DEFDEF` or `VALDEF` at `addr`.
    pub fn def_sig(&mut self, addr: Addr) -> DefSig {
        self.r.pos = addr as usize;
        let tag = self.r.byte();
        let end = self.r.end();
        let name = self.r.nat();
        let clauses = if tag == DEFDEF { self.clauses(end) } else { Vec::new() };
        let ret_inferred = self.tpt_is_inferred();
        let ret = self.read_tpt();
        let body = self.skip_rhs(end);
        let mods = self.modifiers(end);
        DefSig { addr, tag, name, clauses, ret, ret_inferred, mods, body }
    }

    /// The `TYPEDEF` at `addr` of a type member, alias or opaque type.
    pub fn type_def_sig(&mut self, addr: Addr) -> TypeDefSig {
        self.r.pos = addr as usize;
        self.r.byte();
        let end = self.r.end();
        let name = self.r.nat();
        let (rhs, opaque_bounds) = match self.read_tpt() {
            TType::BoundedAlias(bounds, alias) => (*alias, Some(*bounds)),
            rhs => (rhs, None),
        };
        let mods = self.modifiers(end);
        TypeDefSig { addr, name, rhs, opaque_bounds, mods }
    }

    /// The header of the class at `addr` (a `TYPEDEF` with a template) and the index of its
    /// members.
    pub fn class_sig(&mut self, addr: Addr) -> ClassSig {
        self.r.pos = addr as usize;
        self.r.byte();
        let end = self.r.end();
        let name = self.r.nat();
        let mut sig = ClassSig {
            name,
            tparams: Vec::new(),
            accessors: Vec::new(),
            parents: Vec::new(),
            self_type: None,
            ctor: None,
            mods: Mods::default(),
            index: TemplateIndex { members: Vec::new(), exports: Vec::new(), statements: false },
        };
        if self.r.peek() != TEMPLATE {
            self.r.pos = end;
            return sig;
        }
        self.r.byte();
        let template_end = self.r.end();
        while self.r.peek() == TYPEPARAM && self.r.pos < template_end {
            let p = self.tparam();
            sig.tparams.push(p);
        }
        while self.r.peek() == PARAM && self.r.pos < template_end {
            let p = self.param();
            sig.accessors.push(p);
        }
        while !matches!(self.r.peek(), DEFDEF | SPLITCLAUSE | SELFDEF) && self.r.pos < template_end {
            let p = self.parent();
            sig.parents.push(p);
        }
        if self.r.peek() == SPLITCLAUSE {
            self.r.byte();
        }
        if self.r.peek() == SELFDEF {
            self.r.byte();
            let self_name = self.r.nat();
            let ty = self.read_tpt();
            sig.self_type = Some((self_name, ty));
        }
        if self.r.peek() == DEFDEF && self.r.pos < template_end {
            let ctor_addr = self.r.pos as Addr;
            let ctor = self.def_sig(ctor_addr);
            sig.ctor = Some(ctor);
        }
        sig.index = self.index_stats(template_end);
        self.r.pos = template_end;
        sig.mods = self.modifiers(end);
        sig
    }

    /// A parent is a type tree, or the constructor call `new P[Ts](args)` whose `NEW` holds it.
    fn parent(&mut self) -> TType {
        let start = self.r.pos;
        let t = self.parent_type();
        self.r.pos = start;
        self.skip_tree();
        t
    }

    fn parent_type(&mut self) -> TType {
        match self.r.peek() {
            // `new P[Ts]` with inferred arguments names `P` in its `NEW` and `Ts` here.
            TYPEAPPLY => {
                self.r.byte();
                let end = self.r.end();
                let fn_start = self.r.pos;
                let t = self.parent_type();
                if matches!(t, TType::Applied(..)) {
                    return t;
                }
                self.r.pos = fn_start;
                self.skip_tree();
                let mut args = Vec::new();
                while self.r.pos < end {
                    args.push(self.read_tpt());
                }
                TType::Applied(Box::new(t), args)
            }
            // Each starts with the tree to descend into.
            APPLY | BLOCK | INLINED | TYPED => {
                self.r.byte();
                self.r.end();
                self.parent_type()
            }
            SELECTIN => {
                self.r.byte();
                self.r.end();
                self.r.nat();
                self.parent_type()
            }
            SELECT => {
                self.r.byte();
                self.r.nat();
                self.parent_type()
            }
            NEW => {
                self.r.byte();
                self.read_tpt()
            }
            // An annotation that repeats within a file is pickled once and referred to.
            SHAREDTERM => {
                self.r.byte();
                let addr = self.r.nat();
                self.shared(addr, |d| d.parent_type())
            }
            _ => self.read_tpt(),
        }
    }

    pub fn export_sig(&mut self, addr: Addr) -> ExportSig {
        self.r.pos = addr as usize;
        self.r.byte();
        let end = self.r.end();
        let path = self.read_path();
        let mut selectors: Vec<(NameRef, Option<NameRef>)> = Vec::new();
        while self.r.pos < end {
            match self.r.byte() {
                IMPORTED => selectors.push((self.r.nat(), None)),
                RENAMED => {
                    let to = self.r.nat();
                    if let Some(last) = selectors.last_mut() {
                        last.1 = Some(to);
                    }
                }
                BOUNDED => self.skip_tree(),
                _ => break,
            }
        }
        ExportSig { path, selectors }
    }

    // ---- types ----

    /// Runs a reader one level down; past the depth limit the tree is skipped instead, so that
    /// the cursor moves on either way.
    fn nested(&mut self, f: impl FnOnce(&mut Self) -> TType) -> TType {
        if self.depth >= MAX_DEPTH {
            self.skip_tree();
            return TType::Unknown(0);
        }
        self.depth += 1;
        let t = f(self);
        self.depth -= 1;
        t
    }

    /// Decodes the tree at `addr` and comes back to where the cursor stood. A reference to a
    /// tree that is being decoded is the self-reference of a recursive type.
    fn shared(&mut self, addr: Addr, f: impl FnOnce(&mut Self) -> TType) -> TType {
        if self.open_shared.contains(&addr) {
            return TType::RecThis(addr);
        }
        let saved = self.r.pos;
        self.r.pos = addr as usize;
        self.open_shared.push(addr);
        let t = self.nested(f);
        self.open_shared.pop();
        self.r.pos = saved;
        t
    }

    /// A type tree or a type.
    /// Whether the type tree at the reader is a type scalac inferred, pickled as the type alone,
    /// rather than one the source wrote.
    pub fn tpt_is_inferred(&self) -> bool {
        let mut at = self.r.pos as Addr;
        loop {
            match self.tag_at(at) {
                SHAREDTERM => at = self.r.at(at as usize + 1).nat() as Addr,
                IDENTTPT | SELECTTPT | SINGLETONTPT | BYNAMETPT | APPLIEDTPT | LAMBDATPT | TYPEBOUNDSTPT | ANNOTATEDTPT | REFINEDTPT | MATCHTPT => return false,
                _ => return true,
            }
        }
    }

    pub fn read_tpt(&mut self) -> TType {
        self.nested(|d| d.read_tpt_now())
    }

    fn read_tpt_now(&mut self) -> TType {
        let tag = self.r.peek();
        match tag {
            IDENTTPT => {
                self.r.byte();
                self.r.nat();
                self.read_type()
            }
            SELECTTPT => {
                self.r.byte();
                let name = self.r.nat();
                let qual = self.read_path();
                TType::TypeRef(Box::new(qual), name)
            }
            SINGLETONTPT => {
                self.r.byte();
                self.read_path()
            }
            BYNAMETPT => {
                self.r.byte();
                TType::ByName(Box::new(self.read_tpt()))
            }
            SHAREDTERM => {
                self.r.byte();
                let addr = self.r.nat();
                self.shared(addr, |d| d.read_tpt_now())
            }
            APPLIEDTPT => {
                self.r.byte();
                let end = self.r.end();
                let tycon = self.read_tpt();
                let mut args = Vec::new();
                while self.r.pos < end {
                    args.push(self.read_tpt());
                }
                self.applied(tycon, args)
            }
            LAMBDATPT => {
                let binder = self.r.pos as Addr;
                self.r.byte();
                let end = self.r.end();
                let mut params = Vec::new();
                while self.r.peek() == TYPEPARAM && self.r.pos < end {
                    params.push(self.tparam());
                }
                let result = self.read_tpt();
                self.r.pos = end;
                TType::Lambda { kind: LambdaKind::Type, binder, params, result: Box::new(result) }
            }
            TYPEBOUNDSTPT => {
                self.r.byte();
                let end = self.r.end();
                let lo = self.read_tpt();
                let hi = if self.r.pos < end { Some(self.read_tpt()) } else { None };
                let alias = if self.r.pos < end { Some(self.read_tpt()) } else { None };
                self.r.pos = end;
                match (hi, alias) {
                    (None, _) => TType::Alias(Box::new(lo)),
                    (Some(hi), None) => TType::Bounds(Box::new(lo), Box::new(hi)),
                    (Some(hi), Some(alias)) => {
                        TType::BoundedAlias(Box::new(TType::Bounds(Box::new(lo), Box::new(hi))), Box::new(alias))
                    }
                }
            }
            ANNOTATEDTPT => {
                self.r.byte();
                let end = self.r.end();
                let underlying = self.read_tpt();
                let annot = self.annotation_class();
                self.r.pos = end;
                TType::Annotated(Box::new(underlying), Box::new(annot))
            }
            REFINEDTPT => {
                self.r.byte();
                let end = self.r.end();
                let parent = self.read_tpt();
                let mut members = Vec::new();
                while self.r.pos < end {
                    let at = self.r.pos as Addr;
                    match self.r.peek() {
                        TYPEDEF => {
                            let sig = self.type_def_sig(at);
                            let info = match sig.rhs {
                                info @ (TType::Bounds(..) | TType::Lambda { .. }) => info,
                                alias => TType::Alias(Box::new(alias)),
                            };
                            members.push((sig.name, Some(info)));
                        }
                        VALDEF | DEFDEF => {
                            let sig = self.def_sig(at);
                            members.push((sig.name, Some(sig.as_type())));
                        }
                        _ => self.skip_tree(),
                    }
                }
                TType::Refined(Box::new(parent), members)
            }
            MATCHTPT => {
                self.r.byte();
                let end = self.r.end();
                // `bound? sel CaseDef*`: the selector is the last tree before the cases.
                let mut scrutinee = self.read_tpt();
                let mut bound = None;
                if self.r.pos < end && self.r.peek() != CASEDEF_TAG {
                    bound = Some(Box::new(scrutinee));
                    scrutinee = self.read_tpt();
                }
                let mut cases = Vec::new();
                while self.r.pos < end && self.r.peek() == CASEDEF_TAG {
                    self.r.byte();
                    let case_end = self.r.end();
                    let pattern = self.read_tpt();
                    let body = self.read_tpt();
                    self.r.pos = case_end;
                    let mut addrs = Vec::new();
                    collect_binders(&pattern, &mut addrs);
                    let binders = addrs
                        .into_iter()
                        .filter(|&a| self.tag_at(a) == BIND)
                        .filter_map(|a| Some((a, self.name_at(a)?, self.bind_bounds(a))))
                        .collect();
                    cases.push(TMatchCase { binders, lambda: None, pattern, body });
                }
                self.r.pos = end;
                TType::Match { scrutinee: Box::new(scrutinee), bound, cases }
            }
            BLOCK | INLINED | TYPED => {
                // `{ type T = ...; T }` and the like: the expression comes first.
                self.r.byte();
                let end = self.r.end();
                let t = self.read_tpt();
                self.r.pos = end;
                t
            }
            _ => self.read_type_now(),
        }
    }

    /// `A | B` and `A & B` are written as applications of the types `|` and `&` of package `scala`.
    fn applied(&self, tycon: TType, mut args: Vec<TType>) -> TType {
        if args.len() == 2 {
            if let TType::TypeRef(prefix, n) = &tycon {
                let in_scala = matches!(&**prefix, TType::Package(p) if self.file.simple(*p) == Some("scala"));
                let op = if in_scala { self.file.simple(*n) } else { None };
                if let Some(op @ ("|" | "&")) = op {
                    let b = Box::new(args.pop().unwrap());
                    let a = Box::new(args.pop().unwrap());
                    return if op == "|" { TType::Or(a, b) } else { TType::And(a, b) };
                }
            }
        }
        TType::Applied(Box::new(tycon), args)
    }

    /// The class of the annotation whose application `new A(args)` is at the cursor.
    fn annotation_class(&mut self) -> TType {
        self.parent_type()
    }

    /// A term that is a path: what a singleton type, a prefix or an export names.
    pub fn read_path(&mut self) -> TType {
        self.nested(|d| d.read_path_now())
    }

    fn read_path_now(&mut self) -> TType {
        match self.r.peek() {
            IDENT => {
                self.r.byte();
                self.r.nat();
                self.read_type()
            }
            SELECT => {
                self.r.byte();
                let name = self.r.nat();
                let qual = self.read_path();
                TType::TermRef(Box::new(qual), name)
            }
            SELECTIN => {
                self.r.byte();
                let end = self.r.end();
                let name = self.r.nat();
                let qual = self.read_path();
                self.r.pos = end;
                TType::TermRef(Box::new(qual), name)
            }
            QUALTHIS => {
                self.r.byte();
                TType::This(Box::new(self.read_tpt()))
            }
            SHAREDTERM => {
                self.r.byte();
                let addr = self.r.nat();
                self.shared(addr, |d| d.read_path_now())
            }
            TYPED | BLOCK | INLINED => {
                self.r.byte();
                let end = self.r.end();
                let t = self.read_path();
                self.r.pos = end;
                t
            }
            _ => self.read_type_now(),
        }
    }

    pub fn read_type(&mut self) -> TType {
        self.nested(|d| d.read_type_now())
    }

    fn boxed_type(&mut self) -> Box<TType> {
        Box::new(self.read_type())
    }

    fn read_type_now(&mut self) -> TType {
        let start = self.r.pos as Addr;
        let tag = self.r.byte();
        match tag {
            SHAREDTYPE => {
                let addr = self.r.nat();
                self.shared(addr, |d| d.read_type_now())
            }
            SHAREDTERM => {
                let addr = self.r.nat();
                self.shared(addr, |d| d.read_tpt_now())
            }
            TYPEREFPKG | TERMREFPKG => TType::Package(self.r.nat()),
            TYPEREFDIRECT => TType::LocalType(self.r.nat(), None),
            TERMREFDIRECT => TType::LocalTerm(self.r.nat(), None),
            TYPEREFSYMBOL => {
                let addr = self.r.nat();
                TType::LocalType(addr, Some(self.boxed_type()))
            }
            TERMREFSYMBOL => {
                let addr = self.r.nat();
                TType::LocalTerm(addr, Some(self.boxed_type()))
            }
            TYPEREF => {
                let name = self.r.nat();
                TType::TypeRef(self.boxed_type(), name)
            }
            TERMREF => {
                let name = self.r.nat();
                TType::TermRef(self.boxed_type(), name)
            }
            TYPEREFIN | TERMREFIN => {
                let end = self.r.end();
                let name = self.r.nat();
                let prefix = self.boxed_type();
                self.r.pos = end;
                if tag == TYPEREFIN {
                    TType::TypeRef(prefix, name)
                } else {
                    TType::TermRef(prefix, name)
                }
            }
            THIS => TType::This(self.boxed_type()),
            RECTHIS => TType::RecThis(self.r.nat()),
            RECTYPE => TType::Rec(start, self.boxed_type()),
            BYNAMETYPE => TType::ByName(self.boxed_type()),
            SUPERTYPE => {
                let end = self.r.end();
                let this = self.boxed_type();
                let underlying = self.boxed_type();
                self.r.pos = end;
                TType::Super(this, underlying)
            }
            REFINEDTYPE => {
                let end = self.r.end();
                let name = self.r.nat();
                let parent = self.read_type();
                let info = self.read_type();
                self.r.pos = end;
                match parent {
                    TType::Refined(p, mut members) => {
                        members.push((name, Some(info)));
                        TType::Refined(p, members)
                    }
                    parent => TType::Refined(Box::new(parent), vec![(name, Some(info))]),
                }
            }
            APPLIEDTYPE => {
                let end = self.r.end();
                let tycon = self.read_type();
                let mut args = Vec::new();
                while self.r.pos < end {
                    args.push(self.read_type());
                }
                self.applied(tycon, args)
            }
            TYPEBOUNDS => {
                let end = self.r.end();
                let lo = self.boxed_type();
                let t = if self.r.pos < end && !is_modifier(self.r.peek()) {
                    let mut hi = self.boxed_type();
                    // The declared variances of a higher-kinded bound, one byte per parameter
                    // of the lambda: STABLE, COVARIANT or CONTRAVARIANT.
                    if let TType::Lambda { params, .. } = &mut *hi {
                        let mut i = 0;
                        while self.r.pos < end {
                            let m = self.r.byte();
                            if let Some(p) = params.get_mut(i) {
                                if m == COVARIANT || m == CONTRAVARIANT {
                                    p.flags.set(m);
                                }
                            }
                            i += 1;
                        }
                    }
                    TType::Bounds(lo, hi)
                } else {
                    TType::Alias(lo)
                };
                self.r.pos = end;
                t
            }
            ANNOTATEDTYPE => {
                let end = self.r.end();
                let underlying = self.boxed_type();
                let annot = self.annotation_class();
                self.r.pos = end;
                TType::Annotated(underlying, Box::new(annot))
            }
            ANDTYPE | ORTYPE => {
                let end = self.r.end();
                let (a, b) = (self.boxed_type(), self.boxed_type());
                self.r.pos = end;
                if tag == ANDTYPE {
                    TType::And(a, b)
                } else {
                    TType::Or(a, b)
                }
            }
            MATCHTYPE => {
                let end = self.r.end();
                let bound = self.boxed_type();
                let scrutinee = self.boxed_type();
                let mut cases = Vec::new();
                while self.r.pos < end {
                    match self.read_type() {
                        TType::MatchCase(pattern, body) => cases.push(TMatchCase { binders: Vec::new(), lambda: None, pattern: *pattern, body: *body }),
                        TType::Lambda { binder, params, result, .. } => match *result {
                            TType::MatchCase(pattern, body) => {
                                cases.push(TMatchCase { binders: Vec::new(), lambda: Some((binder, params)), pattern: *pattern, body: *body })
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                TType::Match { scrutinee, bound: Some(bound), cases }
            }
            MATCHCASETYPE => {
                let end = self.r.end();
                let (pattern, body) = (self.boxed_type(), self.boxed_type());
                self.r.pos = end;
                TType::MatchCase(pattern, body)
            }
            FLEXIBLETYPE => {
                let end = self.r.end();
                let t = self.boxed_type();
                self.r.pos = end;
                TType::Flexible(t)
            }
            PARAMTYPE => {
                let end = self.r.end();
                let binder = self.r.nat();
                let num = self.r.nat();
                self.r.pos = end;
                TType::ParamRef(binder, num)
            }
            POLYTYPE | METHODTYPE | TYPELAMBDATYPE => {
                let end = self.r.end();
                let result_at = self.r.pos;
                self.skip_tree();
                let mut params = Vec::new();
                while self.r.pos < end && !is_modifier(self.r.peek()) {
                    let info = self.read_type();
                    let name = self.r.nat();
                    params.push(TParam { addr: 0, name, info, flags: Flags::default() });
                }
                let mut flags = Flags::default();
                while self.r.pos < end {
                    let m = self.r.byte();
                    if m < 64 {
                        flags.set(m);
                    }
                }
                for p in &mut params {
                    p.flags = flags;
                }
                self.r.pos = result_at;
                let result = self.boxed_type();
                self.r.pos = end;
                let kind = match tag {
                    POLYTYPE => LambdaKind::Poly,
                    METHODTYPE => LambdaKind::Method,
                    _ => LambdaKind::Type,
                };
                TType::Lambda { kind, binder: start, params, result }
            }
            UNITCONST => TType::Const(Const::Unit),
            FALSECONST => TType::Const(Const::Bool(false)),
            TRUECONST => TType::Const(Const::Bool(true)),
            NULLCONST => TType::Const(Const::Null),
            BYTECONST => TType::Const(Const::Byte(self.r.long_int() as i32)),
            SHORTCONST => TType::Const(Const::Short(self.r.long_int() as i32)),
            CHARCONST => TType::Const(Const::Char(self.r.nat())),
            INTCONST => TType::Const(Const::Int(self.r.long_int() as i32)),
            LONGCONST => TType::Const(Const::Long(self.r.long_int())),
            FLOATCONST => TType::Const(Const::Float(self.r.long_int() as u32)),
            DOUBLECONST => TType::Const(Const::Double(self.r.long_int() as u64)),
            STRINGCONST => TType::Const(Const::Str(self.r.nat())),
            CLASSCONST => TType::Const(Const::Class(self.boxed_type())),
            // Type trees where a type was expected: the tag has not been consumed by them.
            IDENTTPT | SELECTTPT | SINGLETONTPT | BYNAMETPT | APPLIEDTPT | LAMBDATPT | TYPEBOUNDSTPT
            | ANNOTATEDTPT | REFINEDTPT | MATCHTPT => {
                self.r.pos = start as usize;
                self.read_tpt_now()
            }
            IDENT | SELECT | SELECTIN | QUALTHIS => {
                self.r.pos = start as usize;
                self.read_path_now()
            }
            BIND => {
                // A type variable of a type pattern, which match type cases bind.
                let end = self.r.end();
                self.r.pos = end;
                TType::LocalType(start, None)
            }
            other => {
                self.r.pos = start as usize;
                self.skip_tree();
                TType::Unknown(other)
            }
        }
    }
}

const CASEDEF_TAG: u8 = 155;

/// The addresses of the local types a pattern names, among which the case's binders are.
fn collect_binders(t: &TType, out: &mut Vec<Addr>) {
    match t {
        TType::LocalType(addr, prefix) => {
            if !out.contains(addr) {
                out.push(*addr);
            }
            if let Some(p) = prefix {
                collect_binders(p, out);
            }
        }
        TType::TypeRef(p, _) | TType::TermRef(p, _) | TType::This(p) | TType::ByName(p) | TType::Flexible(p) | TType::Alias(p) => collect_binders(p, out),
        TType::Applied(f, args) => {
            collect_binders(f, out);
            args.iter().for_each(|a| collect_binders(a, out));
        }
        TType::Bounds(a, b) | TType::BoundedAlias(a, b) | TType::And(a, b) | TType::Or(a, b) | TType::Annotated(a, b) => {
            collect_binders(a, out);
            collect_binders(b, out);
        }
        TType::Refined(p, members) => {
            collect_binders(p, out);
            members.iter().filter_map(|(_, i)| i.as_ref()).for_each(|i| collect_binders(i, out));
        }
        TType::Match { scrutinee, .. } => collect_binders(scrutinee, out),
        _ => {}
    }
}
