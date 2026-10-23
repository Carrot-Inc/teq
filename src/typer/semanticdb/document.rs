//! SemanticDB's text document as dotty 3.8.4's generated classes hold it (compiler/src/dotty/tools/dotc/semanticdb/
//! generated/*.scala), the parts `ExtractSemanticDB`, `TypeOps` and `SyntheticsExtractor` fill, and its protobuf form
//! as their `writeTo` writes it: the fields in number order, a scalar or a string only when it is not its default
//! (`-0.0` is `0.0`), a singular sealed oneof (a type, a signature, a constant, an access, a tree) only when its
//! message is not empty, an element of a repeated one always, an optional message (a range, a scope) whenever it is
//! present. A document holds what ExtractSemanticDB.scala 168-178 fills: the schema `SEMANTICDB4`, the language
//! `SCALA`, no text (`-semanticdb-text` is not teq's) and no diagnostics (the later `AppendDiagnostics` phase's,
//! 64-78); a symbol's information has no documentation (dotty fills none) and the language `SCALA`
//! (Scala3.scala 108-157).
// The schema whole, which the port fills (tests/semanticdb/TABLE.md, the order of work's step 7); until then the
// dump writes a document's head alone.
#![allow(dead_code)]

/// `TextDocument` (TextDocument.scala).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TextDocument {
    pub uri: String,
    pub md5: String,
    pub symbols: Vec<SymbolInformation>,
    pub occurrences: Vec<SymbolOccurrence>,
    pub synthetics: Vec<Synthetic>,
}

/// `SymbolInformation.Kind`'s values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    Unknown = 0,
    Local = 19,
    Field = 20,
    Method = 3,
    Constructor = 21,
    Macro = 6,
    Type = 7,
    Parameter = 8,
    SelfParameter = 17,
    TypeParameter = 9,
    Object = 10,
    Package = 11,
    PackageObject = 12,
    Class = 13,
    Trait = 14,
    Interface = 18,
}

/// `SymbolInformation.Property`'s bits.
pub mod property {
    pub const ABSTRACT: i32 = 0x4;
    pub const FINAL: i32 = 0x8;
    pub const SEALED: i32 = 0x10;
    pub const IMPLICIT: i32 = 0x20;
    pub const LAZY: i32 = 0x40;
    pub const CASE: i32 = 0x80;
    pub const COVARIANT: i32 = 0x100;
    pub const CONTRAVARIANT: i32 = 0x200;
    pub const VAL: i32 = 0x400;
    pub const VAR: i32 = 0x800;
    pub const STATIC: i32 = 0x1000;
    pub const PRIMARY: i32 = 0x2000;
    pub const ENUM: i32 = 0x4000;
    pub const DEFAULT: i32 = 0x8000;
    pub const GIVEN: i32 = 0x10000;
    pub const INLINE: i32 = 0x20000;
    pub const OPEN: i32 = 0x40000;
    pub const TRANSPARENT: i32 = 0x80000;
    pub const INFIX: i32 = 0x100000;
    pub const OPAQUE: i32 = 0x200000;
}

/// `SymbolInformation`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SymbolInformation {
    pub symbol: String,
    pub kind: Kind,
    pub properties: i32,
    pub display_name: String,
    pub annotations: Vec<Annotation>,
    pub signature: Signature,
    pub access: Access,
    pub overridden_symbols: Vec<String>,
}

/// `Annotation`.
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub tpe: Type,
}

/// `Scope`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Scope {
    pub symlinks: Vec<String>,
    pub hardlinks: Vec<SymbolInformation>,
}

/// `Signature`, a sealed oneof.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Signature {
    #[default]
    Empty,
    Class { type_parameters: Option<Scope>, parents: Vec<Type>, self_type: Type, declarations: Option<Scope> },
    Method { type_parameters: Option<Scope>, parameter_lists: Vec<Scope>, return_type: Type },
    Type { type_parameters: Option<Scope>, lower_bound: Type, upper_bound: Type },
    Value { tpe: Type },
}

/// `Type`, a sealed oneof: the cases TypeOps makes.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Type {
    #[default]
    Empty,
    TypeRef { prefix: Box<Type>, symbol: String, type_arguments: Vec<Type> },
    SingleType { prefix: Box<Type>, symbol: String },
    ThisType { symbol: String },
    SuperType { prefix: Box<Type>, symbol: String },
    ConstantType { constant: Constant },
    IntersectionType { types: Vec<Type> },
    UnionType { types: Vec<Type> },
    StructuralType { tpe: Box<Type>, declarations: Option<Scope> },
    AnnotatedType { annotations: Vec<Annotation>, tpe: Box<Type> },
    ExistentialType { tpe: Box<Type>, declarations: Option<Scope> },
    ByNameType { tpe: Box<Type> },
    RepeatedType { tpe: Box<Type> },
    MatchType { scrutinee: Box<Type>, cases: Vec<CaseType> },
    LambdaType { parameters: Option<Scope>, return_type: Box<Type> },
}

/// `MatchType.CaseType`.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseType {
    pub key: Type,
    pub body: Type,
}

/// `Constant`, a sealed oneof (ConstantOps.scala).
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Constant {
    #[default]
    Empty,
    Unit,
    Boolean(bool),
    Byte(i32),
    Short(i32),
    Char(i32),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    Null,
}

/// `Access`, a sealed oneof.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Access {
    #[default]
    Empty,
    Private,
    PrivateThis,
    PrivateWithin(String),
    Protected,
    ProtectedThis,
    ProtectedWithin(String),
    Public,
}

/// `Range`: lines and characters from 0, the end exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Range {
    pub start_line: i32,
    pub start_character: i32,
    pub end_line: i32,
    pub end_character: i32,
}

/// `SymbolOccurrence.Role`'s values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Role {
    #[default]
    Unknown = 0,
    Reference = 1,
    Definition = 2,
}

/// `SymbolOccurrence`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SymbolOccurrence {
    pub range: Option<Range>,
    pub symbol: String,
    pub role: Role,
}

/// `Synthetic`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Synthetic {
    pub range: Option<Range>,
    pub tree: Tree,
}

/// `Tree`, a sealed oneof: the cases SyntheticsExtractor makes.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Tree {
    #[default]
    Empty,
    /// `properties`, a bitmask of `SymbolInformation.Property`: SyntheticsExtractor.scala 65-78 sets `GIVEN` on the
    /// application of a `using` clause's arguments.
    Apply { function: Box<Tree>, arguments: Vec<Tree>, properties: i32 },
    Id { symbol: String },
    Original { range: Option<Range> },
    Select { qualifier: Box<Tree>, id: Option<String> },
    TypeApply { function: Box<Tree>, type_arguments: Vec<Type> },
}

/// `TextDocuments(List(doc))` as ExtractSemanticDB.scala 179-183 writes it.
pub fn write_documents(doc: &TextDocument) -> Vec<u8> {
    let mut o = Out::default();
    o.message(1, &document(doc));
    o.0
}

#[derive(Default)]
struct Out(Vec<u8>);

impl Out {
    fn varint(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.0.push(v as u8 | 0x80);
            v >>= 7;
        }
        self.0.push(v as u8);
    }

    fn tag(&mut self, field: u32, wire: u32) {
        self.varint((field << 3 | wire) as u64);
    }

    /// An `int32` or an enum, sign-extended as `writeInt32`/`writeEnum` write a negative one.
    fn int32(&mut self, field: u32, v: i32) {
        if v != 0 {
            self.tag(field, 0);
            self.varint(v as i64 as u64);
        }
    }

    fn int64(&mut self, field: u32, v: i64) {
        if v != 0 {
            self.tag(field, 0);
            self.varint(v as u64);
        }
    }

    fn boolean(&mut self, field: u32, v: bool) {
        if v {
            self.tag(field, 0);
            self.varint(1);
        }
    }

    /// `writeFloatNoTag`'s `floatToIntBits`: a NaN is the canonical one whatever its sign and payload.
    fn float(&mut self, field: u32, v: f32) {
        if v != 0.0 {
            self.tag(field, 5);
            let bits = if v.is_nan() { 0x7fc0_0000 } else { v.to_bits() };
            self.0.extend_from_slice(&bits.to_le_bytes());
        }
    }

    /// `writeDoubleNoTag`'s `doubleToLongBits`: a NaN is the canonical one whatever its sign and payload.
    fn double(&mut self, field: u32, v: f64) {
        if v != 0.0 {
            self.tag(field, 1);
            let bits = if v.is_nan() { 0x7ff8_0000_0000_0000 } else { v.to_bits() };
            self.0.extend_from_slice(&bits.to_le_bytes());
        }
    }

    fn string(&mut self, field: u32, s: &str) {
        if !s.is_empty() {
            self.bytes(field, s.as_bytes());
        }
    }

    /// A repeated string's element, an optional message or a repeated message's element: always written.
    fn bytes(&mut self, field: u32, b: &[u8]) {
        self.tag(field, 2);
        self.varint(b.len() as u64);
        self.0.extend_from_slice(b);
    }

    fn message(&mut self, field: u32, b: &[u8]) {
        self.bytes(field, b);
    }

    /// A singular sealed oneof: written only when its message is not empty.
    fn oneof(&mut self, field: u32, b: &[u8]) {
        if !b.is_empty() {
            self.bytes(field, b);
        }
    }
}

fn build(f: impl FnOnce(&mut Out)) -> Vec<u8> {
    let mut o = Out::default();
    f(&mut o);
    o.0
}

fn document(d: &TextDocument) -> Vec<u8> {
    build(|o| {
        o.int32(1, 4); // Schema.SEMANTICDB4
        o.string(2, &d.uri);
        for s in &d.symbols {
            o.message(5, &information(s));
        }
        for x in &d.occurrences {
            o.message(6, &occurrence(x));
        }
        o.int32(10, 1); // Language.SCALA
        o.string(11, &d.md5);
        for s in &d.synthetics {
            o.message(12, &synthetic(s));
        }
    })
}

fn information(s: &SymbolInformation) -> Vec<u8> {
    build(|o| {
        o.string(1, &s.symbol);
        o.int32(3, s.kind as i32);
        o.int32(4, s.properties);
        o.string(5, &s.display_name);
        for a in &s.annotations {
            o.message(13, &annotation(a));
        }
        o.int32(16, 1); // Language.SCALA
        o.oneof(17, &signature(&s.signature));
        o.oneof(18, &access(&s.access));
        for x in &s.overridden_symbols {
            o.bytes(19, x.as_bytes());
        }
    })
}

fn annotation(a: &Annotation) -> Vec<u8> {
    build(|o| o.oneof(1, &ty(&a.tpe)))
}

fn scope(s: &Scope) -> Vec<u8> {
    build(|o| {
        for x in &s.symlinks {
            o.bytes(1, x.as_bytes());
        }
        for x in &s.hardlinks {
            o.message(2, &information(x));
        }
    })
}

fn opt_scope(o: &mut Out, field: u32, s: &Option<Scope>) {
    if let Some(s) = s {
        o.message(field, &scope(s));
    }
}

fn signature(s: &Signature) -> Vec<u8> {
    let (field, inner) = match s {
        Signature::Empty => return Vec::new(),
        Signature::Class { type_parameters, parents, self_type, declarations } => (
            1,
            build(|o| {
                opt_scope(o, 1, type_parameters);
                for p in parents {
                    o.message(2, &ty(p));
                }
                o.oneof(3, &ty(self_type));
                opt_scope(o, 4, declarations);
            }),
        ),
        Signature::Method { type_parameters, parameter_lists, return_type } => (
            2,
            build(|o| {
                opt_scope(o, 1, type_parameters);
                for p in parameter_lists {
                    o.message(2, &scope(p));
                }
                o.oneof(3, &ty(return_type));
            }),
        ),
        Signature::Type { type_parameters, lower_bound, upper_bound } => (
            3,
            build(|o| {
                opt_scope(o, 1, type_parameters);
                o.oneof(2, &ty(lower_bound));
                o.oneof(3, &ty(upper_bound));
            }),
        ),
        Signature::Value { tpe } => (4, build(|o| o.oneof(1, &ty(tpe)))),
    };
    build(|o| o.message(field, &inner))
}

fn ty(t: &Type) -> Vec<u8> {
    let (field, inner) = match t {
        Type::Empty => return Vec::new(),
        Type::TypeRef { prefix, symbol, type_arguments } => (
            2,
            build(|o| {
                o.oneof(1, &ty(prefix));
                o.string(2, symbol);
                for a in type_arguments {
                    o.message(3, &ty(a));
                }
            }),
        ),
        Type::StructuralType { tpe, declarations } => (
            7,
            build(|o| {
                o.oneof(4, &ty(tpe));
                opt_scope(o, 5, declarations);
            }),
        ),
        Type::AnnotatedType { annotations, tpe } => (
            8,
            build(|o| {
                o.oneof(1, &ty(tpe));
                for a in annotations {
                    o.message(3, &annotation(a));
                }
            }),
        ),
        Type::ExistentialType { tpe, declarations } => (
            9,
            build(|o| {
                o.oneof(1, &ty(tpe));
                opt_scope(o, 3, declarations);
            }),
        ),
        Type::ByNameType { tpe } => (13, build(|o| o.oneof(1, &ty(tpe)))),
        Type::RepeatedType { tpe } => (14, build(|o| o.oneof(1, &ty(tpe)))),
        Type::IntersectionType { types } => (17, build(|o| types.iter().for_each(|x| o.message(1, &ty(x))))),
        Type::UnionType { types } => (18, build(|o| types.iter().for_each(|x| o.message(1, &ty(x))))),
        Type::SingleType { prefix, symbol } => (
            20,
            build(|o| {
                o.oneof(1, &ty(prefix));
                o.string(2, symbol);
            }),
        ),
        Type::ThisType { symbol } => (21, build(|o| o.string(1, symbol))),
        Type::SuperType { prefix, symbol } => (
            22,
            build(|o| {
                o.oneof(1, &ty(prefix));
                o.string(2, symbol);
            }),
        ),
        Type::ConstantType { constant: c } => (23, build(|o| o.oneof(1, &constant(c)))),
        Type::MatchType { scrutinee, cases } => (
            25,
            build(|o| {
                o.oneof(1, &ty(scrutinee));
                for c in cases {
                    o.message(
                        2,
                        &build(|o| {
                            o.oneof(1, &ty(&c.key));
                            o.oneof(2, &ty(&c.body));
                        }),
                    );
                }
            }),
        ),
        Type::LambdaType { parameters, return_type } => (
            26,
            build(|o| {
                opt_scope(o, 1, parameters);
                o.oneof(2, &ty(return_type));
            }),
        ),
    };
    build(|o| o.message(field, &inner))
}

fn constant(c: &Constant) -> Vec<u8> {
    let (field, inner) = match c {
        Constant::Empty => return Vec::new(),
        Constant::Unit => (1, Vec::new()),
        Constant::Boolean(v) => (2, build(|o| o.boolean(1, *v))),
        Constant::Byte(v) => (3, build(|o| o.int32(1, *v))),
        Constant::Short(v) => (4, build(|o| o.int32(1, *v))),
        Constant::Char(v) => (5, build(|o| o.int32(1, *v))),
        Constant::Int(v) => (6, build(|o| o.int32(1, *v))),
        Constant::Long(v) => (7, build(|o| o.int64(1, *v))),
        Constant::Float(v) => (8, build(|o| o.float(1, *v))),
        Constant::Double(v) => (9, build(|o| o.double(1, *v))),
        Constant::String(v) => (10, build(|o| o.string(1, v))),
        Constant::Null => (11, Vec::new()),
    };
    build(|o| o.message(field, &inner))
}

fn access(a: &Access) -> Vec<u8> {
    let (field, inner) = match a {
        Access::Empty => return Vec::new(),
        Access::Private => (1, Vec::new()),
        Access::PrivateThis => (2, Vec::new()),
        Access::PrivateWithin(s) => (3, build(|o| o.string(1, s))),
        Access::Protected => (4, Vec::new()),
        Access::ProtectedThis => (5, Vec::new()),
        Access::ProtectedWithin(s) => (6, build(|o| o.string(1, s))),
        Access::Public => (7, Vec::new()),
    };
    build(|o| o.message(field, &inner))
}

fn range(r: &Range) -> Vec<u8> {
    build(|o| {
        o.int32(1, r.start_line);
        o.int32(2, r.start_character);
        o.int32(3, r.end_line);
        o.int32(4, r.end_character);
    })
}

fn occurrence(x: &SymbolOccurrence) -> Vec<u8> {
    build(|o| {
        if let Some(r) = &x.range {
            o.message(1, &range(r));
        }
        o.string(2, &x.symbol);
        o.int32(3, x.role as i32);
    })
}

fn synthetic(s: &Synthetic) -> Vec<u8> {
    build(|o| {
        if let Some(r) = &s.range {
            o.message(1, &range(r));
        }
        o.oneof(2, &tree(&s.tree));
    })
}

fn tree(t: &Tree) -> Vec<u8> {
    let (field, inner) = match t {
        Tree::Empty => return Vec::new(),
        Tree::Apply { function, arguments, properties } => (
            1,
            build(|o| {
                o.oneof(1, &tree(function));
                for a in arguments {
                    o.message(2, &tree(a));
                }
                o.int32(3, *properties);
            }),
        ),
        Tree::Id { symbol } => (3, build(|o| o.string(1, symbol))),
        Tree::Original { range: r } => (
            6,
            build(|o| {
                if let Some(r) = r {
                    o.message(1, &range(r));
                }
            }),
        ),
        Tree::Select { qualifier, id } => (
            7,
            build(|o| {
                o.oneof(1, &tree(qualifier));
                if let Some(s) = id {
                    o.message(2, &build(|o| o.string(1, s)));
                }
            }),
        ),
        Tree::TypeApply { function, type_arguments } => (
            8,
            build(|o| {
                o.oneof(1, &tree(function));
                for a in type_arguments {
                    o.message(2, &ty(a));
                }
            }),
        ),
    };
    build(|o| o.message(field, &inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_as_the_generated_writers_write_them() {
        // A negative int32 is ten bytes, sign-extended; `-0.0` is omitted as `0.0` is, a NaN written as the canonical
        // one whatever its sign and payload (`floatToIntBits`, `doubleToLongBits`).
        assert_eq!(constant(&Constant::Int(-1)), [0x32, 0x0b, 0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]);
        assert_eq!(constant(&Constant::Double(-0.0)), [0x4a, 0x00]);
        assert_eq!(constant(&Constant::Float(f32::NAN)), [0x42, 0x05, 0x0d, 0x00, 0x00, 0xc0, 0x7f]);
        assert_eq!(constant(&Constant::Float(f32::from_bits(0xffc0_0001))), [0x42, 0x05, 0x0d, 0x00, 0x00, 0xc0, 0x7f]);
        let canonical_nan = [0x4a, 0x09, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x7f];
        assert_eq!(constant(&Constant::Double(f64::from_bits(0xfff8_0000_0000_0123))), canonical_nan);
        assert_eq!(constant(&Constant::Boolean(false)), [0x12, 0x00]);
        assert_eq!(constant(&Constant::Unit), [0x0a, 0x00]);
    }

    #[test]
    fn oneofs_omitted_when_empty_in_a_singular_field_kept_in_a_repeated_one() {
        let v = Signature::Value { tpe: Type::Empty };
        assert_eq!(signature(&v), [0x22, 0x00]);
        let r = Type::TypeRef { prefix: Box::new(Type::Empty), symbol: "a/B#".into(), type_arguments: vec![Type::Empty] };
        assert_eq!(ty(&r), [0x12, 0x08, 0x12, 0x04, b'a', b'/', b'B', b'#', 0x1a, 0x00]);
        let x = SymbolOccurrence { range: Some(Range::default()), symbol: String::new(), role: Role::Unknown };
        assert_eq!(occurrence(&x), [0x0a, 0x00]);
    }

    /// The synthetics of tests/semanticdb/corpus/given/Given.scala, a `using` clause's arguments applied at the call and
    /// in `summon`, as scalac 3.8.4 writes them: each `ApplyTree` with `properties = GIVEN` (SyntheticsExtractor.scala
    /// 65-78), field 3, the bytes of its document's `synthetics` (the corpus's gold).
    #[test]
    fn an_application_of_given_arguments_keeps_its_properties() {
        let apply = |l: i32, a: i32, b: i32, arg: &str| Synthetic {
            range: Some(Range { start_line: l, start_character: a, end_line: l, end_character: b }),
            tree: Tree::Apply {
                function: Box::new(Tree::Original { range: Some(Range { start_line: l, start_character: a, end_line: l, end_character: b }) }),
                arguments: vec![Tree::Id { symbol: arg.into() }],
                properties: property::GIVEN,
            },
        };
        let hex = |s: &str| (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let summoned = "0a080802101a1802202512310a2f0a0c320a0a080802101a18022025121b1a190a175f656d7074795f2f476976656e2e6628292e287824312918808004";
        let called = "0a080803100a1803200b12320a300a0c320a0a080803100a1803200b121c1a1a0a185f656d7074795f2f476976656e2e676976656e5f496e742e18808004";
        assert_eq!(synthetic(&apply(2, 26, 37, "_empty_/Given.f().(x$1)")), hex(summoned));
        assert_eq!(synthetic(&apply(3, 10, 11, "_empty_/Given.given_Int.")), hex(called));
    }
}
