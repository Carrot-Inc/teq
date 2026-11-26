//! NIR as text, character for character what `nir.Show` of Scala Native 0.5.12 prints: the form
//! its tools dump, and the one the reader is held to against them.

use super::*;
use std::io::Write;

pub struct Show<'c> {
    cx: &'c Cx,
    pub out: Vec<u8>,
    indent: usize,
}

const NO_DEBUG: &DebugInfo = &DebugInfo::NONE;

impl<'c> Show<'c> {
    pub fn new(cx: &'c Cx) -> Show<'c> {
        Show { cx, out: Vec::new(), indent: 0 }
    }

    fn str(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }

    fn num(&mut self, n: impl std::fmt::Display) {
        let _ = write!(self.out, "{}", n);
    }

    fn newline(&mut self) {
        self.out.push(b'\n');
        for _ in 0..self.indent {
            self.out.extend_from_slice(b"  ");
        }
    }

    /// A string's text as a UTF-8 encoder writes its UTF-16: an unpaired surrogate as `?`.
    fn text(&mut self, bytes: &[u8]) {
        if !bytes.contains(&0xed) {
            return self.out.extend_from_slice(bytes);
        }
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0xed && bytes.get(i + 1).is_some_and(|&b| b >= 0xa0) {
                self.out.push(b'?');
                i += 3;
            } else {
                self.out.push(bytes[i]);
                i += 1;
            }
        }
    }

    fn quoted(&mut self, bytes: &[u8]) {
        self.out.push(b'"');
        let escaped = escape_quotes(bytes);
        self.text(&escaped);
        self.out.push(b'"');
    }

    fn sep<T>(&mut self, items: &[T], sep: &str, mut each: impl FnMut(&mut Self, &T)) {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.str(sep);
            }
            each(self, item);
        }
    }

    pub fn global(&mut self, g: Global) {
        let mut mangled = Vec::new();
        self.cx.mangle(g, &mut mangled);
        self.out.push(b'@');
        self.quoted(&mangled);
    }

    fn local(&mut self, local: Local, debug: &DebugInfo) {
        self.out.push(b'%');
        self.num(local as i64);
        if let Some(name) = debug.local_name(local) {
            self.str(" <");
            self.text(self.cx.bytes(name));
            self.str(">");
        }
    }

    pub fn ty(&mut self, ty: Ty) {
        match self.cx.type_of(ty) {
            Type::Vararg => self.str("..."),
            Type::Bool => self.str("bool"),
            Type::Ptr => self.str("ptr"),
            Type::Size => self.str("size"),
            Type::Char => self.str("char"),
            Type::Byte => self.str("byte"),
            Type::Short => self.str("short"),
            Type::Int => self.str("int"),
            Type::Int128 => self.str("int128"),
            Type::Long => self.str("long"),
            Type::Float => self.str("float"),
            Type::Double => self.str("double"),
            Type::ArrayValue(of, n) => {
                self.str("[");
                self.ty(*of);
                self.str(" x ");
                self.num(*n);
                self.str("]");
            }
            Type::Function(args, ret) => {
                self.str("(");
                self.sep(args, ", ", |s, &t| s.ty(t));
                self.str(") => ");
                self.ty(*ret);
            }
            Type::StructValue(tys) => {
                self.str("{");
                self.sep(tys, ", ", |s, &t| s.ty(t));
                self.str("}");
            }
            Type::Null => self.str("null"),
            Type::Nothing => self.str("nothing"),
            Type::Virtual => self.str("virtual"),
            Type::Var(of) => {
                self.str("var[");
                self.ty(*of);
                self.str("]");
            }
            Type::Unit => self.str("unit"),
            Type::Array { of, nullable } => {
                if !nullable {
                    self.str("?");
                }
                self.str("array[");
                self.ty(*of);
                self.str("]");
            }
            Type::Ref { name, exact, nullable } => {
                if *exact {
                    self.str("!");
                }
                if !nullable {
                    self.str("?");
                }
                self.global(*name);
            }
        }
    }

    fn val(&mut self, v: &Val, debug: &DebugInfo) {
        match v {
            Val::True => self.str("true"),
            Val::False => self.str("false"),
            Val::Null => self.str("null"),
            Val::Zero(ty) => {
                self.str("zero[");
                self.ty(*ty);
                self.str("]");
            }
            Val::Size(n) => {
                self.str("size ");
                self.num(n);
            }
            Val::Char(c) => {
                self.str("char ");
                self.num(c);
            }
            Val::Byte(n) => {
                self.str("byte ");
                self.num(n);
            }
            Val::Short(n) => {
                self.str("short ");
                self.num(n);
            }
            Val::Int(n) => {
                self.str("int ");
                self.num(n);
            }
            Val::Long(n) => {
                self.str("long ");
                self.num(n);
            }
            Val::Int128 { hi, lo } => {
                // `Val.Int128.bigIntValue`: `BigInt(hi) << 64 | BigInt(lo & 0xffffffffffffffffL)`,
                // where the mask, a `Long`, leaves the low half signed.
                self.str("int128 ");
                self.num(((*hi as i128) << 64) | (*lo as i128));
            }
            Val::Float(bits) => {
                self.str("float ");
                self.str(&crate::javanum::float_string(f32::from_bits(*bits)));
            }
            Val::Double(bits) => {
                self.str("double ");
                self.str(&crate::javanum::double_string(f64::from_bits(*bits)));
            }
            Val::StructValue(values) => {
                self.str("structvalue {");
                self.sep(values, ", ", |s, v| s.val(v, debug));
                self.str("}");
            }
            Val::ArrayValue(ty, values) => {
                self.str("arrayvalue ");
                self.ty(*ty);
                self.str(" {");
                self.sep(values, ", ", |s, v| s.val(v, debug));
                self.str("}");
            }
            Val::ByteString(bytes, _) => {
                let mut latin1 = Vec::with_capacity(bytes.len());
                for &b in bytes.iter() {
                    if b < 0x80 {
                        latin1.push(b);
                    } else {
                        latin1.extend_from_slice(&[0xc0 | (b >> 6), 0x80 | (b & 0x3f)]);
                    }
                }
                self.str("c\"");
                let escaped = escape_newlines(&escape_quotes(&latin1));
                self.out.extend_from_slice(&escaped);
                self.str("\"");
            }
            Val::Local(id, ty) => {
                self.local(*id, debug);
                self.str(" : ");
                self.ty(*ty);
            }
            Val::Global(name, ty) => {
                self.global(*name);
                self.str(" : ");
                self.ty(*ty);
            }
            Val::Unit => self.str("unit"),
            Val::Const(v) => {
                self.str("const ");
                self.val(v, debug);
            }
            Val::String(s) => {
                self.str("\"");
                let escaped = escape_newlines(&escape_quotes(self.cx.bytes(*s)));
                self.text(&escaped);
                self.str("\"");
            }
            Val::Virtual(key) => {
                self.str("virtual ");
                self.num(key);
            }
            Val::ClassOf(cls) => {
                self.str("classOf[");
                self.global(*cls);
                self.str("]");
            }
        }
    }

    fn next(&mut self, next: &Next, debug: &DebugInfo) {
        match next {
            Next::None => {}
            Next::Unwind { exc, ty, next } => {
                self.str("unwind ");
                self.local(*exc, debug);
                self.str(" : ");
                self.ty(*ty);
                self.str(" to ");
                self.next(next, debug);
            }
            Next::Case(v, next) => {
                self.str("case ");
                self.val(v, debug);
                self.str(" => ");
                self.next(next, debug);
            }
            Next::Label(name, args) => {
                self.local(*name, debug);
                if !args.is_empty() {
                    self.str("(");
                    self.sep(args, ", ", |s, v| s.val(v, debug));
                    self.str(")");
                }
            }
        }
    }

    fn memory_order(&mut self, order: MemoryOrder) {
        self.str(match order {
            MemoryOrder::Unordered => "unordered",
            MemoryOrder::Monotonic => "monotonic",
            MemoryOrder::Acquire => "acquire",
            MemoryOrder::Release => "release",
            MemoryOrder::AcqRel => "onAcqrel",
            MemoryOrder::SeqCst => "onSeqcst",
        });
    }

    /// The space `nir.Show` writes after a load or a store whether or not an order follows.
    fn order(&mut self, order: Option<MemoryOrder>) {
        self.str(" ");
        if let Some(order) = order {
            self.memory_order(order);
        }
    }

    fn bin(&mut self, bin: Bin) {
        self.str(match bin {
            Bin::Iadd => "iadd",
            Bin::Fadd => "fadd",
            Bin::Isub => "isub",
            Bin::Fsub => "fsub",
            Bin::Imul => "imul",
            Bin::Fmul => "fmul",
            Bin::Sdiv => "sdiv",
            Bin::Udiv => "udiv",
            Bin::Fdiv => "fdiv",
            Bin::Srem => "srem",
            Bin::Urem => "urem",
            Bin::Frem => "frem",
            Bin::Shl => "shl",
            Bin::Lshr => "lshr",
            Bin::Ashr => "ashr",
            Bin::And => "and",
            Bin::Or => "or",
            Bin::Xor => "xor",
        });
    }

    fn comp(&mut self, comp: Comp) {
        self.str(match comp {
            Comp::Ieq => "ieq",
            Comp::Ine => "ine",
            Comp::Ugt => "ugt",
            Comp::Uge => "uge",
            Comp::Ult => "ult",
            Comp::Ule => "ule",
            Comp::Sgt => "sgt",
            Comp::Sge => "sge",
            Comp::Slt => "slt",
            Comp::Sle => "sle",
            Comp::Feq => "feq",
            Comp::Fne => "fne",
            Comp::Fgt => "fgt",
            Comp::Fge => "fge",
            Comp::Flt => "flt",
            Comp::Fle => "fle",
        });
    }

    fn conv(&mut self, conv: Conv) {
        self.str(match conv {
            Conv::SSizeCast => "ssizecast",
            Conv::ZSizeCast => "zsizecast",
            Conv::Trunc => "trunc",
            Conv::Zext => "zext",
            Conv::Sext => "sext",
            Conv::Fptrunc => "fptrunc",
            Conv::Fpext => "fpext",
            Conv::Fptoui => "fptoui",
            Conv::Fptosi => "fptosi",
            Conv::Uitofp => "uitofp",
            Conv::Sitofp => "sitofp",
            Conv::Ptrtoint => "ptrtoint",
            Conv::Inttoptr => "inttoptr",
            Conv::Bitcast => "bitcast",
        });
    }

    /// `name[ty] ` and the operands, each after a comma.
    fn typed(&mut self, name: &str, ty: Ty, operands: &[&Val], debug: &DebugInfo) {
        self.str(name);
        self.str("[");
        self.ty(ty);
        self.str("] ");
        self.sep(operands, ", ", |s, v| s.val(v, debug));
    }

    fn op(&mut self, op: &Op, debug: &DebugInfo) {
        match op {
            Op::Call { ty, ptr, args } => {
                self.typed("call", *ty, &[ptr], debug);
                self.str("(");
                self.sep(args, ", ", |s, v| s.val(v, debug));
                self.str(")");
            }
            Op::Load { ty, ptr, order } => {
                if order.is_some() {
                    self.str("atomic ");
                }
                self.typed("load", *ty, &[ptr], debug);
                self.order(*order);
            }
            Op::Store { ty, ptr, value, order } => {
                if order.is_some() {
                    self.str("atomic ");
                }
                self.typed("store", *ty, &[ptr, value], debug);
                self.order(*order);
            }
            Op::Elem { ty, ptr, indexes } => {
                self.typed("elem", *ty, &[ptr], debug);
                self.str(", ");
                self.sep(indexes, ", ", |s, v| s.val(v, debug));
            }
            Op::Extract { aggr, indexes } => {
                self.str("extract ");
                self.val(aggr, debug);
                self.str(", ");
                self.sep(indexes, ", ", |s, i| s.num(i));
            }
            Op::Insert { aggr, value, indexes } => {
                self.str("insert ");
                self.val(aggr, debug);
                self.str(", ");
                self.val(value, debug);
                self.str(", ");
                self.sep(indexes, ", ", |s, i| s.num(i));
            }
            Op::Stackalloc { ty, n } => {
                self.str("stackalloc[");
                self.ty(*ty);
                self.str("] ");
                self.val(n, debug);
            }
            Op::Bin { bin, ty, l, r } => {
                self.bin(*bin);
                self.typed("", *ty, &[l, r], debug);
            }
            Op::Comp { comp, ty, l, r } => {
                self.comp(*comp);
                self.typed("", *ty, &[l, r], debug);
            }
            Op::Conv { conv, ty, value } => {
                self.conv(*conv);
                self.typed("", *ty, &[value], debug);
            }
            Op::Fence(order) => {
                self.str("fence ");
                self.memory_order(*order);
            }
            Op::Classalloc { name, zone } => {
                self.str("classalloc ");
                self.global(*name);
                if let Some(zone) = zone {
                    self.str(" inZone ");
                    self.val(zone, debug);
                }
            }
            Op::Fieldload { ty, obj, name } => {
                self.typed("fieldload", *ty, &[obj], debug);
                self.str(", ");
                self.global(*name);
            }
            Op::Fieldstore { ty, obj, name, value } => {
                self.typed("fieldstore", *ty, &[obj], debug);
                self.str(", ");
                self.global(*name);
                self.str(", ");
                self.val(value, debug);
            }
            Op::Field { obj, name } => {
                self.str("field ");
                self.val(obj, debug);
                self.str(", ");
                self.global(*name);
            }
            Op::Method { obj, sig } | Op::Dynmethod { obj, sig } => {
                self.str(if matches!(op, Op::Method { .. }) { "method " } else { "dynmethod " });
                self.val(obj, debug);
                self.str(", ");
                self.quoted(self.cx.bytes(*sig));
            }
            Op::Module(name) => {
                self.str("module ");
                self.global(*name);
            }
            Op::As { ty, obj } => self.typed("as", *ty, &[obj], debug),
            Op::Is { ty, obj } => self.typed("is", *ty, &[obj], debug),
            Op::Copy(value) => {
                self.str("copy ");
                self.val(value, debug);
            }
            Op::SizeOf(ty) => self.typed("sizeOf", *ty, &[], debug),
            Op::AlignmentOf(ty) => self.typed("alignmentOf", *ty, &[], debug),
            Op::Box { ty, obj } => self.typed("box", *ty, &[obj], debug),
            Op::Unbox { ty, obj } => self.typed("unbox", *ty, &[obj], debug),
            Op::Var(ty) => {
                self.str("var[");
                self.ty(*ty);
                self.str("]");
            }
            Op::Varload(slot) => {
                self.str("varload ");
                self.val(slot, debug);
            }
            Op::Varstore { slot, value } => {
                self.str("varstore ");
                self.val(slot, debug);
                self.str(", ");
                self.val(value, debug);
            }
            Op::Arrayalloc { ty, init, zone } => {
                self.typed("arrayalloc", *ty, &[init], debug);
                if let Some(zone) = zone {
                    self.str(" inZone ");
                    self.val(zone, debug);
                }
            }
            Op::Arrayload { ty, arr, idx } => self.typed("arrayload", *ty, &[arr, idx], debug),
            Op::Arraystore { ty, arr, idx, value } => self.typed("arraystore", *ty, &[arr, idx, value], debug),
            Op::Arraylength(arr) => {
                self.str("arraylength ");
                self.val(arr, debug);
            }
        }
    }

    fn linktime_condition(&mut self, cond: &LinktimeCondition) {
        match cond {
            LinktimeCondition::Simple { property, comparison, value, .. } => {
                self.text(self.cx.bytes(*property));
                self.str(" ");
                self.comp(*comparison);
                self.str(" ");
                self.val(value, NO_DEBUG);
            }
            LinktimeCondition::Complex { op, left, right, .. } => {
                self.linktime_condition(left);
                self.str(" ");
                self.bin(*op);
                self.str(" ");
                self.linktime_condition(right);
            }
        }
    }

    fn unwind(&mut self, unwind: &Next, debug: &DebugInfo) {
        if *unwind != Next::None {
            self.str(" ");
            self.next(unwind, debug);
        }
    }

    fn inst(&mut self, inst: &Inst, debug: &DebugInfo) {
        match &inst.kind {
            InstKind::Label { id, params } => {
                self.local(*id, debug);
                if !params.is_empty() {
                    self.str("(");
                    self.sep(params, ", ", |s, &(id, ty)| {
                        s.local(id, debug);
                        s.str(" : ");
                        s.ty(ty);
                    });
                    self.str(")");
                }
                self.str(":");
            }
            InstKind::Let { id, op, unwind, scope } => {
                if *scope != 0 {
                    self.num(*scope as i32);
                    self.str(": ");
                }
                self.local(*id, debug);
                self.str(" = ");
                self.op(op, debug);
                self.unwind(unwind, debug);
            }
            InstKind::Ret(value) => {
                self.str("ret ");
                self.val(value, debug);
            }
            InstKind::Jump(next) => {
                self.str("jump ");
                self.next(next, debug);
            }
            InstKind::If { value, then, otherwise } => {
                self.str("if ");
                self.val(value, debug);
                self.str(" then ");
                self.next(then, debug);
                self.str(" else ");
                self.next(otherwise, debug);
            }
            InstKind::LinktimeIf { cond, then, otherwise } => {
                self.str("linktime if ");
                self.linktime_condition(cond);
                self.str(" then ");
                self.next(then, debug);
                self.str(" else ");
                self.next(otherwise, debug);
            }
            InstKind::Switch { value, default, cases } => {
                self.str("switch ");
                self.val(value, debug);
                self.str(" {");
                for case in cases.iter() {
                    self.str(" ");
                    self.next(case, debug);
                }
                self.str(" default => ");
                self.next(default, debug);
                self.str(" }");
            }
            InstKind::Throw { value, unwind } => {
                self.str("throw ");
                self.val(value, debug);
                self.unwind(unwind, debug);
            }
            InstKind::Unreachable { unwind } => {
                self.str("unreachable");
                self.unwind(unwind, debug);
            }
        }
    }

    /// The attributes in `nir.Attrs.toSeq`'s order, each followed by a space.
    fn attrs(&mut self, a: &Attrs) {
        match a.inline {
            Inline::May => {}
            Inline::Hint => self.str("inlinehint "),
            Inline::No => self.str("noinline "),
            Inline::Always => self.str("alwaysinline "),
        }
        if a.no_specialize {
            self.str("nospecialize ");
        }
        match a.opt {
            Opt::Un => {}
            Opt::No => self.str("noopt "),
            Opt::Did => self.str("didopt "),
            Opt::Bail(msg) => {
                self.str("bailopt(");
                self.quoted(self.cx.bytes(msg));
                self.str(") ");
            }
        }
        if let Some((size, group)) = a.align {
            self.str("align(");
            self.num(size);
            if let Some(group) = group {
                self.str(", ");
                let escaped = escape_quotes(self.cx.bytes(group));
                self.text(&escaped);
            }
            self.str(") ");
        }
        if a.is_extern {
            self.str(if a.is_blocking { "extern blocking " } else { "extern " });
        }
        for (set, name) in [
            (a.is_dyn, "dyn "),
            (a.is_stub, "stub "),
            (a.link_cpp_runtime, "linkCppRuntime "),
            (a.is_abstract, "abstract "),
            (a.is_volatile, "volatile "),
            (a.is_final, "final "),
            (a.is_safe_publish, "safe-publish "),
            (a.is_linktime_resolved, "linktime "),
            (a.uses_intrinsic, "uses-intrinsics "),
        ] {
            if set {
                self.str(name);
            }
        }
        for (list, name) in [(&a.links, "link("), (&a.defines, "define(")] {
            for &s in list {
                self.str(name);
                self.quoted(self.cx.bytes(s));
                self.str(") ");
            }
        }
    }

    fn parents(&mut self, parent: Option<Global>, traits: &[Global]) {
        let all: Vec<Global> = parent.into_iter().chain(traits.iter().copied()).collect();
        if !all.is_empty() {
            self.str(" : ");
            self.sep(&all, ", ", |s, &g| s.global(g));
        }
    }

    pub fn defn(&mut self, defn: &Defn) {
        self.attrs(&defn.attrs);
        let keyword = match &defn.kind {
            DefnKind::Var { .. } => "var ",
            DefnKind::Const { .. } => "const ",
            DefnKind::Declare { .. } => "decl ",
            DefnKind::Define { .. } => "def ",
            DefnKind::Trait { .. } => "trait ",
            DefnKind::Class { .. } => "class ",
            DefnKind::Module { .. } => "module ",
        };
        self.str(keyword);
        self.global(defn.name);
        match &defn.kind {
            DefnKind::Var { ty, rhs } | DefnKind::Const { ty, rhs } => {
                self.str(" : ");
                self.ty(*ty);
                self.str(" = ");
                self.val(rhs, NO_DEBUG);
            }
            DefnKind::Declare { ty } => {
                self.str(" : ");
                self.ty(*ty);
            }
            DefnKind::Define { ty, insts, debug } => {
                self.str(" : ");
                self.ty(*ty);
                self.str(" {");
                for inst in insts {
                    self.indent = usize::from(!matches!(inst.kind, InstKind::Label { .. }));
                    self.newline();
                    self.inst(inst, debug);
                }
                self.indent = 0;
                self.newline();
                self.str("}");
            }
            DefnKind::Trait { traits } => self.parents(None, traits),
            DefnKind::Class { parent, traits } | DefnKind::Module { parent, traits } => self.parents(*parent, traits),
        }
    }
}

/// A backslash before each `"` that does not follow one.
fn escape_quotes(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut escaped = false;
    for &b in s {
        if b == b'"' && !escaped {
            out.push(b'\\');
        }
        out.push(b);
        escaped = b == b'\\';
    }
    out
}

/// `nir.Show`'s replacement of `([^\\]|^)\n` by the character and a written `\n`, as a regular
/// expression's successive matches do it: a line feed after a character that is no backslash,
/// or the string's first, is written; one that directly follows a replaced pair stays.
fn escape_newlines(s: &[u8]) -> Vec<u8> {
    if !s.contains(&b'\n') {
        return s.to_vec();
    }
    let mut out = Vec::with_capacity(s.len() + 8);
    let mut i = 0;
    while i < s.len() {
        let len = match s[i] {
            0..=0xbf => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        }
        .min(s.len() - i);
        if s[i] != b'\\' && s.get(i + len) == Some(&b'\n') {
            out.extend_from_slice(&s[i..i + len]);
            out.extend_from_slice(b"\\n");
            i += len + 1;
        } else if i == 0 && s[0] == b'\n' {
            out.extend_from_slice(b"\\n");
            i += 1;
        } else {
            out.extend_from_slice(&s[i..i + len]);
            i += len;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_a_128_bit_integer_as_scala_native_does() {
        let cx = Cx::new();
        let text = |hi, lo| {
            let mut show = Show::new(&cx);
            show.val(&Val::Int128 { hi, lo }, NO_DEBUG);
            String::from_utf8(show.out).unwrap()
        };
        assert_eq!(text(0, 5), "int128 5");
        assert_eq!(text(1, 5), "int128 18446744073709551621");
        assert_eq!(text(0, -1), "int128 -1");
        assert_eq!(text(1, -1), "int128 -1");
        assert_eq!(text(-1, 0), "int128 -18446744073709551616");
        assert_eq!(text(i64::MIN, 0), "int128 -170141183460469231731687303715884105728");
    }
}
