//! Which TASTy types teq's `Type` can state. One classification serves the statistics of
//! `teq tasty --stats` and the loader, so that the numbers describe what the loader does.

use super::tree::{Const, Decoder, LambdaKind, LocalKind, TType};
use super::TastyFile;

/// A shape of type that `Type` has no case for yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Shape {
    /// `this.T` or `Outer.T` for an abstract type member `T`.
    TypeMember,
    /// `x.T` for a value `x`: a type member selected from a parameter or a val.
    PathDependent,
    /// `x.type`, `C.this.type`: the singleton type of a path.
    Singleton,
    /// `T { type A = ...; def m: ... }`
    Refinement,
    /// `[T] => T => List[T]`, a refinement of `PolyFunction`.
    PolyFunction,
    MatchType,
    /// A refinement that names its own members.
    Recursive,
    /// `(A, B) ?=> R`
    ContextFunction,
    /// An annotation other than the ones that change nothing about the type.
    Annotated,
    /// `T#A` for a type `T` that is no object.
    Projection,
    /// `? >: L <: H` as an argument: teq's wildcard keeps no bounds.
    BoundedWildcard,
    /// `=> T` anywhere but directly as the type of a parameter: a function type's parameter.
    ByNameInside,
    /// A literal type of a value teq has no literal for (`Byte`, `Short`, `Float`, `null`, `classOf`).
    ConstantType,
    /// `AnyKind` and what is bounded by it.
    AnyKind,
    /// A nullable type of a Java signature under explicit nulls.
    JavaFlexible,
    SuperType,
    /// A tag the reader has no reading for.
    Unknown,
}

pub const SHAPE_COUNT: usize = 17;

pub const ALL_SHAPES: [Shape; SHAPE_COUNT] = [
    Shape::TypeMember,
    Shape::PathDependent,
    Shape::Singleton,
    Shape::Refinement,
    Shape::PolyFunction,
    Shape::MatchType,
    Shape::Recursive,
    Shape::ContextFunction,
    Shape::Annotated,
    Shape::Projection,
    Shape::BoundedWildcard,
    Shape::ByNameInside,
    Shape::ConstantType,
    Shape::AnyKind,
    Shape::JavaFlexible,
    Shape::SuperType,
    Shape::Unknown,
];

impl Shape {
    /// Whether teq reads the shape today as something weaker instead of refusing it: a bounded
    /// wildcard without its bounds, an annotated type without the annotation, a Java nullable
    /// type as the type. A signature with such a shape loads and checks, less precisely than
    /// scalac would.
    pub fn approximated(self) -> bool {
        matches!(self, Shape::BoundedWildcard | Shape::Annotated | Shape::JavaFlexible)
    }

    /// Shapes `Type` states since type members and match types: an abstract member through
    /// `this` or a path, a singleton type, a refinement, a projection, a match type, a by-name
    /// parameter of a function type.
    pub fn supported(self) -> bool {
        matches!(self, Shape::TypeMember | Shape::PathDependent | Shape::Singleton | Shape::Refinement | Shape::Projection | Shape::MatchType | Shape::ByNameInside)
    }

    pub fn describe(self) -> &'static str {
        match self {
            Shape::TypeMember => "abstract type member (this.T, Outer.T)",
            Shape::PathDependent => "path-dependent type (x.T)",
            Shape::Singleton => "singleton type (x.type, C.this.type)",
            Shape::Refinement => "refinement (T { ... })",
            Shape::PolyFunction => "polymorphic function type",
            Shape::MatchType => "match type",
            Shape::Recursive => "recursive refinement",
            Shape::ContextFunction => "context function type",
            Shape::Annotated => "annotated type",
            Shape::Projection => "type projection (T#A)",
            Shape::BoundedWildcard => "wildcard with bounds",
            Shape::ByNameInside => "by-name type inside a type",
            Shape::ConstantType => "constant type without a teq literal",
            Shape::AnyKind => "AnyKind",
            Shape::JavaFlexible => "Java nullable (flexible) type",
            Shape::SuperType => "super type",
            Shape::Unknown => "unknown tag",
        }
    }
}

/// A function class of package `scala` by its name. A library compiled under capture checking
/// (scala-library since 3.8) writes `A => B` as `ImpureFunction1[A, B]` and `A ?=> B` as
/// `ImpureContextFunction1[A, B]`; teq checks no captures, so both read as the plain types.
pub fn function_class(name: &str) -> Option<(bool, usize)> {
    let name = name.strip_prefix("Impure").unwrap_or(name);
    let (context, digits) = match name.strip_prefix("ContextFunction") {
        Some(d) => (true, d),
        None => (false, name.strip_prefix("Function")?),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((context, digits.parse().ok()?))
}

/// A set of shapes.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Shapes(pub u32);

impl Shapes {
    pub fn add(&mut self, s: Shape) {
        self.0 |= 1 << s as u8;
    }
    pub fn has(self, s: Shape) -> bool {
        self.0 & (1 << s as u8) != 0
    }
}

/// Annotations that say nothing the type system of teq would have to keep.
const TRANSPARENT_ANNOTATIONS: &[&str] =
    &["uncheckedVariance", "Repeated", "unchecked", "unused", "nowarn", "uncheckedStable", "retainsCap", "retains"];

/// What a walk over the types of one signature found.
#[derive(Default)]
pub struct Found {
    pub shapes: Shapes,
    /// Occurrences per shape, by `Shape as usize`.
    pub occurrences: [u32; SHAPE_COUNT],
    /// `Outer.T` or `this.T` that names a type of another file: a class or an alias maps, an
    /// abstract type member does not, and the name alone does not tell which. A type of the same
    /// file is looked up and counted for what it is.
    pub static_members: u32,
    /// What those references name, as `Outer.T`.
    pub static_targets: Vec<String>,
    /// What the type member shapes name: `Owner.T` for `this.T`, `x.T` by the member for a path.
    pub member_targets: Vec<(Shape, String)>,
}

impl Found {
    fn hit(&mut self, s: Shape) {
        self.shapes.add(s);
        self.occurrences[s as usize] += 1;
    }
}

pub struct Classifier<'a> {
    pub file: &'a TastyFile,
    pub decoder: &'a Decoder<'a>,
}

impl<'a> Classifier<'a> {
    fn simple_name(&self, t: &TType) -> Option<&'a str> {
        match t {
            TType::TypeRef(_, n) => self.file.simple(self.file.source_name(*n)),
            TType::Applied(tycon, _) => self.simple_name(tycon),
            _ => None,
        }
    }

    fn type_name(&self, t: &TType) -> String {
        let n = match t {
            TType::TypeRef(_, n) | TType::TermRef(_, n) => Some(*n),
            TType::LocalType(addr, _) | TType::LocalTerm(addr, _) => self.decoder.name_at(*addr),
            _ => None,
        };
        n.map_or("?".to_string(), |n| self.file.name(self.file.source_name(n)))
    }

    fn is_package(&self, t: &TType) -> bool {
        match t {
            TType::Package(_) => true,
            TType::This(inner) => self.is_package(inner),
            _ => false,
        }
    }

    /// A path made of packages and objects, which names one place in the program.
    fn is_static_path(&self, t: &TType) -> bool {
        match t {
            TType::Package(_) => true,
            TType::This(_) => true,
            TType::TermRef(prefix, _) => self.is_static_path(prefix),
            TType::TypeRef(prefix, n) => self.file.is_object_class(*n) && self.is_static_path(prefix),
            TType::LocalTerm(addr, prefix) => {
                self.decoder.is_module_val(*addr) && prefix.as_ref().map_or(true, |p| self.is_static_path(p))
            }
            // A class of this file is a place only when it is the class of an object; any other
            // class as a prefix makes a projection `C#T`.
            TType::LocalType(addr, prefix) => {
                self.decoder.name_at(*addr).map_or(false, |n| self.file.is_object_class(n))
                    && prefix.as_ref().map_or(true, |p| self.is_static_path(p))
            }
            _ => false,
        }
    }

    /// The type of a parameter or a result: a by-name type is a parameter's own business.
    pub fn top(&self, t: &TType, found: &mut Found) {
        match t {
            TType::ByName(u) => self.walk(u, found),
            other => self.walk(other, found),
        }
    }

    pub fn walk(&self, t: &TType, found: &mut Found) {
        match t {
            TType::Package(_) | TType::ParamRef(..) => {}
            TType::TypeRef(prefix, n) => {
                if self.file.is_object_class(*n) {
                    // `Obj.type`: teq types an object by its class.
                    if !self.is_static_path(prefix) {
                        found.hit(Shape::Singleton);
                    }
                    return;
                }
                if self.file.simple(*n) == Some("AnyKind") && self.is_package(prefix) {
                    found.hit(Shape::AnyKind);
                    return;
                }
                if self.is_package(prefix) {
                    return;
                }
                if self.is_static_path(prefix) {
                    found.static_members += 1;
                    let owner = match &**prefix {
                        TType::TermRef(_, o) | TType::TypeRef(_, o) => self.file.name(self.file.source_name(*o)),
                        TType::This(inner) => self.simple_name(inner).unwrap_or("this").to_string(),
                        _ => "?".to_string(),
                    };
                    found.static_targets.push(format!("{}.{}", owner, self.file.name(*n)));
                    return;
                }
                match &**prefix {
                    TType::TermRef(..) | TType::LocalTerm(..) => {
                        found.hit(Shape::PathDependent);
                        found.member_targets.push((Shape::PathDependent, format!("_.{}", self.file.name(*n))));
                    }
                    TType::RecThis(_) => found.hit(Shape::Recursive),
                    other => {
                        found.hit(Shape::Projection);
                        self.walk(other, found);
                    }
                }
            }
            TType::LocalType(addr, prefix) => match prefix.as_deref() {
                None => {}
                Some(p) if self.is_static_path(p) => {
                    if self.decoder.local_type_kind(*addr) == LocalKind::AbstractMember {
                        found.hit(Shape::TypeMember);
                        let owner = match p {
                            TType::This(inner) => self.type_name(inner),
                            other => self.type_name(other),
                        };
                        let member = self.decoder.name_at(*addr).map_or(String::new(), |n| self.file.name(n));
                        found.member_targets.push((Shape::TypeMember, format!("{}.{}", owner, member)));
                    }
                }
                Some(TType::TermRef(..)) | Some(TType::LocalTerm(..)) => {
                    found.hit(Shape::PathDependent);
                    let member = self.decoder.name_at(*addr).map_or(String::new(), |n| self.file.name(n));
                    found.member_targets.push((Shape::PathDependent, format!("_.{}", member)));
                }
                Some(TType::RecThis(_)) => found.hit(Shape::Recursive),
                Some(other) => {
                    found.hit(Shape::Projection);
                    self.walk(other, found);
                }
            },
            TType::TermRef(prefix, _) => {
                // An object named by its val on a static path is the object's class.
                if !self.is_static_path(prefix) {
                    found.hit(Shape::Singleton);
                }
            }
            TType::LocalTerm(..) if self.is_static_path(t) => {}
            TType::LocalTerm(..) | TType::This(_) => found.hit(Shape::Singleton),
            TType::Applied(tycon, args) => {
                if self.is_package_member(tycon) {
                    if let Some((true, _)) = self.simple_name(tycon).and_then(function_class) {
                        found.hit(Shape::ContextFunction);
                    }
                }
                self.walk(tycon, found);
                for a in args {
                    match a {
                        TType::Bounds(lo, hi) => {
                            let open = self.is_scala(lo, "Nothing") && self.is_scala(hi, "Any");
                            if !open {
                                found.hit(Shape::BoundedWildcard);
                            }
                            self.walk(lo, found);
                            self.walk(hi, found);
                        }
                        other => self.walk(other, found),
                    }
                }
            }
            TType::Bounds(lo, hi) => {
                self.walk(lo, found);
                self.walk(hi, found);
            }
            TType::Alias(a) => self.walk(a, found),
            TType::BoundedAlias(b, a) => {
                self.walk(b, found);
                self.walk(a, found);
            }
            TType::And(a, b) | TType::Or(a, b) => {
                self.walk(a, found);
                self.walk(b, found);
            }
            TType::ByName(u) => {
                found.hit(Shape::ByNameInside);
                self.walk(u, found);
            }
            TType::Annotated(u, annot) => {
                let name = self.simple_name(annot).unwrap_or("");
                if !TRANSPARENT_ANNOTATIONS.contains(&name) {
                    found.hit(Shape::Annotated);
                }
                self.walk(u, found);
            }
            TType::Refined(parent, members) => {
                let poly = self.simple_name(parent) == Some("PolyFunction");
                found.hit(if poly { Shape::PolyFunction } else { Shape::Refinement });
                self.walk(parent, found);
                for (_, info) in members {
                    if let Some(i) = info {
                        self.walk(i, found);
                    }
                }
            }
            TType::Rec(_, u) => {
                found.hit(Shape::Recursive);
                self.walk(u, found);
            }
            TType::RecThis(_) => found.hit(Shape::Recursive),
            TType::Super(..) => found.hit(Shape::SuperType),
            TType::Lambda { kind, params, result, .. } => {
                // Method types occur in refinements, which are counted where they stand.
                for p in params {
                    match (kind, &p.info) {
                        (LambdaKind::Method, info) => self.top(info, found),
                        (_, info) => self.walk(info, found),
                    }
                }
                self.top(result, found);
            }
            TType::Match { scrutinee, bound, cases } => {
                found.hit(Shape::MatchType);
                self.walk(scrutinee, found);
                if let Some(b) = bound {
                    self.walk(b, found);
                }
                for c in cases {
                    self.walk(&c.pattern, found);
                    self.walk(&c.body, found);
                }
            }
            TType::MatchCase(p, b) => {
                self.walk(p, found);
                self.walk(b, found);
            }
            TType::Flexible(u) => {
                found.hit(Shape::JavaFlexible);
                self.walk(u, found);
            }
            TType::Const(c) => match c {
                Const::Int(_) | Const::Long(_) | Const::Double(_) | Const::Char(_) | Const::Bool(_) | Const::Str(_) => {}
                Const::Unit | Const::Byte(_) | Const::Short(_) | Const::Float(_) | Const::Null | Const::Class(_) => {
                    found.hit(Shape::ConstantType)
                }
            },
            TType::Unknown(_) => found.hit(Shape::Unknown),
        }
    }

    fn is_package_member(&self, t: &TType) -> bool {
        matches!(t, TType::TypeRef(prefix, _) if self.is_package(prefix))
    }

    fn is_scala(&self, t: &TType, wanted: &str) -> bool {
        match t {
            TType::TypeRef(prefix, n) => {
                self.file.simple(*n) == Some(wanted)
                    && matches!(&**prefix, TType::Package(p) if self.file.simple(*p) == Some("scala"))
            }
            _ => false,
        }
    }
}
