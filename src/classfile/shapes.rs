//! Which shapes of Java signatures teq's `Type` can state, as `tasty::shapes` decides it for
//! TASTy: one classification serves `teq classfile --stats` and, later, the loader.

use super::sig::{ClassSig, JType, MethodSig, Prim, TypeArg, TypeParam};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Shape {
    /// `?` as a type argument: teq's `Wild`.
    Wildcard,
    /// `? extends T`: teq's wildcard keeps no bounds (the loader keeps the bound a covariant
    /// parameter allows, as for TASTy).
    ExtendsWildcard,
    /// `? super T`
    SuperWildcard,
    /// A generic class named without arguments, `List` for `List<E>`: read as `List[?]`, which
    /// is scalac's reading of a raw type.
    RawType,
    /// `Outer<A>.Inner<B>`: the inner class of a parameterized outer, a projection.
    InnerOfGeneric,
    /// `byte`, `short` and `float`: the builtins `Byte`, `Short` and `Float`.
    ByteShortFloat,
    /// `T extends A & B`: `Inter`.
    IntersectionBound,
    /// `T extends Comparable<T>`: a bound naming its own parameter.
    RecursiveBound,
    /// A parameterized type as a type argument, `List<List<T>>`.
    NestedGeneric,
    /// `int[]`: `Array[Int]`.
    PrimitiveArray,
    /// `T...`: `T*`.
    Varargs,
    /// A `throws` clause: dropped, teq checks no exceptions.
    Throws,
    /// `java.lang.Object` in a signature: read as `AnyRef` by the loader today; scalac reads it
    /// as `Any` (`FromJavaObject`), so that a boxed `Int` passes where Java takes an `Object`.
    JavaObject,
    /// A type variable: `Param`.
    TypeVariable,
    /// A generic class type with arguments: `Class(c, args)`.
    Applied,
}

pub const SHAPE_COUNT: usize = 15;

pub const ALL_SHAPES: [Shape; SHAPE_COUNT] = [
    Shape::Wildcard,
    Shape::ExtendsWildcard,
    Shape::SuperWildcard,
    Shape::RawType,
    Shape::InnerOfGeneric,
    Shape::ByteShortFloat,
    Shape::IntersectionBound,
    Shape::RecursiveBound,
    Shape::NestedGeneric,
    Shape::PrimitiveArray,
    Shape::Varargs,
    Shape::Throws,
    Shape::JavaObject,
    Shape::TypeVariable,
    Shape::Applied,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mapping {
    Maps,
    Approximated,
    Blocked,
}

impl Shape {
    pub fn mapping(self) -> Mapping {
        match self {
            Shape::ExtendsWildcard | Shape::SuperWildcard | Shape::RawType => Mapping::Approximated,
            Shape::InnerOfGeneric => Mapping::Blocked,
            _ => Mapping::Maps,
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Shape::Wildcard => "unbounded wildcard (?)",
            Shape::ExtendsWildcard => "wildcard with an upper bound (? extends T)",
            Shape::SuperWildcard => "wildcard with a lower bound (? super T)",
            Shape::RawType => "raw type (List for List<E>)",
            Shape::InnerOfGeneric => "inner class of a parameterized outer (Outer<A>.Inner)",
            Shape::ByteShortFloat => "byte, short or float",
            Shape::IntersectionBound => "intersection bound (T extends A & B)",
            Shape::RecursiveBound => "recursive bound (T extends Comparable<T>)",
            Shape::NestedGeneric => "nested generic (List<List<T>>)",
            Shape::PrimitiveArray => "primitive array (int[])",
            Shape::Varargs => "varargs (T...)",
            Shape::Throws => "throws clause",
            Shape::JavaObject => "java.lang.Object",
            Shape::TypeVariable => "type variable",
            Shape::Applied => "parameterized type (List<T>)",
        }
    }
}

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

/// What a walk over the types of one signature found.
#[derive(Default)]
pub struct Found {
    pub shapes: Shapes,
    pub occurrences: [u32; SHAPE_COUNT],
    /// Classes named without arguments that the arity table does not know: generic or not,
    /// the name alone does not tell, so the signature is raw or not.
    pub unknown_classes: Vec<String>,
}

impl Found {
    fn hit(&mut self, s: Shape) {
        self.shapes.add(s);
        self.occurrences[s as usize] += 1;
    }

    pub fn mapping(&self) -> Mapping {
        let mut m = Mapping::Maps;
        for s in ALL_SHAPES {
            if self.shapes.has(s) {
                match s.mapping() {
                    Mapping::Blocked => return Mapping::Blocked,
                    Mapping::Approximated => m = Mapping::Approximated,
                    Mapping::Maps => {}
                }
            }
        }
        m
    }
}

/// The number of type parameters of every class the input and the JDK define, which is what
/// tells a raw type from a class that takes no arguments.
#[derive(Default)]
pub struct Arities {
    pub of: HashMap<String, u16>,
}

pub struct Classifier<'a> {
    pub arities: &'a Arities,
}

impl<'a> Classifier<'a> {
    pub fn walk(&self, t: &JType, found: &mut Found) {
        self.walk_in(t, found, false);
    }

    fn walk_in(&self, t: &JType, found: &mut Found, as_argument: bool) {
        match t {
            JType::Prim(Prim::Byte | Prim::Short | Prim::Float) => found.hit(Shape::ByteShortFloat),
            JType::Prim(_) | JType::Void => {}
            JType::Var(_) => found.hit(Shape::TypeVariable),
            JType::Array(elem) => {
                if matches!(**elem, JType::Prim(_)) {
                    found.hit(Shape::PrimitiveArray);
                }
                self.walk_in(elem, found, as_argument);
            }
            JType::Class(c) => {
                if c.outer.is_some() {
                    found.hit(Shape::InnerOfGeneric);
                }
                if c.args.is_empty() {
                    if c.name == "java/lang/Object" {
                        found.hit(Shape::JavaObject);
                    } else {
                        match self.arities.of.get(&c.name) {
                            Some(&n) if n > 0 => found.hit(Shape::RawType),
                            Some(_) => {}
                            None => found.unknown_classes.push(c.name.clone()),
                        }
                    }
                } else {
                    found.hit(Shape::Applied);
                    if as_argument {
                        found.hit(Shape::NestedGeneric);
                    }
                }
                for a in &c.args {
                    match a {
                        TypeArg::Wild => found.hit(Shape::Wildcard),
                        TypeArg::Extends(t) => {
                            found.hit(Shape::ExtendsWildcard);
                            self.walk_in(t, found, true);
                        }
                        TypeArg::Super(t) => {
                            found.hit(Shape::SuperWildcard);
                            self.walk_in(t, found, true);
                        }
                        TypeArg::Exact(t) => self.walk_in(t, found, true),
                    }
                }
            }
        }
    }

    pub fn tparams(&self, ps: &[TypeParam], found: &mut Found) {
        for p in ps {
            if p.bounds().count() > 1 {
                found.hit(Shape::IntersectionBound);
            }
            if p.bounds().any(|b| mentions(b, &p.name, 0)) {
                found.hit(Shape::RecursiveBound);
            }
            for b in p.bounds() {
                self.walk(b, found);
            }
        }
    }

    pub fn method(&self, sig: &MethodSig, varargs: bool) -> Found {
        let mut found = Found::default();
        self.tparams(&sig.tparams, &mut found);
        for p in &sig.params {
            self.walk(p, &mut found);
        }
        if varargs && matches!(sig.params.last(), Some(JType::Array(_))) {
            found.hit(Shape::Varargs);
        }
        self.walk(&sig.ret, &mut found);
        if !sig.throws.is_empty() {
            found.hit(Shape::Throws);
        }
        for t in &sig.throws {
            self.walk(t, &mut found);
        }
        found
    }

    pub fn class_header(&self, sig: &ClassSig) -> Found {
        let mut found = Found::default();
        self.tparams(&sig.tparams, &mut found);
        if !matches!(sig.superclass, JType::Void) && !sig.superclass.is_class("java/lang/Object") {
            self.walk(&sig.superclass, &mut found);
        }
        for i in &sig.interfaces {
            self.walk(i, &mut found);
        }
        found
    }
}

fn mentions(t: &JType, var: &str, depth: u32) -> bool {
    if depth > 64 {
        return false;
    }
    match t {
        JType::Var(v) => v == var,
        JType::Array(e) => mentions(e, var, depth + 1),
        JType::Class(c) => {
            c.args.iter().any(|a| match a {
                TypeArg::Wild => false,
                TypeArg::Extends(t) | TypeArg::Super(t) | TypeArg::Exact(t) => mentions(t, var, depth + 1),
            }) || c.outer.as_ref().map_or(false, |o| mentions(&JType::Class((**o).clone()), var, depth + 1))
        }
        _ => false,
    }
}
