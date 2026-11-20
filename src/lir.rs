//! The lowered IR of the class-based targets: a method body after the platform lowering
//! (erasure, boxing, the choice of each member's reference), as a tree of nodes that leave
//! values of erased types, and the erased names it speaks in: types, and the references to
//! classes and members that stand where an erased symbol table would. The JVM back end writes
//! bytecode from it; a node the lowering does not make yet is `Legacy`, which the JVM back
//! end generates from the typed IR as it did before (docs/LIR.md).

use crate::tir::{StrRef, TExprId};
use crate::types::{ClassId, SymId, TList};
use std::rc::Rc;

/// An erased type: a primitive, or a class or interface by its binary name with `/` between
/// the packages (an array is its element's descriptor behind `[`). `V` is no value: the result
/// of a method that returns `Unit`. The variants are spelled as the class file's descriptors
/// spell them (docs/LIR.md, "The vocabulary").
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    V,
    Z,
    B,
    S,
    C,
    I,
    J,
    F,
    D,
    L(Rc<str>),
}

/// How a method is reached: through the receiver's class, through an interface, with no
/// receiver, or the named class's own (a constructor, a private method, a `super` call).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Invoke {
    Virtual,
    Interface,
    Static,
    Special,
}

/// An erased method: the class that declares it, its encoded name and its erased signature.
#[derive(Clone)]
pub struct Method {
    pub owner: Rc<str>,
    pub owner_is_interface: bool,
    pub name: String,
    pub params: Vec<Type>,
    pub ret: Type,
    /// The signature as a class file spells it, made once with the reference.
    pub desc: String,
    pub kind: Invoke,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LirId(pub u32);

/// Nodes in order, a slice of `Body::lists`.
#[derive(Clone, Copy, Debug)]
pub struct List {
    pub start: u32,
    pub len: u32,
}

pub struct Node {
    pub kind: Kind,
    /// The erased type of the value the node leaves, `Type::V` for none.
    pub ty: Type,
    /// The expression of the typed IR the node is the whole of, whose position its code takes.
    pub of: Option<TExprId>,
}

#[derive(Clone)]
pub enum Kind {
    /// An expression of the typed IR brought to the node's type, not lowered yet.
    Legacy(TExprId),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    Char(u16),
    Str(StrRef),
    Unit,
    /// A local or a parameter held as its value.
    Local(SymId),
    /// The instance of an object, by its class's name.
    Module(Rc<str>),
    /// The instance the method runs on, at the node's type.
    This,
    /// A field of an instance read, named on the class `owner`.
    Field { receiver: LirId, owner: Rc<str>, name: Rc<str> },
    /// A call of a method on a receiver: the receiver, then the arguments as they are written,
    /// each at the type its parameter takes. `at` is the class the reference names where that
    /// is the receiver's and not the declaring one's (dotty's `receiverClass`), with whether it
    /// is an interface. `order`, where the method takes its parameters in another order than
    /// the arguments are written and evaluated in (a right-associative extension), is the
    /// argument each parameter takes.
    Call { method: Rc<Method>, at: Option<(Rc<str>, bool)>, receiver: LirId, args: List, order: Option<Rc<[usize]>> },
    /// An argument of the typed IR for a Java array parameter, at the node's type (a varargs
    /// literal's elements, an array spliced in, a sequence copied); not lowered yet
    /// (`Gen::java_array_argument`).
    JavaArrayArgument(TExprId),
    /// `try`: the body, the cases of the typed IR's `tries[index]` (their patterns are not
    /// lowered yet), and the finalizer, which runs on every way out.
    Try { index: u32, body: LirId, finalizer: Option<LirId> },
    /// A value of the type `from` as one of the node's type: dropped, the unit value or a zero
    /// for none, a cast, a box or its content, a primitive conversion (`Gen::adapt`).
    Adapt { value: LirId, from: Type },
    /// A value of the value class `class` applied to `args`, left as `from`, as one of the
    /// node's type: the box opened or the underlying value boxed; a null reference gives the
    /// underlying type's zero where `null_safe`. The cases are not lowered yet
    /// (`Gen::adapt_vc_as`).
    ValueClass { value: LirId, from: Type, class: ClassId, args: TList, null_safe: bool },
}

/// The nodes of one method.
#[derive(Default)]
pub struct Body {
    pub nodes: Vec<Node>,
    pub lists: Vec<LirId>,
}

impl Body {
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.lists.clear();
    }

    pub fn list(&mut self, items: &[LirId]) -> List {
        let start = self.lists.len() as u32;
        self.lists.extend_from_slice(items);
        List { start, len: items.len() as u32 }
    }

    pub fn push(&mut self, kind: Kind, ty: Type) -> LirId {
        self.nodes.push(Node { kind, ty, of: None });
        LirId(self.nodes.len() as u32 - 1)
    }

    pub fn node(&self, id: LirId) -> &Node {
        &self.nodes[id.0 as usize]
    }

    pub fn ty(&self, id: LirId) -> &Type {
        &self.nodes[id.0 as usize].ty
    }
}
