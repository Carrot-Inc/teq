//! The values of the interpreter, and what needs no program to compute over them: string
//! conversion of the primitives as the JVM formats them, hashes, UTF-16 views of strings.

use super::Frame;
use crate::source::FileId;
use crate::tir::{FunId, TExprId, TPatId};
use crate::types::{AliasId, ClassId, LitVal, PkgId, SymId, TParamId, TypeId};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Unit,
    Null,
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    Float(f32),
    Byte(i8),
    Short(i16),
    Char(u16),
    Str(Rc<str>),
    Obj(Rc<Object>),
    Fun(Rc<Closure>),
    /// A JavaScript array: `Array[T]`, and the packing the compiler makes for itself.
    Array(Rc<ArrayCell>),
    /// A `RawMap`: an insertion-ordered hash map with `==` equality of keys.
    Map(Rc<MapCell>),
    /// A node of the immutable Map and Set's hash trie index (`TrieNode` in std/maps.scala),
    /// kept native by the natives of the trie's operations: the keys, their `Int` ordinals and
    /// the sub-nodes in `content`, laid out as the Scala class lays them out.
    Trie(Rc<Trie>),
    /// What `getClass` gives: the class of a value, or the boxed class of a primitive.
    Class(Rc<ClassValue>),
    /// A regex match: the groups and where they start, `None` for a group that did not take part.
    Match(Rc<MatchValue>),
    /// A quoted tree: an `Expr` or a `quotes.reflect.Tree` (`src/interp/quoted.rs`).
    Tree(TreeRef),
    /// A `quotes.reflect.TypeRepr`, and a `scala.quoted.Type`.
    Type(TypeId),
    /// A `quotes.reflect.Symbol`.
    Sym(SymRef),
    /// A `quotes.reflect.Position`: the file and the span.
    Pos(FileId, u32, u32),
    /// A `quotes.reflect.SourceFile`.
    Src(FileId),
    /// A slot no write has reached: what a lazy val holds before its first read.
    Absent,
}

/// What a `quotes.reflect.Tree` stands for in the typed IR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeRef {
    Expr(TExprId),
    /// A statement of a block, by its index in `Program::stmts`.
    Stmt(u32),
    Fun(FunId),
    Class(ClassId),
    /// A member or parameter definition, by its symbol.
    Def(SymId),
    /// A case of a match, by its index in `Program::cases`.
    Case(u32),
    Pat(TPatId),
    /// A type tree.
    Type(TypeId),
    /// A selection or reference that awaits its arguments, in `Interp::pending`.
    Pending(u32),
    /// The `DefDef`, the `Closure` and the reference to the method of a lambda, which scalac
    /// shows as a block of the two.
    LambdaDef(TExprId),
    LambdaClosure(TExprId),
    LambdaRef(TExprId),
    /// The primary constructor of a class, as a `DefDef`.
    Ctor(ClassId),
    /// A type alias, as a `TypeDef`.
    Alias(AliasId),
    /// The method scalac makes for the default of a constructor parameter.
    Default(ClassId, u32),
    /// A named argument, `name = arg`, in `Interp::named_args`.
    NamedArg(u32),
    /// A call applied to the arguments of one parameter clause, with more clauses to come, in
    /// `Interp::partials`: the inner `Apply` of a call with several clauses.
    Partial(u32),
}

/// What a `quotes.reflect.Symbol` stands for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymRef {
    Term(SymId),
    Class(ClassId),
    Pkg(PkgId),
    TParam(TParamId),
    Alias(AliasId),
    /// The primary constructor of a class.
    Ctor(ClassId),
    /// The method scalac makes for the default of a constructor parameter.
    Default(ClassId, u32),
    /// The method a lambda closes over.
    Lambda(TExprId),
    /// scalac's `file$package` object, the owner of the top-level definitions of a file.
    FilePackage(FileId),
    /// A package as the owner of what it contains: scalac's package class, where `Pkg` is the
    /// package's term.
    PkgClass(PkgId),
    /// The synthetic `macro` val a macro's expansion stands in, `Symbol.spliceOwner`, of the
    /// expansion whose number the run carries (`MacroCtx::run`): the owner chain is the run's.
    Splice(u32),
    /// An entry of the owner chain of the expansion (by its run's number) without a symbol of
    /// its own: a lambda, a local dummy, a pattern binder or the temporary of a pattern
    /// definition.
    Site(u32, u32),
    /// The root package, which the packages hang off; `Pkg(ROOT_PKG)` is the empty package
    /// of the definitions outside any package clause.
    Root,
    Any,
    Nothing,
    None,
}

pub struct Object {
    pub class: ClassId,
    pub fields: RefCell<Vec<Value>>,
    /// The scope an anonymous class was created in, which its members read their captures from.
    pub env: Option<Rc<Frame>>,
    /// The name and ordinal of an enum value; the ordinal of a class case.
    pub name: Option<Rc<str>>,
    pub ordinal: Cell<i32>,
    /// The identity hash: a function of the object's contents when it was first asked, kept
    /// for the object's life, 0 until then (`identity.rs`).
    pub hash: Cell<i32>,
    /// The object's entry in the registry of a resident (`registry.rs`), or zero.
    pub(super) slot: Cell<u32>,
    /// The epoch that stamped the object, 0 while it is its epoch's alone (`super::watch`).
    pub(super) born: Cell<u32>,
    /// When the object was made, counted while the watch is on (`super::made_now`): what tells
    /// an initialiser's own values from the ones that existed before it began.
    pub(super) made: Cell<u32>,
}

impl Drop for Object {
    fn drop(&mut self) {
        if self.slot.get() != 0 {
            super::registry::vacate(self.slot.get());
        }
    }
}

/// The cell of an array, registered when it is made in a resident (`registry.rs`).
pub struct ArrayCell {
    items: RefCell<Vec<Value>>,
    /// The identity hash, 0 until first asked (`Object::hash`).
    pub hash: Cell<i32>,
    pub(super) slot: Cell<u32>,
    /// The epoch that stamped the array (`Object::born`).
    pub(super) born: Cell<u32>,
    /// When the array was made (`Object::made`).
    pub(super) made: Cell<u32>,
}

impl ArrayCell {
    pub fn new(items: Vec<Value>) -> Rc<ArrayCell> {
        let a = Rc::new(ArrayCell { items: RefCell::new(items), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
        if super::registry::on() {
            super::registry::array(&a);
        }
        a
    }
}

impl ArrayCell {
    /// The items for a change: a change of an array another epoch stamped is state the runs
    /// share (`super::written`).
    #[inline]
    pub fn write(self: &Rc<Self>) -> std::cell::RefMut<'_, Vec<Value>> {
        let born = self.born.get();
        super::written(born, self.made.get(), || Some(super::Node::Value(Value::Array(self.clone()))), |_| format!("an array{}", super::made_by(born)));
        self.items.borrow_mut()
    }
}

impl std::ops::Deref for ArrayCell {
    type Target = RefCell<Vec<Value>>;
    fn deref(&self) -> &RefCell<Vec<Value>> {
        &self.items
    }
}

impl Drop for ArrayCell {
    fn drop(&mut self) {
        if self.slot.get() != 0 {
            super::registry::vacate(self.slot.get());
        }
    }
}

/// The cell of a `RawMap`, registered as an array's is.
pub struct MapCell {
    map: RefCell<HMap>,
    /// The identity hash, 0 until first asked (`Object::hash`).
    pub hash: Cell<i32>,
    pub(super) slot: Cell<u32>,
    /// The epoch that stamped the map (`Object::born`).
    pub(super) born: Cell<u32>,
    /// When the map was made (`Object::made`).
    pub(super) made: Cell<u32>,
}

impl MapCell {
    pub fn new(map: HMap) -> Rc<MapCell> {
        let m = Rc::new(MapCell { map: RefCell::new(map), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) });
        if super::registry::on() {
            super::registry::map(&m);
        }
        m
    }
}

impl MapCell {
    /// The map for a change (`ArrayCell::write`).
    #[inline]
    pub fn write(self: &Rc<Self>) -> std::cell::RefMut<'_, HMap> {
        let born = self.born.get();
        super::written(born, self.made.get(), || Some(super::Node::Value(Value::Map(self.clone()))), |_| format!("a map{}", super::made_by(born)));
        self.map.borrow_mut()
    }
}

impl std::ops::Deref for MapCell {
    type Target = RefCell<HMap>;
    fn deref(&self) -> &RefCell<HMap> {
        &self.map
    }
}

impl Drop for MapCell {
    fn drop(&mut self) {
        if self.slot.get() != 0 {
            super::registry::vacate(self.slot.get());
        }
    }
}

pub struct Closure {
    pub kind: ClosureKind,
    pub env: Rc<Frame>,
    pub this: Value,
    /// The def the closure was made in, which a `return` inside it leaves.
    pub def_key: u64,
    pub class: Option<ClassId>,
    /// The identity hash, 0 until first asked (`Object::hash`).
    pub hash: Cell<i32>,
}

pub enum ClosureKind {
    /// Parameters live in `Program::sym_lists`.
    Lambda(crate::ast::ListRef, TExprId),
    LocalDef(FunId),
    /// A `{ case ... }` literal: `applyOrElse` and `isDefinedAt`.
    Partial(Value, Value),
    /// A local lazy val before its first read: the initialiser in its scope.
    Lazy(TExprId),
    /// The default `applyOrElse` gets from callers that tell "no case matched" from a result.
    Miss,
    /// The default that throws a `MatchError` for its argument.
    MatchError,
    /// A function the macro package supplies.
    #[allow(dead_code)]
    Builtin(super::Builtin),
}

/// `Class.toString`: the bare name of a primitive's class (`int`, `void`), `class ` before any
/// other's, as the JDK prints them.
pub fn class_text(qname: &str) -> String {
    match qname {
        "boolean" | "byte" | "short" | "char" | "int" | "long" | "float" | "double" | "void" => qname.to_string(),
        _ => format!("class {}", qname),
    }
}

pub struct ClassValue {
    pub class: Option<ClassId>,
    pub qname: Rc<str>,
}

pub struct MatchValue {
    /// Per group its UTF-16 indices in the source.
    pub groups: Vec<Option<(usize, usize)>>,
    pub names: Vec<(String, usize)>,
    pub source: Rc<str>,
    /// The byte offset of each UTF-16 index of the source (`regex::unit_chars`).
    pub bytes: Rc<[u32]>,
}

impl MatchValue {
    pub fn slice(&self, from: usize, to: usize) -> Cow<'_, str> {
        unit_slice(&self.source, &self.bytes, from, to)
    }
}

/// The UTF-16 units `from..to` of `s`, `bytes` giving the byte offset of each index, `u32::MAX`
/// inside a pair.
pub fn unit_slice<'s>(s: &'s str, bytes: &[u32], from: usize, to: usize) -> Cow<'s, str> {
    match (bytes[from], bytes[to]) {
        (a, b) if a != u32::MAX && b != u32::MAX => Cow::Borrowed(&s[a as usize..(b as usize).max(a as usize)]),
        _ => utf16_slice(s, from, to),
    }
}

/// An insertion-ordered map whose keys compare as `==`: the runtime's `$HMap`. Removed entries
/// leave a hole, so that the order of the rest stays.
#[derive(Default, Clone)]
pub struct HMap {
    pub entries: Vec<Option<(Value, Value)>>,
    pub index: crate::intern::FxMap<i32, Vec<u32>>,
    pub live: usize,
}

/// The kind of an expression the interpreter has not looked at.
pub const K_UNKNOWN: u8 = u8::MAX;
pub const K_INT: u8 = 1;
pub const K_LONG: u8 = 2;
pub const K_DOUBLE: u8 = 3;
pub const K_FLOAT: u8 = 4;
pub const K_BYTE: u8 = 5;
pub const K_SHORT: u8 = 6;
pub const K_CHAR: u8 = 7;

impl Value {
    /// Whether the text of the value is a function of the value alone: no `toString` of the
    /// program runs for it and nothing it holds can change.
    pub fn renders_by_value(&self) -> bool {
        matches!(
            self,
            Value::Unit
                | Value::Null
                | Value::Bool(_)
                | Value::Int(_)
                | Value::Long(_)
                | Value::Double(_)
                | Value::Float(_)
                | Value::Byte(_)
                | Value::Short(_)
                | Value::Char(_)
                | Value::Str(_)
        )
    }

    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn string(s: String) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn array(items: Vec<Value>) -> Value {
        Value::Array(ArrayCell::new(items))
    }

    pub fn map(map: HMap) -> Value {
        Value::Map(MapCell::new(map))
    }

    pub fn from_lit(v: LitVal, interner: &crate::intern::Interner) -> Value {
        match v {
            LitVal::Int(i) => Value::Int(i),
            LitVal::Long(l) => Value::Long(l),
            LitVal::Double(d) => Value::Double(f64::from_bits(d)),
            LitVal::Char(c) => Value::Char(c),
            LitVal::Bool(b) => Value::Bool(b),
            LitVal::Str(n) => Value::str(interner.get(n)),
        }
    }

    /// The literal a value stands for, when it is one; a Float, Byte or Short is the Double or
    /// Int of the same value, since literal types have no case for them.
    pub fn to_lit(&self, interner: &crate::intern::Interner) -> Option<LitVal> {
        Some(match self {
            Value::Int(i) => LitVal::Int(*i),
            Value::Byte(b) => LitVal::Int(*b as i32),
            Value::Short(s) => LitVal::Int(*s as i32),
            Value::Long(l) => LitVal::Long(*l),
            Value::Double(d) => LitVal::Double(d.to_bits()),
            Value::Float(f) => LitVal::Double((*f as f64).to_bits()),
            Value::Char(c) => LitVal::Char(*c),
            Value::Bool(b) => LitVal::Bool(*b),
            Value::Str(s) => LitVal::Str(interner.intern(s)),
            _ => return None,
        })
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Long(_) | Value::Double(_) | Value::Float(_) | Value::Byte(_) | Value::Short(_))
    }

    pub fn is_fractional(&self) -> bool {
        matches!(self, Value::Double(_) | Value::Float(_))
    }

    pub fn is_integral(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Long(_) | Value::Byte(_) | Value::Short(_))
    }

    /// A reference, as `AnyRef` tests: anything but a value type.
    pub fn is_ref(&self) -> bool {
        !matches!(
            self,
            Value::Unit | Value::Bool(_) | Value::Int(_) | Value::Long(_) | Value::Double(_) | Value::Float(_) | Value::Byte(_) | Value::Short(_) | Value::Char(_)
        )
    }

    pub fn as_f64(&self) -> Option<f64> {
        Some(match self {
            Value::Int(i) => *i as f64,
            Value::Long(l) => *l as f64,
            Value::Double(d) => *d,
            Value::Float(f) => *f as f64,
            Value::Byte(b) => *b as f64,
            Value::Short(s) => *s as f64,
            Value::Char(c) => *c as f64,
            _ => return None,
        })
    }

    pub fn as_i64(&self) -> Option<i64> {
        Some(match self {
            Value::Int(i) => *i as i64,
            Value::Long(l) => *l,
            Value::Byte(b) => *b as i64,
            Value::Short(s) => *s as i64,
            Value::Char(c) => *c as i64,
            Value::Double(d) => d2l(*d),
            Value::Float(f) => d2l(*f as f64),
            _ => return None,
        })
    }

    pub fn as_i32(&self) -> Option<i32> {
        Some(match self {
            Value::Int(i) => *i,
            Value::Long(l) => *l as i32,
            Value::Byte(b) => *b as i32,
            Value::Short(s) => *s as i32,
            Value::Char(c) => *c as i32,
            Value::Double(d) => d2i(*d),
            Value::Float(f) => d2i(*f as f64),
            _ => return None,
        })
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The numeric kind of a value, `0` for anything else.
    pub fn kind(&self) -> u8 {
        match self {
            Value::Int(_) => K_INT,
            Value::Long(_) => K_LONG,
            Value::Double(_) => K_DOUBLE,
            Value::Float(_) => K_FLOAT,
            Value::Byte(_) => K_BYTE,
            Value::Short(_) => K_SHORT,
            Value::Char(_) => K_CHAR,
            _ => 0,
        }
    }

    /// The value brought to a numeric kind, as the static type of an expression asks where the
    /// typer left the widening implicit. Never inlined: its one caller is `eval`'s `adapt`, where
    /// its body made every result of `eval` stored in pieces its callers cannot read back
    /// directly, whenever the two landed in one codegen unit.
    #[inline(never)]
    pub fn coerce(self, kind: u8) -> Value {
        if self.kind() == kind || self.kind() == 0 {
            return self;
        }
        match kind {
            K_INT => Value::Int(self.as_i32().unwrap()),
            K_LONG => Value::Long(self.as_i64().unwrap()),
            K_DOUBLE => Value::Double(self.as_f64().unwrap()),
            K_FLOAT => Value::Float(self.as_f64().unwrap() as f32),
            K_BYTE => Value::Byte(self.as_i32().unwrap() as i8),
            K_SHORT => Value::Short(self.as_i32().unwrap() as i16),
            K_CHAR => Value::Char(self.as_i32().unwrap() as u16),
            _ => self,
        }
    }

    pub fn same(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Unit, Value::Unit) | (Value::Null, Value::Null) | (Value::Absent, Value::Absent) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Obj(a), Value::Obj(b)) => Rc::ptr_eq(a, b),
            (Value::Fun(a), Value::Fun(b)) => Rc::ptr_eq(a, b),
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b),
            (Value::Trie(a), Value::Trie(b)) => Rc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => a.qname == b.qname,
            (Value::Match(a), Value::Match(b)) => Rc::ptr_eq(a, b),
            (Value::Tree(a), Value::Tree(b)) => a == b,
            (Value::Type(a), Value::Type(b)) => {
                #[cfg(debug_assertions)]
                for t in [*a, *b] {
                    crate::types::view::compared("Value::Type's equality", t);
                }
                a == b
            }
            (Value::Sym(a), Value::Sym(b)) => a == b,
            (Value::Pos(f, s, e), Value::Pos(g, t, u)) => f == g && s == t && e == u,
            (Value::Src(a), Value::Src(b)) => a == b,
            (a, b) if a.is_number() && b.is_number() => numbers_equal(a, b),
            _ => false,
        }
    }
}

/// Boxed numbers compare by value across their classes, as `BoxesRunTime.equals` has it.
pub fn numbers_equal(a: &Value, b: &Value) -> bool {
    if a.is_fractional() || b.is_fractional() {
        a.as_f64() == b.as_f64()
    } else {
        a.as_i64() == b.as_i64()
    }
}

pub fn d2i(d: f64) -> i32 {
    if d.is_nan() {
        0
    } else if d >= 2147483647.0 {
        i32::MAX
    } else if d <= -2147483648.0 {
        i32::MIN
    } else {
        d as i32
    }
}

pub fn d2l(d: f64) -> i64 {
    if d.is_nan() {
        0
    } else if d >= 9223372036854775807.0 {
        i64::MAX
    } else if d <= -9223372036854775808.0 {
        i64::MIN
    } else {
        d as i64
    }
}

// ---- hashes ----

pub fn str_hash(s: &str) -> i32 {
    let mut h: i32 = 0;
    if s.is_ascii() {
        for &b in s.as_bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as i32);
        }
        return h;
    }
    for u in crate::text::utf16_units(s) {
        h = h.wrapping_mul(31).wrapping_add(u as i32);
    }
    h
}

pub fn long_hash_code(l: i64) -> i32 {
    (l ^ (l >> 32)) as i32
}

pub fn double_hash_code(d: f64) -> i32 {
    let bits = if d.is_nan() { 0x7ff8000000000000u64 } else { d.to_bits() };
    long_hash_code(bits as i64)
}

pub fn float_hash_code(f: f32) -> i32 {
    if f.is_nan() { 0x7fc00000 } else { f.to_bits() as i32 }
}

/// `Statics.longHash`: a Long in the range of Int hashes as that Int.
pub fn long_hash(l: i64) -> i32 {
    let i = l as i32;
    if i as i64 == l { i } else { long_hash_code(l) }
}

/// `Statics.doubleHash`: a whole Double hashes as the Int or Long it equals.
pub fn double_hash(d: f64) -> i32 {
    let i = d as i32;
    if i as f64 == d {
        return i;
    }
    let l = d as i64;
    if l as f64 == d {
        return long_hash_code(l);
    }
    let f = d as f32;
    if f as f64 == d {
        return float_hash_code(f);
    }
    double_hash_code(d)
}

pub fn float_hash(f: f32) -> i32 {
    let i = f as i32;
    if i as f32 == f { i } else { float_hash_code(f) }
}

/// The hash of a value that has no class of its own: `##` of the primitives.
pub fn prim_hash(v: &Value) -> Option<i32> {
    Some(match v {
        Value::Null | Value::Unit | Value::Absent => 0,
        Value::Bool(b) => if *b { 1231 } else { 1237 },
        Value::Int(i) => *i,
        Value::Byte(b) => *b as i32,
        Value::Short(s) => *s as i32,
        Value::Char(c) => *c as i32,
        Value::Long(l) => long_hash(*l),
        Value::Double(d) => double_hash(*d),
        Value::Float(f) => float_hash(*f),
        Value::Str(s) => str_hash(s),
        _ => return None,
    })
}

/// `x.hashCode` of a primitive, which differs from `##` for a Long outside the Int range and
/// for a Double that is no Int: the JVM's boxed classes hash their bits.
pub fn prim_hash_code(v: &Value) -> Option<i32> {
    Some(match v {
        Value::Long(l) => long_hash_code(*l),
        Value::Double(d) => double_hash_code(*d),
        Value::Float(f) => float_hash_code(*f),
        _ => return prim_hash(v),
    })
}

// ---- MurmurHash3 ----

pub fn mix_last(h: i32, k: i32) -> i32 {
    let mut k = k.wrapping_mul(0xcc9e2d51u32 as i32);
    k = k.rotate_left(15);
    k = k.wrapping_mul(0x1b873593);
    h ^ k
}

pub fn mix(h: i32, k: i32) -> i32 {
    let mut h = mix_last(h, k);
    h = h.rotate_left(13);
    h.wrapping_mul(5).wrapping_add(0xe6546b64u32 as i32)
}

pub fn avalanche(h: i32) -> i32 {
    let mut h = h;
    h ^= ((h as u32) >> 16) as i32;
    h = h.wrapping_mul(0x85ebca6bu32 as i32);
    h ^= ((h as u32) >> 13) as i32;
    h = h.wrapping_mul(0xc2b2ae35u32 as i32);
    h ^ (((h as u32) >> 16) as i32)
}

pub fn finalize_hash(h: i32, length: i32) -> i32 {
    avalanche(h ^ length)
}

pub const PRODUCT_SEED: i32 = 0xcafebabeu32 as i32;

pub fn unordered_hash(a: i32, b: i32, c: i32, n: i32, seed: i32) -> i32 {
    let mut h = mix(seed, a);
    h = mix(h, b);
    h = mix_last(h, c);
    finalize_hash(h, n)
}

// ---- number formatting as the JVM prints it ----

/// `Double.toString`: the shortest digits that read back as the value, plain between 1e-3 and
/// 1e7 and in scientific notation outside.
pub fn java_double(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    java_number(java_digits(d), d.abs())
}

/// The digits `Double.toString` prints, in Rust's `[-]d[.ddd]e[-]x`: the shortest that read back
/// and of those the closest to the value, as Rust's shortest are, but the even one of two equally
/// close (`2.0233473011568512E15`, Rust `..513`), and two digits where one would do and a second
/// is closer (`4.9E-324`, Rust `5e-324`). Only 16 digits or more are close enough together to tie,
/// and only a small subnormal is imprecise enough for the second digit.
pub fn java_digits(d: f64) -> String {
    let sci = format!("{:e}", d);
    if d.abs() < f64::MIN_POSITIVE && !sci.contains('.') {
        return format!("{:.1e}", d);
    }
    let n = mantissa_digits(&sci);
    if n < 16 {
        return sci;
    }
    let even = format!("{:.*e}", n - 1, d);
    if even != sci && even.parse::<f64>() == Ok(d) { even } else { sci }
}

/// `java_digits` for `Float.toString`, where 7 digits can tie.
fn java_digits_f32(f: f32) -> String {
    let sci = format!("{:e}", f);
    if f.abs() < f32::MIN_POSITIVE && !sci.contains('.') {
        return format!("{:.1e}", f);
    }
    let n = mantissa_digits(&sci);
    if n < 7 {
        return sci;
    }
    let even = format!("{:.*e}", n - 1, f);
    if even != sci && even.parse::<f32>() == Ok(f) { even } else { sci }
}

fn mantissa_digits(sci: &str) -> usize {
    sci.bytes().take_while(|&b| b != b'e').filter(u8::is_ascii_digit).count()
}

pub fn java_float(f: f32) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    java_number(java_digits_f32(f), f.abs() as f64)
}

/// `sci` is `[-]d[.ddd]e[-]x` with the shortest digits, as Rust formats it.
fn java_number(sci: String, abs: f64) -> String {
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if (1e-3..1e7).contains(&abs) {
        // The decimal point stands after `exp + 1` digits.
        let point = exp + 1;
        if point <= 0 {
            out.push_str("0.");
            for _ in 0..-point {
                out.push('0');
            }
            out.push_str(&digits);
        } else if point as usize >= digits.len() {
            out.push_str(&digits);
            for _ in digits.len()..point as usize {
                out.push('0');
            }
            out.push_str(".0");
        } else {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        }
    } else {
        out.push_str(&digits[..1]);
        out.push('.');
        if digits.len() > 1 {
            out.push_str(&digits[1..]);
        } else {
            out.push('0');
        }
        out.push('E');
        out.push_str(&exp.to_string());
    }
    out
}

// ---- UTF-16 views of strings ----
//
// A `Str` is in `crate::text`'s canonical form: an index counts UTF-16 units, a supplementary
// character two, a lone surrogate's stand-in one; a slice that cuts a pair gives the half it keeps
// as its stand-in.

// ---- the index of long strings ----
//
// A view by UTF-16 index scans its string from the start (and `is_ascii` the whole of it), so that
// a loop of `charAt` over a megabyte is quadratic. The last few long strings a builtin was handed
// (`index_str`) are indexed instead: whether each is ASCII, and for one that is not the character
// holding every `MARK`th unit, found as far as an index has reached, and the last character an
// index found, from which a forward scan goes on. The table holds each string, so that no other
// takes its address while it is listed: a `&str` of a listed string's address and length is it.

/// The bytes from which a string is worth indexing.
const INDEXED_MIN: usize = 512;
/// The strings listed at once.
const INDEXED: usize = 4;
/// The units between two marks.
const MARK: usize = 64;

struct Indexed {
    s: Rc<str>,
    ascii: bool,
    /// For a string that is not ASCII, the first unit and the byte offset of the character
    /// holding unit `k * MARK`, for each `k` the scan has reached.
    marks: Vec<(usize, usize)>,
    /// Where the scan that writes the marks stands: a character's first unit and byte offset.
    scan: (usize, usize),
    /// The last character found, as a mark.
    last: (usize, usize),
}

thread_local! {
    static INDEX: RefCell<Vec<Indexed>> = const { RefCell::new(Vec::new()) };
}

/// Lists a long string first in the table, indexed when it is not yet; a short one, which every
/// string builtin is handed, costs the length's test alone.
#[inline]
pub fn index_str(s: &Rc<str>) {
    if s.len() >= INDEXED_MIN {
        list_str(s);
    }
}

#[inline(never)]
fn list_str(s: &Rc<str>) {
    INDEX.with(|t| {
        let mut t = t.borrow_mut();
        match t.iter().position(|x| Rc::ptr_eq(&x.s, s)) {
            Some(0) => {}
            Some(k) => {
                let e = t.remove(k);
                t.insert(0, e);
            }
            None => {
                t.truncate(INDEXED - 1);
                t.insert(0, Indexed { s: s.clone(), ascii: s.is_ascii(), marks: Vec::new(), scan: (0, 0), last: (0, 0) });
            }
        }
    });
}

/// `f` of the listed string `s` is, `None` when it is not listed.
fn indexed<T>(s: &str, f: impl FnOnce(&mut Indexed) -> T) -> Option<T> {
    if s.len() < INDEXED_MIN {
        return None;
    }
    INDEX.with(|t| {
        let mut t = t.borrow_mut();
        let e = t.iter_mut().find(|x| x.s.as_ptr() == s.as_ptr() && x.s.len() == s.len())?;
        Some(f(e))
    })
}

impl Indexed {
    /// Moves the scan on by a character, marking the units it holds.
    fn step(&mut self) -> bool {
        let (u, b) = self.scan;
        let Some(c) = self.s[b..].chars().next() else { return false };
        let n = crate::text::len_utf16(c);
        while self.marks.len() * MARK < u + n {
            self.marks.push((u, b));
        }
        self.scan = (u + n, b + c.len_utf8());
        true
    }

    /// The first unit, the byte offset and the character of the character holding unit `i` of a
    /// string that is not ASCII, `None` past its end.
    fn find(&mut self, i: usize) -> Option<(usize, usize, char)> {
        let k = i / MARK;
        while self.marks.len() <= k && self.step() {}
        let mark = *self.marks.get(k)?;
        let (mut u, mut b) = if self.last.0 <= i && self.last.0 >= mark.0 { self.last } else { mark };
        loop {
            let c = self.s[b..].chars().next()?;
            let n = crate::text::len_utf16(c);
            if u + n > i {
                self.last = (u, b);
                return Some((u, b, c));
            }
            u += n;
            b += c.len_utf8();
        }
    }

    fn units(&mut self) -> usize {
        if self.ascii {
            return self.s.len();
        }
        while self.step() {}
        self.scan.0
    }

    fn unit_at(&mut self, i: usize) -> Option<u16> {
        if self.ascii {
            return self.s.as_bytes().get(i).map(|&b| b as u16);
        }
        let (u, _, c) = self.find(i)?;
        let mut buf = [0u8; 4];
        crate::text::utf16_units(c.encode_utf8(&mut buf)).nth(i - u)
    }

    /// `utf16_position`'s answer.
    fn position(&mut self, i: usize) -> (usize, Option<char>) {
        if self.ascii {
            return (i.min(self.s.len()), None);
        }
        match self.find(i) {
            Some((u, b, _)) if u == i => (b, None),
            Some((_, b, c)) => (b, Some(c)),
            None => (self.s.len(), None),
        }
    }

    /// The units before the byte offset `at`, a character's start.
    fn units_before(&mut self, at: usize) -> usize {
        if self.ascii {
            return at;
        }
        while self.scan.1 < at && self.step() {}
        let k = self.marks.partition_point(|&(_, b)| b <= at);
        let (mut u, mut b) = if k == 0 { (0, 0) } else { self.marks[k - 1] };
        while b < at {
            let Some(c) = self.s[b..].chars().next() else { break };
            u += crate::text::len_utf16(c);
            b += c.len_utf8();
        }
        u
    }
}

/// Whether `s` is ASCII, from the index when it is listed.
fn ascii(s: &str) -> bool {
    indexed(s, |x| x.ascii).unwrap_or_else(|| s.is_ascii())
}

pub fn utf16_len(s: &str) -> usize {
    if let Some(n) = indexed(s, Indexed::units) {
        return n;
    }
    if s.is_ascii() { s.len() } else { s.chars().map(crate::text::len_utf16).sum() }
}

/// The UTF-16 length of `s[..at]`, `at` a character's start.
fn utf16_len_before(s: &str, at: usize) -> usize {
    if let Some(n) = indexed(s, |x| x.units_before(at)) {
        return n;
    }
    utf16_len(&s[..at])
}

/// The byte offset of the UTF-16 index `i`, clamped to the string; an index inside a pair is the
/// character after it.
pub fn utf16_offset(s: &str, i: usize) -> usize {
    if let Some(at) = indexed(s, |x| match x.position(i) {
        (b, Some(c)) => b + c.len_utf8(),
        (b, None) => b,
    }) {
        return at;
    }
    if s.is_ascii() {
        return i.min(s.len());
    }
    let mut units = 0;
    for (at, c) in s.char_indices() {
        if units >= i {
            return at;
        }
        units += crate::text::len_utf16(c);
    }
    s.len()
}

/// The byte offset of the character that holds the UTF-16 index `i`, clamped to the string, and
/// the pair it stands inside of when it is a pair's second unit.
fn utf16_position(s: &str, i: usize) -> (usize, Option<char>) {
    if let Some(p) = indexed(s, |x| x.position(i)) {
        return p;
    }
    let mut units = 0;
    for (at, c) in s.char_indices() {
        if units == i {
            return (at, None);
        }
        let n = crate::text::len_utf16(c);
        if units + n > i {
            return (at, Some(c));
        }
        units += n;
    }
    (s.len(), None)
}

/// The units `from..to`, the stand-in of a pair's half at a cut end.
pub fn utf16_slice(s: &str, from: usize, to: usize) -> Cow<'_, str> {
    if ascii(s) {
        let (a, b) = (from.min(s.len()), to.min(s.len()));
        return Cow::Borrowed(if a >= b { "" } else { &s[a..b] });
    }
    if from >= to {
        return Cow::Borrowed("");
    }
    let (a, cut_start) = utf16_position(s, from);
    let (b, cut_end) = utf16_position(s, to);
    if cut_start.is_none() && cut_end.is_none() {
        return Cow::Borrowed(&s[a..b.max(a)]);
    }
    let mut units = [0u16; 2];
    let mut out = String::new();
    let start = match cut_start {
        Some(c) => {
            c.encode_utf16(&mut units);
            out.push(crate::text::unit_char(units[1]));
            a + c.len_utf8()
        }
        None => a,
    };
    out.push_str(&s[start..b.max(start)]);
    if let Some(c) = cut_end {
        c.encode_utf16(&mut units);
        out.push(crate::text::unit_char(units[0]));
    }
    Cow::Owned(out)
}

pub fn char_at(s: &str, i: usize) -> Option<u16> {
    if let Some(u) = indexed(s, |x| x.unit_at(i)) {
        return u;
    }
    if s.is_ascii() {
        return s.as_bytes().get(i).map(|&b| b as u16);
    }
    crate::text::utf16_units(s).nth(i)
}

pub fn char_to_string(c: u16) -> String {
    crate::text::unit_char(c).to_string()
}

/// A needle whose ends can match half of a pair (a low stand-in first, a high one last), which a
/// search of the bytes cannot see.
pub fn cuts_pairs(part: &str) -> bool {
    crate::text::starts_with_low(part) || crate::text::ends_with_high(part)
}

fn units_of(s: &str) -> Vec<u16> {
    crate::text::utf16_units(s).collect()
}

/// The first UTF-16 index at or after `from` where `part`'s units stand in `s`'s.
fn find_units(s: &[u16], part: &[u16], from: usize) -> Option<usize> {
    (from..=s.len().checked_sub(part.len())?).find(|&i| s[i..].starts_with(part))
}

/// The UTF-16 index of `part` in `s` from `from` on, `-1` when absent.
pub fn index_of(s: &str, part: &str, from: usize) -> i32 {
    if cuts_pairs(part) {
        return find_units(&units_of(s), &units_of(part), from).map_or(-1, |i| i as i32);
    }
    let start = utf16_offset(s, from);
    match s[start..].find(part) {
        Some(at) => utf16_len_before(s, start + at) as i32,
        None => -1,
    }
}

pub fn last_index_of(s: &str, part: &str) -> i32 {
    if cuts_pairs(part) {
        return last_index_of_from(s, part, i32::MAX);
    }
    match s.rfind(part) {
        Some(at) => utf16_len_before(s, at) as i32,
        None => -1,
    }
}

/// Java's `lastIndexOf(part, from)`: the last match starting at a UTF-16 index at most `from`.
pub fn last_index_of_from(s: &str, part: &str, from: i32) -> i32 {
    if from < 0 {
        return -1;
    }
    if cuts_pairs(part) {
        let (s, part) = (units_of(s), units_of(part));
        let Some(last) = s.len().checked_sub(part.len()) else { return -1 };
        return (0..=last.min(from as usize)).rev().find(|&i| s[i..].starts_with(&part)).map_or(-1, |i| i as i32);
    }
    let end = utf16_offset(s, (from as usize).min(utf16_len(s)));
    match (0..=end).rev().find(|&at| s.is_char_boundary(at) && s[at..].starts_with(part)) {
        Some(at) => utf16_len_before(s, at) as i32,
        None => -1,
    }
}

pub fn starts_with(s: &str, part: &str) -> bool {
    if cuts_pairs(part) { units_of(s).starts_with(&units_of(part)) } else { s.starts_with(part) }
}

pub fn ends_with(s: &str, part: &str) -> bool {
    if cuts_pairs(part) { units_of(s).ends_with(&units_of(part)) } else { s.ends_with(part) }
}

/// Java's `String.replace` of a string: every match from the left, over the units.
pub fn replace(s: &str, target: &str, repl: &str) -> String {
    let has_pairs = || s.chars().any(|c| c.len_utf16() == 2 && crate::text::surrogate_of(c).is_none());
    if !cuts_pairs(target) && !(target.is_empty() && has_pairs()) {
        let out = if target.is_empty() {
            let mut out = String::with_capacity(s.len() + repl.len() * (s.len() + 1));
            for c in s.chars() {
                crate::text::push_str(&mut out, repl);
                out.push(c);
            }
            crate::text::push_str(&mut out, repl);
            out
        } else {
            s.replace(target, repl)
        };
        return crate::text::canonical(out);
    }
    let (s, target, repl) = (units_of(s), units_of(target), units_of(repl));
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    loop {
        if target.is_empty() {
            out.extend_from_slice(&repl);
            match s.get(i) {
                Some(&u) => out.push(u),
                None => break,
            }
            i += 1;
            continue;
        }
        match find_units(&s, &target, i) {
            Some(at) => {
                out.extend_from_slice(&s[i..at]);
                out.extend_from_slice(&repl);
                i = at + target.len();
            }
            None => {
                out.extend_from_slice(&s[i..]);
                break;
            }
        }
    }
    crate::text::from_units(out)
}

/// `s` `n` times, a high stand-in at its end paired with a low one at its start.
pub fn repeat(s: &str, n: usize) -> String {
    let out = s.repeat(n);
    if n > 1 && crate::text::starts_with_low(s) && crate::text::ends_with_high(s) { crate::text::canonical(out) } else { out }
}

/// `StringBuilder.reverse` of Java: the units reversed, a pair kept in its order.
pub fn reverse(s: &str) -> String {
    if s.is_ascii() {
        return s.chars().rev().collect();
    }
    let mut units = units_of(s);
    units.reverse();
    let mut i = 0;
    while i + 1 < units.len() {
        if (0xDC00..0xE000).contains(&units[i]) && (0xD800..0xDC00).contains(&units[i + 1]) {
            units.swap(i, i + 1);
            i += 1;
        }
        i += 1;
    }
    crate::text::from_units(units)
}

pub fn compare_strings(a: &str, b: &str) -> i32 {
    if a.is_ascii() && b.is_ascii() {
        return match a.bytes().zip(b.bytes()).find(|(p, q)| p != q) {
            Some((p, q)) => p as i32 - q as i32,
            None => a.len() as i32 - b.len() as i32,
        };
    }
    let mut x = crate::text::utf16_units(a);
    let mut y = crate::text::utf16_units(b);
    loop {
        match (x.next(), y.next()) {
            (Some(p), Some(q)) if p == q => continue,
            (Some(p), Some(q)) => return p as i32 - q as i32,
            (None, None) => return 0,
            (Some(_), None) => return utf16_len(a) as i32 - utf16_len(b) as i32,
            (None, Some(_)) => return utf16_len(a) as i32 - utf16_len(b) as i32,
        }
    }
}

pub fn compare_doubles(a: f64, b: f64) -> i32 {
    if a < b {
        -1
    } else if a > b {
        1
    } else if a == b {
        if a != 0.0 {
            0
        } else {
            match (a.is_sign_negative(), b.is_sign_negative()) {
                (x, y) if x == y => 0,
                (true, false) => -1,
                _ => 1,
            }
        }
    } else if a.is_nan() {
        if b.is_nan() { 0 } else { 1 }
    } else {
        -1
    }
}

/// `Math.rint`: the nearest whole number, halves to the even one.
pub fn rint(x: f64) -> f64 {
    let r = x.round();
    if (r - x).abs() == 0.5 && r % 2.0 != 0.0 { r - x.signum() } else { r }
}

/// `Math.round(double)`: the JVM rounds halves up, `Math.floor(x + 0.5)` with the exact cases.
pub fn java_round(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    d2l((x + 0.5).floor())
}

/// The native form of a `TrieNode`: a CHAMP node whose data pairs (key, ordinal) stand in front
/// of the content in the order of their bits and whose sub-nodes stand at its end in the
/// reverse order, as `std/maps.scala` lays it out.
pub struct Trie {
    pub data_map: i32,
    pub node_map: i32,
    pub content: Vec<Value>,
    /// The identity hash, 0 until first asked (`Object::hash`).
    pub hash: Cell<i32>,
}

impl Trie {
    pub fn new(data_map: i32, node_map: i32, content: Vec<Value>) -> Trie {
        Trie { data_map, node_map, content, hash: Cell::new(0) }
    }
}

/// A copy is another node (`Rc::make_mut` of a shared one), with no identity hash yet.
impl Clone for Trie {
    fn clone(&self) -> Trie {
        Trie::new(self.data_map, self.node_map, self.content.clone())
    }
}
