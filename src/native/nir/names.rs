//! Names in their mangled spelling, after `nir.Mangle` and `nir.Unmangle` of Scala Native 0.5.12,
//! the order `Global.globalOrdering` gives them, and the runtime's names the link and the code
//! generator refer to (`nir.Rt`).

use super::*;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub enum Scope {
    Public,
    PublicStatic,
}

pub fn is_ctor(sig: &[u8]) -> bool {
    sig.first() == Some(&b'R')
}
pub fn is_clinit(sig: &[u8]) -> bool {
    sig.first() == Some(&b'I')
}
pub fn is_method(sig: &[u8]) -> bool {
    sig.first() == Some(&b'D')
}
pub fn is_extern(sig: &[u8]) -> bool {
    sig.first() == Some(&b'C')
}
pub fn is_generated(sig: &[u8]) -> bool {
    sig.first() == Some(&b'G')
}
pub fn is_virtual(sig: &[u8]) -> bool {
    !(is_ctor(sig) || is_clinit(sig) || is_extern(sig))
}

/// The place after an identifier at `pos`: its length in UTF-16 code units, a `-` before one that
/// starts with a digit or a `-`, and the identifier.
fn ident_end(s: &[u8], mut pos: usize) -> Option<(usize, usize)> {
    let mut len = 0usize;
    let digits = pos;
    while let Some(d @ b'0'..=b'9') = s.get(pos) {
        len = len.checked_mul(10)?.checked_add((d - b'0') as usize)?;
        pos += 1;
    }
    if pos == digits {
        return None;
    }
    if s.get(pos) == Some(&b'-') {
        pos += 1;
    }
    let start = pos;
    let mut units = 0;
    while units < len {
        let b = *s.get(pos)?;
        let width = match b {
            0..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        };
        units += if width == 4 { 2 } else { 1 };
        pos += width;
    }
    (pos <= s.len()).then_some((start, pos))
}

/// The place after the type at `pos`.
fn type_end(s: &[u8], pos: usize) -> Option<usize> {
    match *s.get(pos)? {
        b'v' | b'z' | b'c' | b'b' | b's' | b'i' | b'j' | b'w' | b'f' | b'd' | b'l' | b'n' | b'u' => Some(pos + 1),
        b'R' if s.get(pos + 1) == Some(&b'_') => Some(pos + 2),
        b'R' | b'S' => types_end(s, pos + 1),
        b'A' => {
            let mut at = type_end(s, pos + 1)?;
            while let Some(b'0'..=b'9') = s.get(at) {
                at += 1;
            }
            (s.get(at) == Some(&b'_')).then_some(at + 1)
        }
        b'L' => match *s.get(pos + 1)? {
            b'A' => {
                let at = type_end(s, pos + 2)?;
                (s.get(at) == Some(&b'_')).then_some(at + 1)
            }
            b'X' => ident_end(s, pos + 2).map(|(_, e)| e),
            _ => ident_end(s, pos + 1).map(|(_, e)| e),
        },
        b'X' => ident_end(s, pos + 1).map(|(_, e)| e),
        b'0'..=b'9' => ident_end(s, pos).map(|(_, e)| e),
        _ => None,
    }
}

/// The place after the `E` that ends the types from `pos`.
fn types_end(s: &[u8], mut pos: usize) -> Option<usize> {
    while *s.get(pos)? != b'E' {
        pos = type_end(s, pos)?;
    }
    Some(pos + 1)
}

/// A method's signature in its parts: the name, where its types start, where the last of them
/// (the result) starts, and the scope after the `E`.
pub struct MethodSig<'a> {
    pub id: &'a [u8],
    types_start: usize,
    last_type: usize,
    types_stop: usize,
    pub scope: &'a [u8],
}

pub fn method_sig(sig: &[u8]) -> Option<MethodSig<'_>> {
    if !is_method(sig) {
        return None;
    }
    let (start, end) = ident_end(sig, 1)?;
    let (mut pos, mut last) = (end, end);
    while *sig.get(pos)? != b'E' {
        last = pos;
        pos = type_end(sig, pos)?;
    }
    Some(MethodSig { id: &sig[start..end], types_start: end, last_type: last, types_stop: pos, scope: &sig[pos + 1..] })
}

impl MethodSig<'_> {
    pub fn is_static(&self) -> bool {
        matches!(self.scope.first(), Some(b'o' | b'p'))
    }
}

/// The name of a field, an extern or a generated signature.
pub fn sig_id(sig: &[u8]) -> Option<&[u8]> {
    match sig.first()? {
        b'F' | b'C' | b'G' | b'D' | b'P' => ident_end(sig, 1).map(|(s, e)| &sig[s..e]),
        _ => None,
    }
}

/// `java.lang.String.compareTo` in what decides an order: by UTF-16 code unit.
pub fn cmp_strings(a: &[u8], b: &[u8]) -> Ordering {
    // UTF-8's byte order is the code points' order, which is the code units' order but between
    // a character above U+FFFF and one from U+E000.
    if a.iter().chain(b).all(|&x| x < 0xee) {
        a.cmp(b)
    } else {
        java_order(a, b)
    }
}

impl Cx {
    pub fn top(&mut self, id: &str) -> Global {
        let id = self.str(id.as_bytes());
        self.global(GlobalData::Top(id))
    }

    /// The symbol a mangled name spells: `T` and a class, or `M`, a class and a signature.
    pub fn unmangle_global(&mut self, name: Str) -> Option<Global> {
        let bytes = self.bytes(name).to_vec();
        let (start, end) = ident_end(&bytes, 1)?;
        let id = self.str(&bytes[start..end]);
        let owner = self.global(GlobalData::Top(id));
        match bytes[0] {
            b'T' if end == bytes.len() => Some(owner),
            b'M' if end < bytes.len() => {
                let sig = self.str(&bytes[end..]);
                Some(self.member(owner, sig))
            }
            _ => None,
        }
    }

    pub fn member(&mut self, owner: Global, sig: Str) -> Global {
        self.global(GlobalData::Member(owner, sig))
    }

    /// The class a symbol belongs to, a class itself.
    pub fn owner(&self, g: Global) -> Global {
        match self.global_data(g) {
            GlobalData::Member(owner, _) => owner,
            _ => g,
        }
    }

    pub fn is_top(&self, g: Global) -> bool {
        matches!(self.global_data(g), GlobalData::Top(_))
    }

    pub fn sig(&self, g: Global) -> Option<Str> {
        match self.global_data(g) {
            GlobalData::Member(_, sig) => Some(sig),
            _ => None,
        }
    }

    pub fn sig_bytes(&self, g: Global) -> &[u8] {
        self.sig(g).map_or(&[], |s| self.bytes(s))
    }

    /// A class's name as written in the source of its library: `java.lang.Object`.
    pub fn id(&self, top: Global) -> &[u8] {
        match self.global_data(top) {
            GlobalData::Top(id) => self.bytes(id),
            _ => &[],
        }
    }

    pub fn class_ty(&mut self, name: Global) -> Ty {
        self.ty(Type::Ref { name, exact: false, nullable: true })
    }

    /// `Global.globalOrdering`: by the class's name, then by the member's signature.
    pub fn cmp_globals(&self, a: Global, b: Global) -> Ordering {
        let key = |g: Global| match self.global_data(g) {
            GlobalData::None => (&[][..], &[][..]),
            GlobalData::Top(id) => (self.bytes(id), &[][..]),
            GlobalData::Member(owner, sig) => (self.id(owner), self.bytes(sig)),
        };
        let (a, b) = (key(a), key(b));
        cmp_strings(a.0, b.0).then_with(|| cmp_strings(a.1, b.1))
    }

    pub fn mangle_type(&self, ty: Ty, out: &mut Vec<u8>) {
        match self.type_of(ty) {
            Type::Vararg => out.push(b'v'),
            Type::Ptr => out.extend_from_slice(b"R_"),
            Type::Bool => out.push(b'z'),
            Type::Char => out.push(b'c'),
            Type::Byte => out.push(b'b'),
            Type::Short => out.push(b's'),
            Type::Int => out.push(b'i'),
            Type::Long => out.push(b'j'),
            Type::Size => out.push(b'w'),
            Type::Float => out.push(b'f'),
            Type::Double => out.push(b'd'),
            Type::Null => out.push(b'l'),
            Type::Nothing => out.push(b'n'),
            Type::Unit => out.push(b'u'),
            Type::ArrayValue(of, n) => {
                out.push(b'A');
                self.mangle_type(*of, out);
                out.extend_from_slice(n.to_string().as_bytes());
                out.push(b'_');
            }
            Type::StructValue(tys) => {
                out.push(b'S');
                tys.iter().for_each(|&t| self.mangle_type(t, out));
                out.push(b'E');
            }
            Type::Function(args, ret) => {
                out.push(b'R');
                args.iter().for_each(|&t| self.mangle_type(t, out));
                self.mangle_type(*ret, out);
                out.push(b'E');
            }
            Type::Array { of, nullable } => {
                if *nullable {
                    out.push(b'L');
                }
                out.push(b'A');
                self.mangle_type(*of, out);
                out.push(b'_');
            }
            Type::Ref { name, exact, nullable } => {
                if *nullable {
                    out.push(b'L');
                }
                if *exact {
                    out.push(b'X');
                }
                mangle_ident(self.id(*name), out);
            }
            // The mangler has no spelling for these: no signature holds one.
            Type::Int128 | Type::Virtual | Type::Var(_) => out.push(b'?'),
        }
    }

    fn mangle_scope(&self, scope: Scope, out: &mut Vec<u8>) {
        match scope {
            Scope::Public => out.push(b'O'),
            Scope::PublicStatic => out.push(b'o'),
        }
    }

    /// `Sig.Method`: the last of `types` is the result's.
    pub fn method_sig(&mut self, id: &str, types: &[Ty], scope: Scope) -> Str {
        let mut out = vec![b'D'];
        mangle_ident(id.as_bytes(), &mut out);
        types.iter().for_each(|&t| self.mangle_type(t, &mut out));
        out.push(b'E');
        self.mangle_scope(scope, &mut out);
        self.str(&out)
    }

    pub fn ctor_sig(&mut self, types: &[Ty]) -> Str {
        let mut out = vec![b'R'];
        types.iter().for_each(|&t| self.mangle_type(t, &mut out));
        out.push(b'E');
        self.str(&out)
    }

    pub fn clinit_sig(&mut self) -> Str {
        self.str(b"IE")
    }

    pub fn field_sig(&mut self, id: &str, scope: Scope) -> Str {
        let mut out = vec![b'F'];
        mangle_ident(id.as_bytes(), &mut out);
        self.mangle_scope(scope, &mut out);
        self.str(&out)
    }

    pub fn extern_sig(&mut self, id: &str) -> Str {
        let mut out = vec![b'C'];
        mangle_ident(id.as_bytes(), &mut out);
        self.str(&out)
    }

    pub fn generated_sig(&mut self, id: &[u8]) -> Str {
        let mut out = vec![b'G'];
        mangle_ident(id, &mut out);
        self.str(&out)
    }

    /// `Sig.toProxy`: the signature a structural call names a method by, its name and its
    /// parameters' types without the result's.
    pub fn proxy_sig(&mut self, method: Str) -> Option<Str> {
        let bytes = self.bytes(method);
        let m = method_sig(bytes)?;
        let mut out = vec![b'P'];
        mangle_ident(m.id, &mut out);
        out.extend_from_slice(&bytes[m.types_start..m.last_type]);
        out.push(b'E');
        Some(self.str(&out))
    }

    /// The same method made public and of the instance: the form `java.util.ServiceLoader`'s
    /// module has its static methods in.
    pub fn public_sig(&mut self, method: Str) -> Str {
        let bytes = self.bytes(method);
        match method_sig(bytes) {
            Some(m) => {
                let mut out = bytes[..m.types_stop + 1].to_vec();
                out.push(b'O');
                self.str(&out)
            }
            None => method,
        }
    }
}

/// How many of the classes `CFuncPtr<n>` box a pointer: `0.until(22)` in `Type.boxesTo`.
const C_FUNCTION_POINTER_BOXES: u32 = 22;

/// The names of `nir.Rt` and `nir.Type`'s tables.
pub struct Rt {
    pub object: Global,
    pub class: Global,
    pub string: Global,
    /// `scala.scalanative.runtime.package$`.
    pub runtime: Global,
    pub runtime_nothing: Global,
    pub runtime_null: Global,
    pub boxed_ptr: Global,
    pub boxed_unit: Global,
    pub boxed_unit_module: Global,
    pub generic_array: Global,
    pub blob_array: Global,
    pub object_ty: Ty,
    pub class_ty: Ty,
    pub string_ty: Ty,
    pub throwable_ty: Ty,
    pub runtime_ty: Ty,
    pub ptr: Ty,
    pub bool: Ty,
    pub char: Ty,
    pub byte: Ty,
    pub short: Ty,
    pub int: Ty,
    pub long: Ty,
    pub float: Ty,
    pub double: Ty,
    pub size: Ty,
    pub unit: Ty,
    pub nothing: Ty,
    pub null: Ty,
    /// `Type.typeToArray` in its order: the element's type and the array's class, the object
    /// array's last.
    pub arrays: Vec<(Ty, Global)>,
    /// `Rt.PrimitiveTypes`.
    pub primitives: Vec<Global>,
    /// `Type.boxesTo`: a box's class and the type it holds.
    pub boxes: Vec<(Global, Ty)>,
    /// `Rt.arrayAlloc`: the signature of each array class's `alloc`.
    pub array_alloc: Vec<(Str, Global)>,
    pub class_fields: Vec<Global>,
    pub string_fields: Vec<Global>,
}

impl Rt {
    pub fn new(cx: &mut Cx) -> Rt {
        let object = cx.top("java.lang.Object");
        let class = cx.top("java.lang.Class");
        let string = cx.top("java.lang.String");
        let throwable = cx.top("java.lang.Throwable");
        let runtime = cx.top("scala.scalanative.runtime.package$");
        let object_ty = cx.class_ty(object);
        let prim = |cx: &mut Cx, t: Type| cx.ty(t);
        let (ptr, bool, char, byte, short) = (prim(cx, Type::Ptr), prim(cx, Type::Bool), prim(cx, Type::Char), prim(cx, Type::Byte), prim(cx, Type::Short));
        let (int, long, float, double, size) = (prim(cx, Type::Int), prim(cx, Type::Long), prim(cx, Type::Float), prim(cx, Type::Double), prim(cx, Type::Size));
        let rt = |cx: &mut Cx, name: &str| cx.top(&format!("scala.scalanative.runtime.{}", name));
        let arrays: Vec<(Ty, Global)> = [(bool, "BooleanArray"), (char, "CharArray"), (byte, "ByteArray"), (short, "ShortArray"), (int, "IntArray"), (long, "LongArray"), (float, "FloatArray"), (double, "DoubleArray"), (object_ty, "ObjectArray")]
            .into_iter()
            .map(|(ty, name)| (ty, rt(cx, name)))
            .collect();
        let blob_array = rt(cx, "BlobArray");
        let primitives = ["PrimitiveByte", "PrimitiveShort", "PrimitiveInt", "PrimitiveLong", "PrimitiveChar", "PrimitiveFloat", "PrimitiveDouble", "PrimitiveBoolean", "PrimitiveUnit", "RawSize", "RawPtr"].into_iter().map(|n| rt(cx, n)).collect();
        let mut boxes: Vec<(Global, Ty)> = Vec::new();
        for (name, ty) in [
            ("scala.scalanative.unsigned.USize", size),
            ("scala.scalanative.unsigned.UByte", byte),
            ("scala.scalanative.unsigned.UShort", short),
            ("scala.scalanative.unsigned.UInt", int),
            ("scala.scalanative.unsigned.ULong", long),
            ("scala.scalanative.unsafe.CArray", ptr),
            ("scala.scalanative.unsafe.CVarArgList", ptr),
            ("scala.scalanative.unsafe.Ptr", ptr),
            ("java.lang.Boolean", bool),
            ("java.lang.Character", char),
            ("scala.scalanative.unsafe.Size", size),
            ("java.lang.Byte", byte),
            ("java.lang.Short", short),
            ("java.lang.Integer", int),
            ("java.lang.Long", long),
            ("java.lang.Float", float),
            ("java.lang.Double", double),
        ] {
            boxes.push((cx.top(name), ty));
        }
        for arity in 0..C_FUNCTION_POINTER_BOXES {
            boxes.push((cx.top(&format!("scala.scalanative.unsafe.CFuncPtr{}", arity)), ptr));
        }
        let mut array_alloc = Vec::new();
        for &(_, cls) in arrays.iter().chain(std::iter::once(&(byte, blob_array))) {
            let ret = cx.class_ty(cls);
            array_alloc.push((cx.method_sig("alloc", &[int, ret], Scope::Public), cls));
        }
        let field = |cx: &mut Cx, owner: Global, id: &str| {
            let sig = cx.field_sig(id, Scope::Public);
            cx.member(owner, sig)
        };
        let class_fields = ["id", "interfacesCount", "interfaces", "name", "size", "idRangeUntil", "refFieldOffsets", "itablesCount", "itables", "superClass"].into_iter().map(|id| field(cx, class, id)).collect();
        let string_fields = ["value", "offset", "count", "cachedHashCode"].into_iter().map(|id| field(cx, string, id)).collect();
        Rt {
            object,
            class,
            string,
            runtime,
            runtime_nothing: cx.top("scala.runtime.Nothing$"),
            runtime_null: cx.top("scala.runtime.Null$"),
            boxed_ptr: cx.top("scala.scalanative.unsafe.Ptr"),
            boxed_unit: cx.top("scala.runtime.BoxedUnit"),
            boxed_unit_module: cx.top("scala.scalanative.runtime.BoxedUnit$"),
            generic_array: rt(cx, "Array"),
            blob_array,
            object_ty,
            class_ty: cx.class_ty(class),
            string_ty: cx.class_ty(string),
            throwable_ty: cx.class_ty(throwable),
            runtime_ty: cx.class_ty(runtime),
            ptr,
            bool,
            char,
            byte,
            short,
            int,
            long,
            float,
            double,
            size,
            unit: cx.ty(Type::Unit),
            nothing: cx.ty(Type::Nothing),
            null: cx.ty(Type::Null),
            arrays,
            primitives,
            boxes,
            array_alloc,
            class_fields,
            string_fields,
        }
    }

    /// `Type.toArrayClass`: the array class of an element type, the object array for a reference.
    pub fn array_class(&self, of: Ty) -> Global {
        self.arrays.iter().find(|&&(ty, _)| ty == of).map_or(self.arrays[8].1, |&(_, cls)| cls)
    }

    /// `Type.fromArrayClass`: the element type of an array class.
    pub fn array_element(&self, cls: Global) -> Option<Ty> {
        if cls == self.blob_array {
            return Some(self.byte);
        }
        self.arrays.iter().find(|&&(_, c)| c == cls).map(|&(ty, _)| ty)
    }

    /// `RefKind.className`: the class a reference type names, none for a type of values.
    pub fn class_name(&self, cx: &Cx, ty: Ty) -> Option<Global> {
        match cx.type_of(ty) {
            Type::Null => Some(self.runtime_null),
            Type::Unit => Some(self.boxed_unit),
            Type::Array { of, .. } => Some(self.array_class(*of)),
            Type::Ref { name, .. } => Some(*name),
            _ => None,
        }
    }

    /// `Type.unbox`: the type a box's class holds.
    pub fn unboxed(&self, cls: Global) -> Option<Ty> {
        self.boxes.iter().find(|&&(c, _)| c == cls).map(|&(_, ty)| ty)
    }

    /// `Type.box`: the class that boxes a type, the last of the table's that hold it.
    pub fn boxed(&self, ty: Ty) -> Option<Global> {
        self.boxes.iter().rev().find(|&&(_, held)| held == ty).map(|&(cls, _)| cls)
    }

    /// `Val.ty`.
    pub fn val_ty(&self, cx: &mut Cx, v: &Val) -> Ty {
        match v {
            Val::Null => self.null,
            Val::Zero(ty) | Val::Local(_, ty) | Val::Global(_, ty) => *ty,
            Val::True | Val::False => self.bool,
            Val::Size(_) => self.size,
            Val::Char(_) => self.char,
            Val::Byte(_) => self.byte,
            Val::Short(_) => self.short,
            Val::Int(_) => self.int,
            Val::Long(_) => self.long,
            Val::Int128 { .. } => cx.ty(Type::Int128),
            Val::Float(_) => self.float,
            Val::Double(_) => self.double,
            Val::StructValue(vals) => {
                let tys: Vec<Ty> = vals.iter().map(|v| self.val_ty(cx, v)).collect();
                cx.ty(Type::StructValue(tys.into_boxed_slice()))
            }
            Val::ArrayValue(of, vals) => cx.ty(Type::ArrayValue(*of, vals.len() as i32)),
            Val::ByteString(bytes, _) => cx.ty(Type::ArrayValue(self.byte, bytes.len() as i32 + 1)),
            Val::Unit => self.unit,
            Val::Const(_) => self.ptr,
            Val::String(_) => cx.ty(Type::Ref { name: self.string, exact: true, nullable: false }),
            Val::Virtual(_) => cx.ty(Type::Virtual),
            Val::ClassOf(_) => self.class_ty,
        }
    }

    /// `Op.resty`: the type of an operation's result.
    pub fn op_resty(&self, cx: &mut Cx, op: &Op) -> Ty {
        match op {
            Op::Call { ty, .. } => match cx.type_of(*ty) {
                Type::Function(_, ret) => *ret,
                _ => self.ptr,
            },
            Op::Load { ty, .. } | Op::Bin { ty, .. } | Op::Conv { ty, .. } | Op::Fieldload { ty, .. } | Op::As { ty, .. } | Op::Arrayload { ty, .. } => *ty,
            Op::Store { .. } | Op::Fence(_) | Op::Fieldstore { .. } | Op::Varstore { .. } | Op::Arraystore { .. } => self.unit,
            Op::Elem { .. } | Op::Stackalloc { .. } | Op::Field { .. } | Op::Method { .. } | Op::Dynmethod { .. } => self.ptr,
            Op::Extract { aggr, indexes } => {
                let mut ty = self.val_ty(cx, aggr);
                for &i in indexes.iter() {
                    ty = match cx.type_of(ty) {
                        Type::StructValue(tys) => tys.get(i as usize).copied().unwrap_or(ty),
                        Type::ArrayValue(of, _) => *of,
                        _ => ty,
                    };
                }
                ty
            }
            Op::Insert { aggr, .. } => self.val_ty(cx, aggr),
            Op::Comp { .. } | Op::Is { .. } => self.bool,
            Op::Classalloc { name, .. } | Op::Module(name) => cx.ty(Type::Ref { name: *name, exact: true, nullable: false }),
            Op::Copy(v) => self.val_ty(cx, v),
            Op::SizeOf(_) | Op::AlignmentOf(_) => self.size,
            Op::Box { ty, .. } => match self.class_name(cx, *ty) {
                Some(name) => cx.ty(Type::Ref { name, exact: true, nullable: self.unboxed(name) == Some(self.ptr) }),
                None => *ty,
            },
            Op::Unbox { ty, .. } => self.class_name(cx, *ty).and_then(|name| self.unboxed(name)).unwrap_or(*ty),
            Op::Var(ty) => cx.ty(Type::Var(*ty)),
            Op::Varload(slot) => {
                let slot = self.val_ty(cx, slot);
                match cx.type_of(slot) {
                    Type::Var(of) => *of,
                    _ => slot,
                }
            }
            Op::Arrayalloc { ty, .. } => cx.ty(Type::Array { of: *ty, nullable: false }),
            Op::Arraylength(_) => self.int,
        }
    }
}
