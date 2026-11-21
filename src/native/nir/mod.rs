//! Scala Native's intermediate representation, NIR 6.11: what its compiler plugin writes for each
//! top-level class into a library's jar, and what a native link reads. The model follows
//! `scala.scalanative.nir` of Scala Native 0.5.12 (see `NOTICE`); names, signatures and types
//! are interned in a `Cx` shared by every file of a link, so that equality is an integer's.

pub mod cli;
pub mod read;
pub mod show;

use crate::intern::FxMap;
use std::sync::Arc;

/// An interned string. NIR's strings are UTF-16 code units; one is kept as UTF-8, an unpaired
/// surrogate as its three-byte generalized form, so that a string constant survives whole.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Str(pub u32);

/// An interned symbol: a class, trait or module (`GlobalData::Top`) or a member of one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Global(pub u32);

impl Global {
    pub const NONE: Global = Global(0);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GlobalData {
    None,
    Top(Str),
    /// The owner, always a `Top`, and the member's mangled signature.
    Member(Global, Str),
}

/// An interned type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Ty(pub u32);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Type {
    Vararg,
    Ptr,
    Bool,
    Char,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Size,
    Int128,
    ArrayValue(Ty, i32),
    StructValue(Box<[Ty]>),
    Function(Box<[Ty]>, Ty),
    Null,
    Nothing,
    Virtual,
    Var(Ty),
    Unit,
    Array { of: Ty, nullable: bool },
    Ref { name: Global, exact: bool, nullable: bool },
}

/// An SSA identifier, unique within a definition.
pub type Local = u64;

/// A value. A file names one by its place in the values' section, and every reference to one place
/// is the one value (`BinaryDeserializer`'s cache): an aggregate holds its parts behind an `Arc`, so
/// that what a file's references share stays shared and a value read is no larger than its file.
#[derive(Clone, PartialEq, Debug)]
pub enum Val {
    True,
    False,
    Null,
    Zero(Ty),
    Size(i64),
    Char(u16),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Int128 { hi: i64, lo: i64 },
    /// The bits: two values are the same when their bits are.
    Float(u32),
    Double(u64),
    StructValue(Arc<[Val]>),
    ArrayValue(Ty, Arc<[Val]>),
    /// A C string's bytes, without the terminating zero it is emitted with.
    ByteString(Arc<[u8]>),
    Local(Local, Ty),
    Global(Global, Ty),
    Unit,
    Const(Arc<Val>),
    String(Str),
    Virtual(i64),
    ClassOf(Global),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bin {
    Iadd,
    Fadd,
    Isub,
    Fsub,
    Imul,
    Fmul,
    Sdiv,
    Udiv,
    Fdiv,
    Srem,
    Urem,
    Frem,
    Shl,
    Lshr,
    Ashr,
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Comp {
    Ieq,
    Ine,
    Ugt,
    Uge,
    Ult,
    Ule,
    Sgt,
    Sge,
    Slt,
    Sle,
    Feq,
    Fne,
    Fgt,
    Fge,
    Flt,
    Fle,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Conv {
    Trunc,
    Zext,
    Sext,
    Fptrunc,
    Fpext,
    Fptoui,
    Fptosi,
    Uitofp,
    Sitofp,
    Ptrtoint,
    Inttoptr,
    Bitcast,
    SSizeCast,
    ZSizeCast,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MemoryOrder {
    Unordered,
    Monotonic,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Op {
    Call { ty: Ty, ptr: Val, args: Box<[Val]> },
    Load { ty: Ty, ptr: Val, order: Option<MemoryOrder> },
    Store { ty: Ty, ptr: Val, value: Val, order: Option<MemoryOrder> },
    Elem { ty: Ty, ptr: Val, indexes: Box<[Val]> },
    Extract { aggr: Val, indexes: Box<[i32]> },
    Insert { aggr: Val, value: Val, indexes: Box<[i32]> },
    Stackalloc { ty: Ty, n: Val },
    Bin { bin: Bin, ty: Ty, l: Val, r: Val },
    Comp { comp: Comp, ty: Ty, l: Val, r: Val },
    Conv { conv: Conv, ty: Ty, value: Val },
    Fence(MemoryOrder),
    Classalloc { name: Global, zone: Option<Val> },
    Fieldload { ty: Ty, obj: Val, name: Global },
    Fieldstore { ty: Ty, obj: Val, name: Global, value: Val },
    Field { obj: Val, name: Global },
    Method { obj: Val, sig: Str },
    Dynmethod { obj: Val, sig: Str },
    Module(Global),
    As { ty: Ty, obj: Val },
    Is { ty: Ty, obj: Val },
    Copy(Val),
    SizeOf(Ty),
    AlignmentOf(Ty),
    Box { ty: Ty, obj: Val },
    Unbox { ty: Ty, obj: Val },
    Var(Ty),
    Varload(Val),
    Varstore { slot: Val, value: Val },
    Arrayalloc { ty: Ty, init: Val, zone: Option<Val> },
    Arrayload { ty: Ty, arr: Val, idx: Val },
    Arraystore { ty: Ty, arr: Val, idx: Val, value: Val },
    Arraylength(Val),
}

/// Where control goes: a block with its arguments, a handler for what a call throws, or a
/// switch's case.
#[derive(Clone, PartialEq, Debug)]
pub enum Next {
    None,
    Unwind { exc: Local, ty: Ty, next: Box<Next> },
    Case(Val, Box<Next>),
    Label(Local, Box<[Val]>),
}

/// A position in a source file, zero-based; the file is the empty string where there is none.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pos {
    pub file: Str,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum LinktimeCondition {
    Simple { property: Str, comparison: Comp, value: Val, pos: Pos },
    Complex { op: Bin, left: Box<LinktimeCondition>, right: Box<LinktimeCondition>, pos: Pos },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Inst {
    pub pos: Pos,
    pub kind: InstKind,
}

#[derive(Clone, PartialEq, Debug)]
pub enum InstKind {
    Label { id: Local, params: Box<[(Local, Ty)]> },
    /// `scope` is the lexical scope of the definition's debug information, 0 the outermost.
    Let { id: Local, op: Op, unwind: Next, scope: u32 },
    Ret(Val),
    Jump(Next),
    If { value: Val, then: Next, otherwise: Next },
    Switch { value: Val, default: Next, cases: Box<[Next]> },
    Throw { value: Val, unwind: Next },
    Unreachable { unwind: Next },
    LinktimeIf { cond: LinktimeCondition, then: Next, otherwise: Next },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Inline {
    #[default]
    May,
    Hint,
    No,
    Always,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Opt {
    #[default]
    Un,
    No,
    Did,
    Bail(Str),
}

/// A definition's attributes, as `nir.Attrs` folds the file's sequence: the last of a kind wins,
/// the libraries to link and the preprocessor definitions accumulate.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Attrs {
    pub inline: Inline,
    pub no_specialize: bool,
    pub opt: Opt,
    pub align: Option<(i32, Option<Str>)>,
    pub is_extern: bool,
    pub is_blocking: bool,
    pub is_dyn: bool,
    pub is_stub: bool,
    pub is_abstract: bool,
    pub is_volatile: bool,
    pub is_final: bool,
    pub is_safe_publish: bool,
    pub is_linktime_resolved: bool,
    pub uses_intrinsic: bool,
    pub link_cpp_runtime: bool,
    pub links: Vec<Str>,
    pub defines: Vec<Str>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LexicalScope {
    pub id: u32,
    pub parent: u32,
    pub pos: Pos,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct DebugInfo {
    /// The source names of the locals that have one, by local.
    pub local_names: Vec<(Local, Str)>,
    pub scopes: Vec<LexicalScope>,
}

impl DebugInfo {
    pub fn local_name(&self, local: Local) -> Option<Str> {
        self.local_names.binary_search_by_key(&local, |&(l, _)| l).ok().map(|i| self.local_names[i].1)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Defn {
    pub attrs: Attrs,
    pub name: Global,
    pub pos: Pos,
    pub kind: DefnKind,
}

#[derive(Clone, PartialEq, Debug)]
pub enum DefnKind {
    Var { ty: Ty, rhs: Val },
    Const { ty: Ty, rhs: Val },
    Declare { ty: Ty },
    Define { ty: Ty, insts: Vec<Inst>, debug: DebugInfo },
    Trait { traits: Box<[Global]> },
    Class { parent: Option<Global>, traits: Box<[Global]> },
    Module { parent: Option<Global>, traits: Box<[Global]> },
}

/// The strings, symbols and types of a link.
pub struct Cx {
    strings: Vec<Arc<[u8]>>,
    string_ids: FxMap<Arc<[u8]>, Str>,
    globals: Vec<GlobalData>,
    global_ids: FxMap<GlobalData, Global>,
    types: Vec<Type>,
    type_ids: FxMap<Type, Ty>,
}

impl Default for Cx {
    fn default() -> Self {
        Self::new()
    }
}

impl Cx {
    pub fn new() -> Cx {
        let mut cx = Cx {
            strings: Vec::new(),
            string_ids: FxMap::default(),
            globals: vec![GlobalData::None],
            global_ids: FxMap::default(),
            types: Vec::new(),
            type_ids: FxMap::default(),
        };
        cx.global_ids.insert(GlobalData::None, Global::NONE);
        let empty = cx.str(b"");
        debug_assert_eq!(empty, Str::EMPTY);
        cx
    }

    /// `bytes` is UTF-8, an unpaired surrogate in its generalized form.
    pub fn str(&mut self, bytes: &[u8]) -> Str {
        if let Some(&id) = self.string_ids.get(bytes) {
            return id;
        }
        let id = Str(self.strings.len() as u32);
        let held: Arc<[u8]> = Arc::from(bytes);
        self.strings.push(held.clone());
        self.string_ids.insert(held, id);
        id
    }

    pub fn bytes(&self, s: Str) -> &[u8] {
        &self.strings[s.0 as usize]
    }

    pub fn global(&mut self, data: GlobalData) -> Global {
        if let Some(&id) = self.global_ids.get(&data) {
            return id;
        }
        let id = Global(self.globals.len() as u32);
        self.globals.push(data);
        self.global_ids.insert(data, id);
        id
    }

    pub fn global_data(&self, g: Global) -> GlobalData {
        self.globals[g.0 as usize]
    }

    pub fn ty(&mut self, ty: Type) -> Ty {
        if let Some(&id) = self.type_ids.get(&ty) {
            return id;
        }
        let id = Ty(self.types.len() as u32);
        self.types.push(ty.clone());
        self.type_ids.insert(ty, id);
        id
    }

    pub fn type_of(&self, ty: Ty) -> &Type {
        &self.types[ty.0 as usize]
    }

    /// A symbol's mangled name, the one it has in the linked program: `T` and the class's
    /// identifier, or `M`, the owner's identifier and the member's signature.
    pub fn mangle(&self, g: Global, out: &mut Vec<u8>) {
        match self.global_data(g) {
            GlobalData::None => {}
            GlobalData::Top(id) => {
                out.push(b'T');
                mangle_ident(self.bytes(id), out);
            }
            GlobalData::Member(owner, sig) => {
                out.push(b'M');
                if let GlobalData::Top(id) = self.global_data(owner) {
                    mangle_ident(self.bytes(id), out);
                }
                out.extend_from_slice(self.bytes(sig));
            }
        }
    }
}

impl Str {
    pub const EMPTY: Str = Str(0);
}

/// An identifier as the mangler writes it: its length in UTF-16 code units, a `-` before one
/// whose first unit is a `-` or a decimal digit of any script (`Character.isDigit`), and the
/// identifier.
fn mangle_ident(id: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(utf16_len(id).to_string().as_bytes());
    if utf16_units(id).next().is_some_and(|unit| unit == u16::from(b'-') || crate::text::decimal_digit(unit).is_some()) {
        out.push(b'-');
    }
    out.extend_from_slice(id);
}

/// The number of UTF-16 code units of a string kept as (generalized) UTF-8: one for each
/// character, two for one of four bytes.
pub fn utf16_len(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b & 0xc0 != 0x80).count() + bytes.iter().filter(|&&b| b >= 0xf0).count()
}

/// The UTF-16 code units of a string kept as (generalized) UTF-8: a character of four bytes as
/// its two surrogates, a surrogate of three bytes as itself.
pub fn utf16_units(bytes: &[u8]) -> impl Iterator<Item = u16> + '_ {
    let (mut at, mut low) = (0, None);
    std::iter::from_fn(move || {
        if let Some(unit) = low.take() {
            return Some(unit);
        }
        let lead = *bytes.get(at)? as u32;
        let more = |k: usize| bytes.get(at + k).map_or(0, |&b| (b & 0x3f) as u32);
        let (code, len) = match lead {
            0..=0x7f => (lead, 1),
            0x80..=0xdf => (((lead & 0x1f) << 6) | more(1), 2),
            0xe0..=0xef => (((lead & 0x0f) << 12) | (more(1) << 6) | more(2), 3),
            _ => (((lead & 0x07) << 18) | (more(1) << 12) | (more(2) << 6) | more(3), 4),
        };
        at += len;
        if code < 0x10000 {
            return Some(code as u16);
        }
        low = Some((0xdc00 + (code & 0x3ff)) as u16);
        Some((0xd800 + ((code - 0x10000) >> 10)) as u16)
    })
}

/// Java's order of strings (`String.compareTo`), by UTF-16 code units, of two kept as
/// (generalized) UTF-8.
pub fn java_order(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    utf16_units(a).cmp(utf16_units(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mangled(cx: &mut Cx, id: &str) -> String {
        let name = cx.str(id.as_bytes());
        let class = cx.global(GlobalData::Top(name));
        let mut out = Vec::new();
        cx.mangle(class, &mut out);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn marks_a_name_that_starts_with_a_digit_of_any_script() {
        let mut cx = Cx::new();
        // ASCII, Arabic-Indic, Devanagari, Thai, Myanmar, fullwidth: decimal digits, so a `-` before them.
        for digits in ["3x", "\u{663}x", "\u{966}x", "\u{e50}x", "\u{1049}x", "\u{ff10}y"] {
            assert_eq!(mangled(&mut cx, digits), format!("T{}-{}", digits.encode_utf16().count(), digits));
        }
        assert_eq!(mangled(&mut cx, "-z"), "T2--z");
        // A superscript and a circled number are numbers, no decimal digits; a digit beyond the
        // Basic Multilingual Plane starts with a surrogate, which `Character.isDigit` holds none.
        for other in ["\u{b2}x", "\u{2460}x", "x9", "\u{1d7ce}z"] {
            assert_eq!(mangled(&mut cx, other), format!("T{}{}", other.encode_utf16().count(), other));
        }
        // A surrogate on its own, kept in its three bytes.
        let lone = cx.str(b"\xed\xa0\x80x");
        let class = cx.global(GlobalData::Top(lone));
        let mut out = Vec::new();
        cx.mangle(class, &mut out);
        assert_eq!(out, b"T2\xed\xa0\x80x");
    }

    #[test]
    fn orders_names_as_java_orders_strings() {
        // U+FF21 comes before U+1D400 in UTF-8 and after its first surrogate, U+D835, in UTF-16.
        let (fullwidth, bold) = ("F2\u{ff21}\u{ff21}O", "F2\u{1d400}O");
        assert!(fullwidth.as_bytes() < bold.as_bytes());
        let mut names = vec![fullwidth, bold, "F2AAO"];
        names.sort_by(|a, b| java_order(a.as_bytes(), b.as_bytes()));
        assert_eq!(names, ["F2AAO", bold, fullwidth]);
    }
}
