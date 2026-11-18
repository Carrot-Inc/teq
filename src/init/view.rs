//! The initialization checker's trees: a TASTy file's ASTs read as scalac 3.8.4's `TreeUnpickler` reads them
//! (`readTopLevel`, `readIndexedStat`, `readNewDef`, `readTemplate`, `readTree`, `readTpt`, `readCase`), one node a
//! tree of dotty's `tpd` shape, with the address it was read at. What the reference's checker matches is the kind of
//! the tree (`Semantic.cases`, `Objects.cases`), so each node is the kind `TreeUnpickler` makes of its tag: `SELECTin`
//! a `Select`, `QUALTHIS` a `This`, a path in a term position the `Ident` or `Select` that `tpd.ref` makes, a type in
//! a type position one `TypeTree` (`Tpt` here) with the address of its tree, whose span the trace prints
//! (`Semantic.checkTermUsage`). The children are in the order `tpd.TreeTraverser` visits them, which is the order the
//! reference's `InitTreeTraverser` collects classes in.
//!
//! A definition's flags are `readModifiers`' with `normalizeFlags`' (`Method` for a `DEFDEF`, `Deferred` for a member
//! without a body, `ParamAccessor` for a class's `PARAM`, `Param` for a method's, the module flags of an object).

use crate::tasty::positions::{Pos, Section, Spans};
use crate::tasty::tags::*;
use crate::tasty::tree::{Addr, Decoder, TType};
use crate::tasty::{NameRef, Reader, TastyFile};

/// The kind of a node: the class of dotty's tree the reference's unpickler makes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    PackageDef,
    Import,
    Export,
    ValDef,
    DefDef,
    TypeDef,
    Template,
    Ident,
    Select,
    This,
    Super,
    New,
    Literal,
    Throw,
    Apply,
    TypeApply,
    Typed,
    NamedArg,
    Assign,
    Block,
    If,
    InlineIf,
    Closure,
    Match,
    InlineMatch,
    SubMatch,
    CaseDef,
    Return,
    WhileDo,
    Try,
    Inlined,
    SeqLiteral,
    Bind,
    Alternative,
    UnApply,
    Quote,
    Splice,
    QuotePattern,
    SplicePattern,
    Hole,
    /// A type tree: any tree in a type position, one node.
    Tpt,
    /// `ELIDED`: the wildcard `Ident` of the elided tree's type (`ElidedTree`), a body the pickle does not hold.
    Elided,
    /// A tag the reader has no reading for: the tree is skipped and said so.
    Unknown,
}

impl Kind {
    /// The name of the tree's class, as the reference's dump prints `getClass.getSimpleName`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::PackageDef => "PackageDef",
            Kind::Import => "Import",
            Kind::Export => "Export",
            Kind::ValDef => "ValDef",
            Kind::DefDef => "DefDef",
            Kind::TypeDef => "TypeDef",
            Kind::Template => "Template",
            Kind::Ident => "Ident",
            Kind::Select => "Select",
            Kind::This => "This",
            Kind::Super => "Super",
            Kind::New => "New",
            Kind::Literal => "Literal",
            Kind::Throw => "Apply",
            Kind::Apply => "Apply",
            Kind::TypeApply => "TypeApply",
            Kind::Typed => "Typed",
            Kind::NamedArg => "NamedArg",
            Kind::Assign => "Assign",
            Kind::Block => "Block",
            Kind::If => "If",
            Kind::InlineIf => "InlineIf",
            Kind::Closure => "Closure",
            Kind::Match => "Match",
            Kind::InlineMatch => "InlineMatch",
            Kind::SubMatch => "SubMatch",
            Kind::CaseDef => "CaseDef",
            Kind::Return => "Return",
            Kind::WhileDo => "WhileDo",
            Kind::Try => "Try",
            Kind::Inlined => "Inlined",
            Kind::SeqLiteral => "SeqLiteral",
            Kind::Bind => "Bind",
            Kind::Alternative => "Alternative",
            Kind::UnApply => "UnApply",
            Kind::Quote => "Quote",
            Kind::Splice => "Splice",
            Kind::QuotePattern => "QuotePattern",
            Kind::SplicePattern => "SplicePattern",
            Kind::Hole => "Hole",
            Kind::Tpt => "Tpt",
            Kind::Elided => "Ident",
            Kind::Unknown => "Unknown",
        }
    }
}

/// A definition's flags, as the reference's symbol carries them after `readModifiers` and `normalizeFlags`.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct DefFlags(pub u64);

impl DefFlags {
    pub const PACKAGE: u64 = 1 << 0;
    pub const MODULE: u64 = 1 << 1;
    pub const METHOD: u64 = 1 << 2;
    pub const PARAM: u64 = 1 << 3;
    pub const PARAM_ACCESSOR: u64 = 1 << 4;
    pub const ACCESSOR: u64 = 1 << 5;
    pub const CASE_ACCESSOR: u64 = 1 << 6;
    pub const MUTABLE: u64 = 1 << 7;
    pub const LAZY: u64 = 1 << 8;
    pub const DEFERRED: u64 = 1 << 9;
    pub const ABSTRACT: u64 = 1 << 10;
    pub const TRAIT: u64 = 1 << 11;
    pub const FINAL: u64 = 1 << 12;
    pub const SEALED: u64 = 1 << 13;
    pub const OPEN: u64 = 1 << 14;
    pub const CASE: u64 = 1 << 15;
    pub const ENUM: u64 = 1 << 16;
    pub const PRIVATE: u64 = 1 << 17;
    pub const PROTECTED: u64 = 1 << 18;
    pub const LOCAL: u64 = 1 << 19;
    pub const OVERRIDE: u64 = 1 << 20;
    pub const INLINE: u64 = 1 << 21;
    pub const TRANSPARENT: u64 = 1 << 22;
    pub const MACRO: u64 = 1 << 23;
    pub const GIVEN: u64 = 1 << 24;
    pub const IMPLICIT: u64 = 1 << 25;
    pub const ERASED: u64 = 1 << 26;
    pub const SYNTHETIC: u64 = 1 << 27;
    pub const ARTIFACT: u64 = 1 << 28;
    pub const JAVA: u64 = 1 << 29;
    pub const EXPORTED: u64 = 1 << 30;
    pub const STABLE: u64 = 1 << 31;
    pub const ABS_OVERRIDE: u64 = 1 << 32;
    pub const JAVA_STATIC: u64 = 1 << 33;
    pub const INLINE_PROXY: u64 = 1 << 34;
    pub const HAS_DEFAULT: u64 = 1 << 35;
    pub const SUPER_PARAM_ALIAS: u64 = 1 << 36;
    pub const OPAQUE: u64 = 1 << 37;
    pub const EXTENSION: u64 = 1 << 38;

    /// The flags the reference's dump prints, in its order (tests/init-view/InitDump.scala, `flagNames`).
    pub const PRINTED: &'static [(u64, &'static str)] = &[
        (Self::PACKAGE, "package"),
        (Self::MODULE, "module"),
        (Self::METHOD, "method"),
        (Self::PARAM, "param"),
        (Self::PARAM_ACCESSOR, "paramAccessor"),
        (Self::ACCESSOR, "accessor"),
        (Self::CASE_ACCESSOR, "caseAccessor"),
        (Self::MUTABLE, "mutable"),
        (Self::LAZY, "lazy"),
        (Self::DEFERRED, "deferred"),
        (Self::ABSTRACT, "abstract"),
        (Self::TRAIT, "trait"),
        (Self::FINAL, "final"),
        (Self::SEALED, "sealed"),
        (Self::OPEN, "open"),
        (Self::CASE, "case"),
        (Self::ENUM, "enum"),
        (Self::PRIVATE, "private"),
        (Self::PROTECTED, "protected"),
        (Self::LOCAL, "local"),
        (Self::OVERRIDE, "override"),
        (Self::INLINE, "inline"),
        (Self::TRANSPARENT, "transparent"),
        (Self::MACRO, "macro"),
        (Self::GIVEN, "given"),
        (Self::IMPLICIT, "implicit"),
        (Self::ERASED, "erased"),
        (Self::SYNTHETIC, "synthetic"),
        (Self::ARTIFACT, "artifact"),
        (Self::JAVA, "java"),
        (Self::EXPORTED, "exported"),
        (Self::STABLE, "stable"),
    ];

    pub fn has(self, f: u64) -> bool {
        self.0 & f != 0
    }
}

/// One tree.
#[derive(Clone, Debug)]
pub struct Node {
    /// The address the tree was read at: for a shared reference, the occurrence's.
    pub addr: Addr,
    /// The address the tree's bytes are at: a shared reference's target, else `addr`.
    pub at: Addr,
    pub kind: Kind,
    /// The name a definition, an `Ident`, a `Select`, a `NamedArg` or a `Bind` carries.
    pub name: Option<NameRef>,
    /// The type the node was read with: an `Ident`'s, a path's, a `Tpt`'s, `New`'s, a constant's, a `This`'s,
    /// `ELIDED`'s, an `UnApply`'s pattern type, a `SELECTin`'s owner.
    pub ty: Option<TType>,
    /// A definition's or a `Bind`'s flags.
    pub flags: DefFlags,
    /// `Return`'s method (the address of its `DEFDEF`).
    pub target: Option<Addr>,
    /// `Inlined`'s call, which `tpd.TreeTraverser` does not visit.
    pub call: Option<Box<Node>>,
    pub children: Vec<Node>,
}

impl Node {
    fn new(addr: Addr, at: Addr, _tag: u8, kind: Kind) -> Node {
        Node { addr, at, kind, name: None, ty: None, flags: DefFlags::default(), target: None, call: None, children: Vec::new() }
    }

    pub fn is_class_def(&self) -> bool {
        self.kind == Kind::TypeDef && self.children.first().is_some_and(|c| c.kind == Kind::Template)
    }
}

/// Where a definition stands, which `normalizeFlags` reads: a class's template, or a method or a block.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Class,
    Other,
}

/// The reader of one file.
pub struct ViewReader<'f> {
    pub file: &'f TastyFile,
    pub trees: &'f [u8],
    pub section: Option<Section>,
    pub spans: Spans,
    dec: Decoder<'f>,
}

/// The tags of a type tree in a type position (`TastyFormat.isTypeTreeTag`).
fn is_type_tree_tag(tag: u8) -> bool {
    matches!(
        tag,
        IDENTTPT | SELECTTPT | SINGLETONTPT | REFINEDTPT | APPLIEDTPT | LAMBDATPT | TYPEBOUNDSTPT | ANNOTATEDTPT | BYNAMETPT | MATCHTPT
    )
}

impl<'f> ViewReader<'f> {
    pub fn new(file: &'f TastyFile) -> ViewReader<'f> {
        let section = Section::read(file);
        let spans = match &section {
            Some(s) => Spans::of(file, s),
            None => Spans::default(),
        };
        ViewReader { file, trees: &file.bytes[file.asts.clone()], section, spans, dec: Decoder::new(file) }
    }

    /// The span `TreeUnpickler` gives the tree at `addr`.
    pub fn pos(&self, addr: Addr) -> Option<Pos> {
        self.spans.get(addr).and_then(|p| p.pos)
    }

    /// The path of the source of the tree at `addr`.
    pub fn source(&self, addr: Addr) -> Option<NameRef> {
        self.spans.get(addr).and_then(|p| p.source).or_else(|| self.section.as_ref().and_then(|s| s.primary()))
    }

    fn type_at(&mut self, addr: usize) -> TType {
        self.dec.type_at(addr as Addr)
    }

    fn skip(&mut self, r: &mut Reader) {
        r.pos = self.dec.skip_at(r.pos as Addr) as usize;
    }

    /// The file's top-level trees: its package clauses (`readTopLevel`).
    pub fn top_level(&mut self) -> Vec<Node> {
        let trees = self.trees;
        let mut r = Reader::new(trees);
        let mut out = Vec::new();
        while !r.at_end() && matches!(r.peek(), PACKAGE | IMPORT) {
            let before = r.pos;
            out.push(self.stat(&mut r, Owner::Other));
            if r.pos <= before {
                break;
            }
        }
        out
    }

    /// `readIndexedStat`.
    fn stat(&mut self, r: &mut Reader, owner: Owner) -> Node {
        match r.peek() {
            TYPEDEF | VALDEF | DEFDEF => self.def(r, owner),
            IMPORT | EXPORT => {
                let addr = r.pos as Addr;
                let tag = r.byte();
                let end = r.end();
                r.pos = end;
                Node::new(addr, addr, tag, if tag == IMPORT { Kind::Import } else { Kind::Export })
            }
            PACKAGE => {
                let addr = r.pos as Addr;
                r.byte();
                let end = r.end();
                let mut n = Node::new(addr, addr, PACKAGE, Kind::PackageDef);
                // The clause's package: `processPackage`'s `RefTree`, an `Ident` of the package's term.
                let mut pid = self.term(r);
                pid.kind = Kind::Ident;
                n.children.push(pid);
                while r.pos < end {
                    let before = r.pos;
                    n.children.push(self.stat(r, Owner::Other));
                    if r.pos <= before {
                        break;
                    }
                }
                r.pos = end;
                n
            }
            _ => self.term(r),
        }
    }

    /// `readModifiers` from the reader's position to `end`, with `normalizeFlags`.
    fn modifiers(&mut self, r: &mut Reader, end: usize) -> u64 {
        let mut f = 0u64;
        while r.pos < end {
            let tag = r.peek();
            let bit = match tag {
                PRIVATE => DefFlags::PRIVATE,
                PROTECTED => DefFlags::PROTECTED,
                ABSTRACT => {
                    r.byte();
                    if r.peek() == OVERRIDE {
                        r.byte();
                        f |= DefFlags::ABS_OVERRIDE;
                    } else {
                        f |= DefFlags::ABSTRACT;
                    }
                    continue;
                }
                FINAL => DefFlags::FINAL,
                SEALED => DefFlags::SEALED,
                CASE => DefFlags::CASE,
                IMPLICIT => DefFlags::IMPLICIT,
                ERASED => DefFlags::ERASED,
                LAZY => DefFlags::LAZY,
                OVERRIDE => DefFlags::OVERRIDE,
                INLINE => DefFlags::INLINE,
                INLINEPROXY => DefFlags::INLINE_PROXY,
                MACRO => DefFlags::MACRO,
                OPAQUE => DefFlags::OPAQUE,
                STATIC => DefFlags::JAVA_STATIC,
                OBJECT => DefFlags::MODULE,
                TRAIT => DefFlags::TRAIT,
                ENUM => DefFlags::ENUM,
                LOCAL => DefFlags::LOCAL,
                SYNTHETIC => DefFlags::SYNTHETIC,
                ARTIFACT => DefFlags::ARTIFACT,
                MUTABLE => DefFlags::MUTABLE,
                FIELDACCESSOR => DefFlags::ACCESSOR,
                CASEACCESSOR => DefFlags::CASE_ACCESSOR,
                HASDEFAULT => DefFlags::HAS_DEFAULT,
                STABLE => DefFlags::STABLE,
                EXTENSION => DefFlags::EXTENSION,
                GIVEN => DefFlags::GIVEN,
                PARAMSETTER => DefFlags::PARAM_ACCESSOR,
                PARAMALIAS => DefFlags::SUPER_PARAM_ALIAS,
                EXPORTED => DefFlags::EXPORTED,
                OPEN => DefFlags::OPEN,
                TRANSPARENT => DefFlags::TRANSPARENT,
                COVARIANT | CONTRAVARIANT | INVISIBLE | INFIX | TRACKED | INTO => 0,
                PRIVATEQUALIFIED | PROTECTEDQUALIFIED => {
                    r.byte();
                    if tag == PROTECTEDQUALIFIED {
                        f |= DefFlags::PROTECTED;
                    }
                    self.skip(r);
                    continue;
                }
                ANNOTATION => {
                    r.byte();
                    r.pos = r.end();
                    continue;
                }
                _ => {
                    // Not a modifier: what follows is no definition's (a malformed file); stop here.
                    r.pos = end;
                    break;
                }
            };
            r.byte();
            f |= bit;
        }
        f
    }

    /// Whether only modifiers stand between the reader and `end` (`nothingButMods`).
    fn nothing_but_mods(r: &Reader, end: usize) -> bool {
        r.pos >= end || is_modifier(r.peek())
    }

    /// `readNewDef`: a `VALDEF`, `DEFDEF`, `TYPEDEF`, `PARAM` or `TYPEPARAM`.
    fn def(&mut self, r: &mut Reader, owner: Owner) -> Node {
        let addr = r.pos as Addr;
        let tag = r.byte();
        let end = r.end();
        let name = r.nat();
        let kind = match tag {
            VALDEF | PARAM => Kind::ValDef,
            DEFDEF => Kind::DefDef,
            _ => Kind::TypeDef,
        };
        let mut n = Node::new(addr, addr, tag, kind);
        n.name = Some(name);
        let mut is_class = false;
        let mut abstract_type = false;
        match tag {
            DEFDEF => {
                loop {
                    match r.peek() {
                        PARAM | TYPEPARAM => {
                            let ptag = r.peek();
                            while r.peek() == ptag && r.pos < end {
                                n.children.push(self.def(r, Owner::Other));
                            }
                        }
                        EMPTYCLAUSE | SPLITCLAUSE => {
                            r.byte();
                        }
                        _ => break,
                    }
                }
                n.children.push(self.tpt(r));
            }
            VALDEF | PARAM => {
                n.children.push(self.tpt(r));
            }
            _ => {
                if r.peek() == TEMPLATE {
                    is_class = true;
                    n.children.push(self.template(r));
                } else {
                    abstract_type = self.is_abstract_type(r.pos);
                    n.children.push(self.tpt(r));
                }
            }
        }
        // The body, where there is one before the modifiers.
        let rhs_empty = Self::nothing_but_mods(r, end);
        if !rhs_empty && matches!(tag, VALDEF | DEFDEF | PARAM) {
            let rhs = self.term(r);
            n.children.push(rhs);
        }
        let mut flags = self.modifiers(r, end);
        r.pos = end;
        // `normalizeFlags`.
        let term_name = !matches!(tag, TYPEDEF | TYPEPARAM);
        let ctor = term_name && self.file.simple(name) == Some("<init>");
        let lacks = (rhs_empty && term_name && !ctor && flags & (DefFlags::PARAM | DefFlags::PARAM_ACCESSOR) == 0 && tag != PARAM)
            || abstract_type;
        if lacks && tag != PARAM {
            flags |= DefFlags::DEFERRED;
        }
        if tag == DEFDEF {
            flags |= DefFlags::METHOD;
        }
        if flags & DefFlags::MODULE != 0 {
            // ModuleValCreationFlags: Module | Lazy | Final | StableRealizable; ModuleClassCreationFlags: Module | Final.
            flags |= if tag == VALDEF { DefFlags::LAZY | DefFlags::FINAL | DefFlags::STABLE } else { DefFlags::FINAL };
        }
        if flags & DefFlags::ENUM != 0 && flags & DefFlags::METHOD == 0 && term_name {
            flags |= DefFlags::STABLE;
        }
        if owner == Owner::Class {
            if tag == TYPEPARAM {
                flags |= DefFlags::PARAM;
            } else if tag == PARAM {
                flags |= DefFlags::PARAM_ACCESSOR;
                if !rhs_empty {
                    flags |= DefFlags::METHOD;
                }
            }
        } else if matches!(tag, PARAM | TYPEPARAM) {
            flags |= DefFlags::PARAM;
        }
        if is_class && flags & DefFlags::TRAIT != 0 {
            flags |= DefFlags::ABSTRACT;
        }
        n.flags = DefFlags(flags);
        n
    }

    /// `isAbstractType` of a type definition's right side at `pos`.
    fn is_abstract_type(&self, pos: usize) -> bool {
        let mut r = Reader::new(self.trees).at(pos);
        match r.peek() {
            SHAREDTYPE => {
                r.byte();
                let target = r.nat() as usize;
                self.is_abstract_type(target)
            }
            LAMBDATPT => {
                r.byte();
                r.end();
                while r.peek() == TYPEPARAM {
                    r.byte();
                    r.pos = r.end();
                }
                self.is_abstract_type(r.pos)
            }
            TYPEBOUNDS => {
                r.byte();
                let end = r.end();
                // The alias or the lower bound, then whether anything but modifiers follows.
                let mut d = Decoder::new(self.file);
                r.pos = d.skip_at(r.pos as Addr) as usize;
                !Self::nothing_but_mods(&r, end)
            }
            TYPEBOUNDSTPT => true,
            _ => false,
        }
    }

    /// `readTemplate`: the constructor, the parents, the self definition and the body (the type parameters, the
    /// parameter accessors, then the statements), in `tpd.TreeTraverser`'s order.
    fn template(&mut self, r: &mut Reader) -> Node {
        let addr = r.pos as Addr;
        r.byte();
        let end = r.end();
        let mut n = Node::new(addr, addr, TEMPLATE, Kind::Template);
        let mut params = Vec::new();
        while r.peek() == TYPEPARAM {
            params.push(self.def(r, Owner::Class));
        }
        while r.peek() == PARAM {
            params.push(self.def(r, Owner::Class));
        }
        let mut parents = Vec::new();
        while r.pos < end && !matches!(r.peek(), SELFDEF | DEFDEF | SPLITCLAUSE) {
            let before = r.pos;
            // `readParents(withArgs = false)`: a parent's tree as written, a constructor call or a type tree.
            let tag = r.peek();
            let parent = if is_type_tree_tag(tag) || matches!(tag, TYPEREF | TYPEREFDIRECT | TYPEREFSYMBOL | TYPEREFPKG | TYPEREFIN | SHAREDTYPE | APPLIEDTYPE) {
                self.tpt(r)
            } else {
                self.term(r)
            };
            parents.push(parent);
            if r.pos <= before {
                break;
            }
        }
        let mut self_def = None;
        if r.peek() == SELFDEF {
            let saddr = r.pos as Addr;
            r.byte();
            let name = r.nat();
            let mut s = Node::new(saddr, saddr, SELFDEF, Kind::ValDef);
            s.name = Some(name);
            s.children.push(self.tpt(r));
            self_def = Some(s);
        }
        let ctor = if r.peek() == SPLITCLAUSE {
            r.byte();
            None
        } else if r.peek() == DEFDEF {
            Some(self.def(r, Owner::Class))
        } else {
            None
        };
        let mut stats = Vec::new();
        while r.pos < end {
            let before = r.pos;
            stats.push(self.stat(r, Owner::Class));
            if r.pos <= before {
                break;
            }
        }
        r.pos = end;
        if let Some(c) = ctor {
            n.children.push(c);
        }
        n.children.extend(parents);
        if let Some(s) = self_def {
            n.children.push(s);
        }
        n.children.extend(params);
        n.children.extend(stats);
        n
    }

    /// `readTpt`: one `Tpt` node for a type tree or a type, a `Block` of type aliases around one.
    fn tpt(&mut self, r: &mut Reader) -> Node {
        let addr = r.pos as Addr;
        match r.peek() {
            SHAREDTERM => {
                r.byte();
                let target = r.nat() as usize;
                let mut inner = Reader::new(self.trees).at(target);
                let mut n = self.tpt(&mut inner);
                n.addr = addr;
                n
            }
            BLOCK => {
                r.byte();
                let end = r.end();
                let tpt_at = r.pos;
                self.skip(r);
                let mut n = Node::new(addr, addr, BLOCK, Kind::Block);
                while r.pos < end {
                    let before = r.pos;
                    n.children.push(self.stat(r, Owner::Other));
                    if r.pos <= before {
                        break;
                    }
                }
                r.pos = end;
                let mut tr = Reader::new(self.trees).at(tpt_at);
                n.children.push(self.tpt(&mut tr));
                n
            }
            tag => {
                let ty = self.type_at(r.pos);
                self.skip(r);
                let mut n = Node::new(addr, addr, tag, Kind::Tpt);
                n.ty = Some(ty);
                n
            }
        }
    }

    /// `readTree`.
    pub fn term(&mut self, r: &mut Reader) -> Node {
        let addr = r.pos as Addr;
        let tag = r.peek();
        if tag == SHAREDTERM {
            r.byte();
            let target = r.nat() as usize;
            let mut inner = Reader::new(self.trees).at(target);
            let mut n = self.term(&mut inner);
            n.addr = addr;
            return n;
        }
        if tag < 128 {
            return self.simple_term(r);
        }
        r.byte();
        let end = r.end();
        let mut n = Node::new(addr, addr, tag, Kind::Unknown);
        match tag {
            SUPER => {
                n.kind = Kind::Super;
                n.children.push(self.term(r));
                if r.pos < end {
                    n.ty = Some(self.type_at(r.pos));
                    self.skip(r);
                }
            }
            APPLY => {
                n.kind = Kind::Apply;
                n.children.push(self.term(r));
                self.until(r, end, &mut n, false);
            }
            APPLYSIGPOLY => {
                n.kind = Kind::Apply;
                n.children.push(self.term(r));
                self.skip(r);
                self.until(r, end, &mut n, false);
            }
            TYPEAPPLY => {
                n.kind = Kind::TypeApply;
                n.children.push(self.term(r));
                self.until(r, end, &mut n, true);
            }
            TYPED => {
                n.kind = Kind::Typed;
                n.children.push(self.term(r));
                n.children.push(self.tpt(r));
            }
            ASSIGN => {
                n.kind = Kind::Assign;
                n.children.push(self.term(r));
                n.children.push(self.term(r));
            }
            BLOCK => {
                n.kind = Kind::Block;
                let expr_at = r.pos;
                self.skip(r);
                while r.pos < end {
                    let before = r.pos;
                    n.children.push(self.stat(r, Owner::Other));
                    if r.pos <= before {
                        break;
                    }
                }
                let mut er = Reader::new(self.trees).at(expr_at);
                n.children.push(self.term(&mut er));
            }
            INLINED => {
                n.kind = Kind::Inlined;
                let expr_at = r.pos;
                self.skip(r);
                if r.pos < end {
                    let next = self.unshared_tag(r.pos);
                    if !matches!(next, VALDEF | DEFDEF) {
                        n.call = Some(Box::new(self.term(r)));
                    }
                }
                while r.pos < end {
                    let before = r.pos;
                    n.children.push(self.stat(r, Owner::Other));
                    if r.pos <= before {
                        break;
                    }
                }
                let mut er = Reader::new(self.trees).at(expr_at);
                n.children.push(self.term(&mut er));
            }
            IF => {
                n.kind = Kind::If;
                if r.peek() == INLINE {
                    r.byte();
                    n.kind = Kind::InlineIf;
                }
                // `IF Length [INLINE] cond_Term then_Term else_Term` (TastyFormat): the three terms.
                let cond = self.term(r);
                let thenp = self.term(r);
                let elsep = self.term(r);
                n.children.extend([cond, thenp, elsep]);
            }
            LAMBDA => {
                n.kind = Kind::Closure;
                n.children.push(self.term(r));
                if r.pos < end {
                    n.children.push(self.tpt(r));
                }
            }
            MATCH => {
                n.kind = Kind::Match;
                match r.peek() {
                    IMPLICIT => {
                        r.byte();
                        n.kind = Kind::InlineMatch;
                    }
                    INLINE => {
                        r.byte();
                        n.kind = Kind::InlineMatch;
                        n.children.push(self.term(r));
                    }
                    SUBMATCH => {
                        r.byte();
                        n.kind = Kind::SubMatch;
                        n.children.push(self.term(r));
                    }
                    _ => n.children.push(self.term(r)),
                }
                self.cases(r, end, &mut n);
            }
            RETURN => {
                n.kind = Kind::Return;
                n.target = Some(r.nat());
                if r.pos < end {
                    n.children.push(self.term(r));
                }
            }
            WHILE => {
                n.kind = Kind::WhileDo;
                n.children.push(self.term(r));
                n.children.push(self.term(r));
            }
            TRY => {
                n.kind = Kind::Try;
                n.children.push(self.term(r));
                self.cases(r, end, &mut n);
                if r.pos < end {
                    n.children.push(self.term(r));
                }
            }
            SELECTOUTER => {
                // `outerSelect`: a `Select` of the outer accessor (`OuterSelectName`).
                n.kind = Kind::Select;
                r.nat();
                n.children.push(self.term(r));
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
            }
            SELECTIN => {
                n.kind = Kind::Select;
                n.name = Some(r.nat());
                n.children.push(self.term(r));
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
            }
            REPEATED => {
                n.kind = Kind::SeqLiteral;
                let elem = self.tpt(r);
                self.until(r, end, &mut n, false);
                n.children.push(elem);
            }
            BIND => {
                n.kind = Kind::Bind;
                n.name = Some(r.nat());
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
                n.children.push(self.term(r));
                n.flags = DefFlags(self.modifiers(r, end));
            }
            ALTERNATIVE => {
                n.kind = Kind::Alternative;
                self.until(r, end, &mut n, false);
            }
            UNAPPLY => {
                n.kind = Kind::UnApply;
                n.children.push(self.term(r));
                while r.peek() == IMPLICITARG && r.pos < end {
                    r.byte();
                    n.children.push(self.term(r));
                }
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
                self.until(r, end, &mut n, false);
            }
            QUOTE => {
                n.kind = Kind::Quote;
                n.children.push(self.term(r));
            }
            SPLICE => {
                n.kind = Kind::Splice;
                n.children.push(self.term(r));
            }
            QUOTEPATTERN => n.kind = Kind::QuotePattern,
            SPLICEPATTERN => n.kind = Kind::SplicePattern,
            HOLE => n.kind = Kind::Hole,
            REFINEDTPT | APPLIEDTPT | ANNOTATEDTPT | LAMBDATPT | MATCHTPT | TYPEBOUNDSTPT => {
                r.pos = addr as usize;
                return self.tpt(r);
            }
            _ => {
                // A path or a type in a term position: `readPathTree`.
                r.pos = addr as usize;
                return self.path_tree(r);
            }
        }
        r.pos = end;
        n
    }

    fn unshared_tag(&self, pos: usize) -> u8 {
        let mut r = Reader::new(self.trees).at(pos);
        if r.peek() == SHAREDTERM {
            r.byte();
            let t = r.nat() as usize;
            return Reader::new(self.trees).at(t).peek();
        }
        r.peek()
    }

    fn until(&mut self, r: &mut Reader, end: usize, n: &mut Node, types: bool) {
        while r.pos < end {
            let before = r.pos;
            n.children.push(if types { self.tpt(r) } else { self.term(r) });
            if r.pos <= before {
                break;
            }
        }
    }

    /// `readCases`.
    fn cases(&mut self, r: &mut Reader, end: usize, n: &mut Node) {
        while r.pos < end && self.unshared_tag(r.pos) == CASEDEF {
            let occurrence = r.pos as Addr;
            let mut cr = if r.peek() == SHAREDTERM {
                r.byte();
                let t = r.nat() as usize;
                Reader::new(self.trees).at(t)
            } else {
                r.clone()
            };
            let caddr = cr.pos as Addr;
            cr.byte();
            let cend = cr.end();
            let mut c = Node::new(occurrence, caddr, CASEDEF, Kind::CaseDef);
            let pat = self.term(&mut cr);
            let body = self.term(&mut cr);
            c.children.push(pat);
            if cr.pos < cend {
                c.children.push(self.term(&mut cr));
            }
            c.children.push(body);
            if occurrence == caddr {
                r.pos = cend;
            }
            n.children.push(c);
        }
    }

    /// `readSimpleTree`, for a tag below the length trees.
    fn simple_term(&mut self, r: &mut Reader) -> Node {
        let addr = r.pos as Addr;
        let tag = r.peek();
        match tag {
            IDENT => {
                r.byte();
                let name = r.nat();
                let mut n = Node::new(addr, addr, tag, Kind::Ident);
                n.name = Some(name);
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
                n
            }
            ELIDED => {
                r.byte();
                let mut n = Node::new(addr, addr, tag, Kind::Elided);
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
                n
            }
            IDENTTPT | SINGLETONTPT | BYNAMETPT | SELECTTPT => self.tpt(r),
            EXPLICITTPT => {
                r.byte();
                self.tpt(r)
            }
            SELECT => {
                r.byte();
                let name = r.nat();
                let mut n = Node::new(addr, addr, tag, Kind::Select);
                n.name = Some(name);
                n.children.push(self.term(r));
                n
            }
            QUALTHIS => {
                let mut n = Node::new(addr, addr, tag, Kind::This);
                n.ty = Some(self.type_at(r.pos));
                self.skip(r);
                n
            }
            NEW => {
                r.byte();
                let mut n = Node::new(addr, addr, tag, Kind::New);
                n.children.push(self.tpt(r));
                n.ty = n.children[0].ty.clone();
                n
            }
            THROW => {
                r.byte();
                let mut n = Node::new(addr, addr, tag, Kind::Throw);
                n.children.push(self.term(r));
                n
            }
            NAMEDARG => {
                r.byte();
                let name = r.nat();
                let mut n = Node::new(addr, addr, tag, Kind::NamedArg);
                n.name = Some(name);
                n.children.push(self.term(r));
                n
            }
            _ => self.path_tree(r),
        }
    }

    /// `readPathTree`: the type at the reader, made the tree `TreeUnpickler` makes of it: a type a `Tpt`, a constant
    /// a `Literal`, `C.this` a `This`, a term reference what `tpd.ref` makes of it.
    fn path_tree(&mut self, r: &mut Reader) -> Node {
        let addr = r.pos as Addr;
        let tag = r.peek();
        let ty = self.type_at(r.pos);
        self.skip(r);
        let kind = match &ty {
            TType::Const(_) => Kind::Literal,
            TType::This(_) => Kind::This,
            TType::TermRef(..) | TType::LocalTerm(..) => Kind::Ident,
            _ => Kind::Tpt,
        };
        let mut n = Node::new(addr, addr, tag, kind);
        if kind == Kind::Ident {
            // `tpd.ref`: a prefix that is not elidable is a qualifier, `C.this.x` a `Select` of a `This`.
            if let TType::TermRef(prefix, _) | TType::LocalTerm(_, Some(prefix)) = &ty {
                let name = match &ty {
                    TType::TermRef(_, nm) => Some(*nm),
                    _ => None,
                };
                if let TType::This(_) = prefix.as_ref() {
                    n.kind = Kind::Select;
                    n.name = name;
                    let mut q = Node::new(addr, addr, tag, Kind::This);
                    q.ty = Some((**prefix).clone());
                    n.children.push(q);
                } else if let TType::TermRef(..) = prefix.as_ref() {
                    n.kind = Kind::Select;
                    n.name = name;
                    let mut q = Node::new(addr, addr, tag, Kind::Ident);
                    q.ty = Some((**prefix).clone());
                    n.children.push(q);
                } else {
                    n.name = name;
                }
            }
        }
        n.ty = Some(ty);
        n
    }
}

/// Every class definition of a list of trees, children first, as `Checker.InitTreeTraverser` collects them.
pub fn classes_post_order<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>) {
    for n in nodes {
        if let Some(call) = &n.call {
            let _ = call;
        }
        classes_post_order(&n.children, out);
        if n.is_class_def() {
            out.push(n);
        }
    }
}
