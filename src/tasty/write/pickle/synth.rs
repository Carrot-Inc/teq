//! The bodies scalac's `SyntheticMembers` (in `PostTyper`) and `Desugar` give a case class, a
//! case object and their companions, written from
//! the class's symbols as scalac 3.8.4 writes them.

use super::*;

/// Which synthesized body a member takes.
#[derive(Clone, Copy)]
pub(super) enum Synth {
    /// Left `ELIDED`, counted under its name.
    Elided,
    /// `hashCode()` of a case class: the name's hash, the mix of the fields, or
    /// `MurmurHash3.productHash`.
    HashCode(ClassId),
    /// `equals(x$0)` of a case class.
    Equals(ClassId),
    /// `toString()`: `ScalaRunTime._toString(this)`, an object's name.
    ToString(ClassId),
    /// `canEqual(that)`: `that.isInstanceOf[C @unchecked]`.
    CanEqual(ClassId),
    ProductArity(ClassId),
    ProductPrefix(ClassId),
    /// `productElement(n)`: `n match { case i => this._{i+1}; case _ => throw ..}`.
    ProductElement(ClassId),
    /// `productElementName(n)`: the fields' names by index.
    ProductElementName(ClassId),
    /// A string literal: a companion's `toString`, an object's `productPrefix`.
    Name(ClassId),
    /// `this.x`, the field of the class: `_N`, `copy$default$N`.
    Field(ClassId, u32),
    /// `new C[T](params)`: `copy`, a companion's `apply`.
    New(ClassId),
    /// The parameter itself: a companion's `unapply`.
    Param,
    /// `()`: a var's setter before `Memoize`.
    Unit,
    /// `hashCode` of a value class: its field's `hashCode()`, `Objects.hashCode` of a reference.
    ValueHashCode(ClassId),
    /// `equals` of a value class: the match without `eq` and `canEqual`.
    ValueEquals(ClassId),
    /// A value case of an enum of values: `$new(ordinal, "Name")`, by the enum and the case.
    EnumValue(ClassId, ClassId),
    /// `$values`: `Array[E](values)(ClassTag[E](classOf[E]))`.
    EnumArray(ClassId),
    /// `values`: `$values.clone()`.
    EnumValuesClone(ClassId),
    /// `valueOf($name)`: the value of the name, `IllegalArgumentException` otherwise.
    EnumValueOf(ClassId),
    /// `fromOrdinal(ordinal)`: `$values(ordinal)` or the match of the values' ordinals.
    FromOrdinal(ClassId),
    /// `x$0.ordinal`: the companion's `ordinal` of its mirror, by the enum.
    SelectOrdinal(ClassId),
    /// `$new(_$ordinal, $name)` of an enum of values: the instance of an anonymous subclass
    /// whose members answer the two.
    EnumNew(ClassId),
    /// A value case scalac expands as an enum module, one of an enum of type parameters or with
    /// a parent of its own: the instance of an anonymous subclass of that parent, by the enum
    /// and the case.
    EnumObject(ClassId, ClassId),
    Int(i32),
    /// `scala.scalajs.js.native`, a facade member's body as the source writes it.
    JsNative,
}

/// `java.lang.String.hashCode` of a name.
fn string_hash(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |h, u| h.wrapping_mul(31).wrapping_add(u as i32))
}

/// `scala.runtime.Statics.mix`.
fn statics_mix(hash: i32, data: i32) -> i32 {
    let k = data.wrapping_mul(0xcc9e2d51u32 as i32).rotate_left(15).wrapping_mul(0x1b873593);
    let h = (hash ^ k).rotate_left(13);
    h.wrapping_mul(5).wrapping_add(0xe6546b64u32 as i32)
}

impl<'w, 'a> P<'w, 'a> {
    /// A synthesized definition: its parameters, result, body and flags.
    pub(super) fn synth_def(&mut self, name: &'static str, params: &[(&str, TypeId)], empty_clause: bool, ret: TypeId, flags: &[u8], body: Synth) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        if empty_clause && params.is_empty() {
            self.buf.byte(EMPTYCLAUSE);
        }
        let mut addrs = Vec::new();
        for &(pname, pty) in params {
            let pa = self.buf.addr();
            addrs.push(pa);
            self.buf.byte(PARAM);
            let pl = self.buf.begin_length();
            let pn = self.names.simple(pname);
            self.buf.nat(pn as u64);
            self.tpt(pty);
            self.synthetic_position(pa);
            self.buf.end_length(pl);
        }
        self.tpt(ret);
        self.synth_rhs(name, ret, body, &addrs);
        self.write_flags(flags);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }

    /// A synthesized right-hand side, rolled back to `ELIDED` where a part of it cannot be
    /// written.
    pub(super) fn synth_rhs(&mut self, name: &'static str, ret: TypeId, body: Synth, params: &[usize]) {
        if matches!(body, Synth::Elided) {
            return self.rhs(Producer::Synthesized(name), ret);
        }
        let mark = self.body_begin();
        self.synth_body(body, params);
        match self.body_end(mark) {
            Ok(()) => self.count_body(Producer::Synthesized(name)),
            Err(reason) => {
                self.count_withheld(&reason);
                self.elided(ret);
            }
        }
    }

    /// A synthesized body of no parameters, written into the body the caller opened.
    pub(super) fn synth_body_of(&mut self, body: Synth) {
        self.synth_body(body, &[]);
    }

    fn synth_body(&mut self, body: Synth, params: &[usize]) {
        match body {
            Synth::Elided => {}
            Synth::HashCode(c) => self.case_hash_code(c),
            Synth::Equals(c) => self.equals_body(c, params[0], false),
            Synth::ToString(c) => {
                // An object nested in a class or a block is a class of teq's with its instance's
                // val, an object of scalac's.
                let info = self.w.syms.class(c);
                if info.kind == ClassKind::Object || info.inner_object.is_some() || info.local_module.is_some() {
                    return self.name_literal(c);
                }
                self.term_at(None);
                let l = self.open_tree(APPLY);
                self.library_static("scala.runtime", "ScalaRunTime", "_toString", &["scala.Product"], "java.lang.String");
                self.synth_this(c);
                self.buf.end_length(l);
            }
            Synth::CanEqual(c) => {
                self.term_at(None);
                let ta = self.open_tree(TYPEAPPLY);
                let sl = self.open_tree(SELECTIN);
                let n = self.names.signed("isInstanceOf", None, &[SigParam::Types(1)], "scala.Boolean");
                self.buf.nat(n as u64);
                self.param_ref(params[0]);
                self.external_typeref("scala", "Any");
                self.buf.end_length(sl);
                self.mark_tree();
                let t = self.synth_class_type(c);
                self.unchecked_type(t);
                self.buf.end_length(ta);
            }
            Synth::ProductArity(c) => {
                let n = self.case_fields(c).len() as i64;
                self.mark_tree();
                self.buf.byte(INTCONST);
                self.buf.long_int(n);
            }
            Synth::ProductPrefix(c) | Synth::Name(c) => self.name_literal(c),
            Synth::ProductElement(c) => self.product_match(c, params[0], false),
            Synth::ProductElementName(c) => self.product_match(c, params[0], true),
            Synth::New(c) => {
                let ctor = self.w.syms.class(c).ctor.clone();
                let mut clauses = Vec::new();
                let mut at = 0;
                for cl in &ctor {
                    clauses.push(params[at.min(params.len())..(at + cl.params.len()).min(params.len())].to_vec());
                    at += cl.params.len();
                }
                if at != params.len() {
                    return self.fail("a constructor call's parameters".to_string());
                }
                let targs: Vec<TypeId> = self.w.syms.class(c).own_tparams().to_vec().into_iter().map(|p| self.w.types.param(p)).collect();
                self.synth_new(c, &targs, &clauses);
            }
            Synth::Param => self.param_ref(params[0]),
            Synth::Unit => {
                self.mark_tree();
                self.buf.byte(UNITCONST);
            }
            Synth::ValueHashCode(c) => self.value_hash_code(c),
            Synth::EnumValue(e, case) => self.enum_value(e, case),
            Synth::EnumArray(e) => self.enum_array(e),
            Synth::EnumValuesClone(e) => {
                let arr = self.enum_array_type(e);
                self.term_at(None);
                let l = self.open_tree(APPLY);
                let sl = self.open_tree(SELECTIN);
                let n = self.names.signed("clone", None, &[], "java.lang.Object");
                self.buf.nat(n as u64);
                self.companion_member("$values");
                self.ty(arr);
                self.buf.end_length(sl);
                self.buf.end_length(l);
            }
            Synth::EnumValueOf(e) => self.enum_value_of(e, params[0]),
            Synth::FromOrdinal(e) => self.from_ordinal(e, params[0]),
            Synth::SelectOrdinal(e) => self.ordinal_of(e, |p| p.param_ref(params[0])),
            Synth::Int(i) => self.int_literal(i),
            Synth::JsNative => {
                self.term_at(None);
                self.buf.byte(SELECT);
                let n = self.names.simple("native");
                self.buf.nat(n as u64);
                self.term_at(None);
                self.buf.byte(SELECT);
                let p = self.names.simple("package");
                self.buf.nat(p as u64);
                self.term_at(None);
                self.package_path("scala.scalajs.js");
            }
            Synth::EnumNew(e) => {
                let et = self.w.types.class(e, &[]);
                self.enum_value_class(e, et, None, EnumLabel::Params(params[0], params[1]));
            }
            Synth::EnumObject(e, case) => {
                let info = self.w.syms.class(case);
                let label = EnumLabel::Literal(info.ordinal as i32, self.name(info.name));
                let Some(s) = info.singleton else { return self.fail("an enum's value case".to_string()) };
                let ty = self.w.sig_of(s).ret;
                self.enum_value_class(e, ty, Some(case), label);
            }
            Synth::ValueEquals(c) => self.equals_body(c, params[0], true),
            Synth::Field(c, i) => {
                let Some((name, _)) = self.case_fields(c).get(i as usize).cloned() else { return self.fail("a case class field".to_string()) };
                self.term_at(None);
                self.buf.byte(SELECT);
                let n = self.names.simple(&name);
                self.buf.nat(n as u64);
                self.synth_this(c);
            }
        }
    }

    fn open_tree(&mut self, tag: u8) -> crate::tasty::write::buf::Slot {
        self.buf.byte(tag);
        self.buf.begin_length()
    }

    /// The name of a class as its `productPrefix` and its companion's `toString` have it.
    fn name_literal(&mut self, c: ClassId) {
        let name = self.name(self.w.syms.class(c).name);
        self.string_literal(&name);
    }

    fn string_literal(&mut self, s: &str) {
        self.mark_tree();
        self.buf.byte(STRINGCONST);
        let n = self.names.simple(s);
        self.buf.nat(n as u64);
    }

    /// The fields a case class's synthesized members read: its first clause's, by name and
    /// type.
    pub(super) fn case_fields(&mut self, c: ClassId) -> Vec<(String, TypeId)> {
        self.w.complete_class(c);
        let info = self.class_info(c);
        let seq = self.w.b.seq;
        let field_type = |p: &mut Self, t: TypeId, repeated: bool| match seq {
            Some(s) if repeated => p.w.types.class(s, &[t]),
            _ => t,
        };
        info.ctor.first().map(|cl| cl.params.iter().map(|p| (self.name(p.name), field_type(self, p.ty, p.repeated))).collect()).unwrap_or_default()
    }

    /// `arg: _*`, a sequence passed to a repeated parameter of the element type `elem`.
    fn splice(&mut self, elem: TypeId, arg: impl FnOnce(&mut Self)) {
        self.term_at(None);
        let l = self.open_tree(TYPED);
        arg(self);
        self.repeated_tpt(elem);
        self.buf.end_length(l);
    }

    /// The parameters at `clause` as arguments, a repeated one's spliced.
    fn param_args(&mut self, clause: &[usize], repeated: &[Option<TypeId>]) {
        for (j, &a) in clause.iter().enumerate() {
            match repeated.get(j).copied().flatten() {
                Some(elem) => self.splice(elem, |p| p.param_ref(a)),
                None => self.param_ref(a),
            }
        }
    }

    /// The class applied to its own type parameters.
    fn synth_class_type(&mut self, c: ClassId) -> TypeId {
        let tps: Vec<TParamId> = self.w.syms.class(c).own_tparams().to_vec();
        let args: Vec<TypeId> = tps.iter().map(|&p| self.w.types.param(p)).collect();
        self.w.types.class(c, &args)
    }

    /// `C.this` of the class whose member is being written.
    fn synth_this(&mut self, c: ClassId) {
        self.qual_this(c);
    }

    fn param_ref(&mut self, addr: usize) {
        self.term_at(None);
        self.buf.byte(TERMREFDIRECT);
        self.buf.reference(addr);
    }

    /// `T @unchecked` as a type.
    fn unchecked_type(&mut self, t: TypeId) {
        self.buf.byte(ANNOTATEDTYPE);
        let l = self.buf.begin_length();
        self.ty(t);
        self.annotation_tree("scala", "unchecked");
        self.buf.end_length(l);
    }

    /// A method of a static object of scala-library as scalac's `ref` writes it, the term
    /// reference on the object class's `this`: `Statics.mix`.
    fn library_static(&mut self, pkg: &str, object: &str, name: &str, params: &[&str], result: &str) {
        self.term_at(None);
        self.buf.byte(TERMREF);
        let sig: Vec<SigParam> = params.iter().map(|p| SigParam::Type(p.to_string())).collect();
        let n = self.names.signed(name, None, &sig, result);
        self.buf.nat(n as u64);
        self.buf.byte(THIS);
        self.buf.byte(TYPEREF);
        let o = self.names.object_class(object);
        self.buf.nat(o as u64);
        self.package_path(pkg);
    }

    /// `Statics.f(args)`, the arguments written by `args`.
    fn statics_call(&mut self, name: &str, params: &[&str], args: impl FnOnce(&mut Self)) {
        self.term_at(None);
        let l = self.open_tree(APPLY);
        self.library_static("scala.runtime", "Statics", name, params, "scala.Int");
        args(self);
        self.buf.end_length(l);
    }

    fn int_literal(&mut self, i: i32) {
        self.mark_tree();
        self.buf.byte(INTCONST);
        self.buf.long_int(i as i64);
    }

    /// `this.x` of a case field, `ref(accessor)` of scalac.
    fn this_field(&mut self, c: ClassId, name: &str) {
        self.term_at(None);
        self.buf.byte(SELECT);
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        self.synth_this(c);
    }

    /// `hashCode` of a case class (`SyntheticMembers.chooseHashcode`).
    fn case_hash_code(&mut self, c: ClassId) {
        let fields = self.case_fields(c);
        let own = self.name(self.w.syms.class(c).name);
        let h = string_hash(&own);
        if fields.is_empty() {
            return self.int_literal(h);
        }
        let primitive = fields.iter().any(|&(_, t)| self.numeric_class(self.w.widen_lit(t)).is_some() || self.is_unit(t));
        if !primitive {
            // MurmurHash3.productHash(this, Statics.mix(0xcafebabe, "C".hashCode), true)
            self.term_at(None);
            let l = self.open_tree(APPLY);
            let sl = self.open_tree(SELECTIN);
            let sig = [SigParam::Type("scala.Product".into()), SigParam::Type("scala.Int".into()), SigParam::Type("scala.Boolean".into())];
            let n = self.names.signed("productHash", None, &sig, "scala.Int");
            self.buf.nat(n as u64);
            self.term_at(None);
            self.buf.byte(TERMREF);
            let m = self.names.simple("MurmurHash3");
            self.buf.nat(m as u64);
            self.package_path("scala.util.hashing");
            self.buf.byte(TYPEREF);
            let mc = self.names.object_class("MurmurHash3");
            self.buf.nat(mc as u64);
            self.package_path("scala.util.hashing");
            self.buf.end_length(sl);
            self.synth_this(c);
            self.int_literal(statics_mix(0xcafebabeu32 as i32, h));
            self.mark_tree();
            self.buf.byte(TRUECONST);
            self.buf.end_length(l);
            return;
        }
        // { var acc = 0xcafebabe; acc = mix(acc, "C".hashCode); acc = mix(acc, hash(x)); ..; finalizeHash(acc, n) }
        self.term_at(None);
        let b = self.open_tree(BLOCK);
        // The block's value first: it reads `acc`, defined after it, by a forward reference.
        self.term_at(None);
        let fl = self.open_tree(APPLY);
        self.library_static("scala.runtime", "Statics", "finalizeHash", &["scala.Int", "scala.Int"], "scala.Int");
        self.term_at(None);
        self.buf.byte(TERMREFDIRECT);
        let fwd = self.buf.forward_reference();
        self.int_literal(fields.len() as i32);
        self.buf.end_length(fl);
        // var acc: Int = 0xcafebabe
        let va = self.buf.addr();
        self.buf.fill(fwd, va);
        self.term_at(None);
        let vl = self.open_tree(VALDEF);
        let an = self.names.simple("acc");
        self.buf.nat(an as u64);
        let int = self.w.b.t_int;
        self.tpt(int);
        self.int_literal(0xcafebabeu32 as i32);
        self.write_flags(&[SYNTHETIC, MUTABLE]);
        self.buf.end_length(vl);
        let acc = va;
        let mix = |p: &mut Self, value: &mut dyn FnMut(&mut Self)| {
            p.term_at(None);
            let al = p.open_tree(ASSIGN);
            p.param_ref(acc);
            p.statics_call("mix", &["scala.Int", "scala.Int"], |p| {
                p.param_ref(acc);
                value(p);
            });
            p.buf.end_length(al);
        };
        mix(self, &mut |p| p.int_literal(h));
        for (name, t) in fields.clone() {
            mix(self, &mut |p| p.field_hash(c, &name, t));
        }
        self.buf.end_length(b);
    }

    /// A field's hash in `hashCode` (`SyntheticMembers.hashImpl`).
    fn field_hash(&mut self, c: ClassId, name: &str, t: TypeId) {
        let t = self.w.widen_lit(t);
        let b = &self.w.b;
        let class = match self.w.types.get(t) {
            Type::Class(k, _) => Some(k),
            _ => None,
        };
        let (int, boolean, short, byte, char, long, double, float, unit) = (b.int, b.boolean, b.short, b.byte, b.char, b.long, b.double, b.float, b.unit);
        match class {
            Some(k) if k == unit => self.int_literal(0),
            Some(k) if k == boolean => {
                self.term_at(None);
                let l = self.open_tree(IF);
                self.this_field(c, name);
                self.int_literal(1231);
                self.int_literal(1237);
                self.buf.end_length(l);
            }
            Some(k) if k == int => self.this_field(c, name),
            Some(k) if k == short || k == byte || k == char => {
                self.term_at(None);
                self.buf.byte(SELECT);
                let n = self.names.simple("toInt");
                self.buf.nat(n as u64);
                self.this_field(c, name);
            }
            Some(k) if k == long => self.statics_call("longHash", &["scala.Long"], |p| p.this_field(c, name)),
            Some(k) if k == double => self.statics_call("doubleHash", &["scala.Double"], |p| p.this_field(c, name)),
            Some(k) if k == float => self.statics_call("floatHash", &["scala.Float"], |p| p.this_field(c, name)),
            _ => self.statics_call("anyHash", &["java.lang.Object"], |p| p.this_field(c, name)),
        }
    }

    /// `a op b` of `Boolean`'s `||` or `&&`.
    fn boolean_op(&mut self, op: &str, a: &mut dyn FnMut(&mut Self), b: &mut dyn FnMut(&mut Self)) {
        self.term_at(None);
        let l = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed(op, None, &[SigParam::Type("scala.Boolean".into())], "scala.Boolean");
        self.buf.nat(n as u64);
        a(self);
        self.external_typeref("scala", "Boolean");
        self.buf.end_length(sl);
        b(self);
        self.buf.end_length(l);
    }

    /// The conjunction of `terms`, balanced as scalac's `reduceBalanced` makes it.
    fn and_balanced(&mut self, terms: &[Box<dyn Fn(&mut Self)>]) {
        match terms {
            [] => {
                self.mark_tree();
                self.buf.byte(TRUECONST);
            }
            [t] => t(self),
            _ => {
                let (l, r) = terms.split_at(terms.len() / 2);
                self.boolean_op("&&", &mut |p| p.and_balanced(l), &mut |p| p.and_balanced(r));
            }
        }
    }

    /// `this.x == x$0.x` of a field, `==` of the field's primitive class or of `Any`.
    fn field_equals(&mut self, c: ClassId, name: &str, t: TypeId, bind: usize) {
        let t = self.w.widen_lit(t);
        let primitive = self.numeric_class(t).map(|k| self.name(self.w.syms.class(k).name));
        self.term_at(None);
        let l = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let param = match &primitive {
            Some(k) => format!("scala.{}", k),
            None => "java.lang.Object".to_string(),
        };
        let n = self.names.signed("==", None, &[SigParam::Type(param)], "scala.Boolean");
        self.buf.nat(n as u64);
        self.this_field(c, name);
        match &primitive {
            Some(k) => self.external_typeref("scala", k),
            None => self.external_typeref("scala", "Any"),
        }
        self.buf.end_length(sl);
        self.term_at(None);
        self.buf.byte(SELECT);
        let f = self.names.simple(name);
        self.buf.nat(f as u64);
        self.param_ref(bind);
        self.buf.end_length(l);
    }

    /// `equals` of a case class (`SyntheticMembers.equalsBody`): `this.eq(that) || (that match
    /// { case x$0 @ (_: C @unchecked) => fields && x$0.canEqual(this); case _ => false })`.
    fn equals_body(&mut self, c: ClassId, that: usize, value_class: bool) {
        let mut fields = self.case_fields(c);
        // The primitive fields first.
        fields.sort_by_key(|&(_, t)| if self.numeric_class(self.w.widen_lit(t)).is_some() { 0 } else { 1 });
        if value_class {
            fields.truncate(1);
        }
        let final_class = value_class || self.w.syms.class(c).mods & mods::FINAL != 0;
        let class_ty = self.synth_class_type(c);
        let mut eq_compare = |p: &mut Self| {
            p.term_at(None);
            let l = p.open_tree(APPLY);
            let sl = p.open_tree(SELECTIN);
            let n = p.names.signed("eq", None, &[SigParam::Type("java.lang.Object".into())], "scala.Boolean");
            p.buf.nat(n as u64);
            p.synth_this(c);
            p.external_typeref("java.lang", "Object");
            p.buf.end_length(sl);
            p.term_at(None);
            let ta = p.open_tree(TYPEAPPLY);
            let cl = p.open_tree(SELECTIN);
            let m = p.names.signed("$asInstanceOf$", None, &[SigParam::Types(1)], "java.lang.Object");
            p.buf.nat(m as u64);
            p.param_ref(that);
            p.external_typeref("scala", "Any");
            p.buf.end_length(cl);
            p.external_tpt("java.lang", "Object");
            p.buf.end_length(ta);
            p.buf.end_length(l);
        };
        let mut match_expr = |p: &mut Self| {
            p.term_at(None);
            let m = p.open_tree(MATCH);
            p.param_ref(that);
            // case x$0 @ (_: C @unchecked) => ..
            p.term_at(None);
            let cd = p.open_tree(CASEDEF);
            let bind = p.buf.addr();
            p.term_at(None);
            let bl = p.open_tree(BIND);
            let xn = p.names.simple("x$0");
            p.buf.nat(xn as u64);
            p.ty(class_ty);
            p.term_at(None);
            let tl = p.open_tree(TYPED);
            p.term_at(None);
            p.buf.byte(IDENT);
            let us = p.names.simple("_");
            p.buf.nat(us as u64);
            let ut = p.buf.addr();
            p.unchecked_type(class_ty);
            p.mark_tree();
            p.shared_type_at(ut);
            p.buf.end_length(tl);
            p.write_flags(&[SYNTHETIC]);
            p.buf.end_length(bl);
            let terms: Vec<Box<dyn Fn(&mut Self)>> = fields.iter().cloned().map(|(name, t)| Box::new(move |p: &mut Self| p.field_equals(c, &name, t, bind)) as Box<dyn Fn(&mut Self)>).collect();
            if final_class {
                p.and_balanced(&terms);
            } else {
                // `.canEqual(this)` of the bound value, after the fields.
                p.boolean_op("&&", &mut |p| p.and_balanced(&terms), &mut |p| {
                    p.term_at(None);
                    let l = p.open_tree(APPLY);
                    let sl = p.open_tree(SELECTIN);
                    let n = p.names.signed("canEqual", None, &[SigParam::Type("java.lang.Object".into())], "scala.Boolean");
                    p.buf.nat(n as u64);
                    p.param_ref(bind);
                    p.external_typeref("scala", "Equals");
                    p.buf.end_length(sl);
                    p.synth_this(c);
                    p.buf.end_length(l);
                });
            }
            p.buf.end_length(cd);
            // case _ => false
            p.term_at(None);
            let dd = p.open_tree(CASEDEF);
            p.wildcard(ANY);
            p.mark_tree();
            p.buf.byte(FALSECONST);
            p.buf.end_length(dd);
            p.buf.end_length(m);
        };
        if value_class {
            return match_expr(self);
        }
        self.boolean_op("||", &mut eq_compare, &mut match_expr);
    }

    /// `this.x.hashCode()` of a value class's primitive field, `java.util.Objects.hashCode(this.x)`
    /// of a reference.
    fn value_hash_code(&mut self, c: ClassId) {
        let Some((name, t)) = self.case_fields(c).first().cloned() else { return self.fail("a value class's field".to_string()) };
        let t = self.w.widen_lit(t);
        match self.numeric_class(t) {
            Some(k) => {
                let kn = self.name(self.w.syms.class(k).name);
                self.term_at(None);
                let l = self.open_tree(APPLY);
                let sl = self.open_tree(SELECTIN);
                let n = self.names.signed("hashCode", None, &[], "scala.Int");
                self.buf.nat(n as u64);
                self.this_field(c, &name);
                self.external_typeref("scala", &kn);
                self.buf.end_length(sl);
                self.buf.end_length(l);
            }
            None => {
                self.term_at(None);
                let l = self.open_tree(APPLY);
                let sl = self.open_tree(SELECTIN);
                let n = self.names.signed("hashCode", None, &[SigParam::Type("java.lang.Object".into())], "scala.Int");
                self.buf.nat(n as u64);
                self.term_at(None);
                self.buf.byte(TERMREF);
                let o = self.names.simple("Objects");
                self.buf.nat(o as u64);
                self.package_path("java.util");
                self.buf.byte(TYPEREF);
                let oc = self.names.object_class("Objects");
                self.buf.nat(oc as u64);
                self.package_path("java.util");
                self.buf.end_length(sl);
                self.this_field(c, &name);
                self.buf.end_length(l);
            }
        }
    }

    /// `productElement` and `productElementName`: `n match { case i => this._{i+1} | "name";
    /// case _ => throw new IndexOutOfBoundsException(n.toString()) }`.
    fn product_match(&mut self, c: ClassId, n: usize, names: bool) {
        let fields = self.case_fields(c);
        self.term_at(None);
        let m = self.open_tree(MATCH);
        self.param_ref(n);
        for (i, (name, _)) in fields.iter().enumerate() {
            self.term_at(None);
            let cd = self.open_tree(CASEDEF);
            self.int_literal(i as i32);
            if names {
                self.mark_tree();
                self.buf.byte(STRINGCONST);
                let s = self.names.simple(name);
                self.buf.nat(s as u64);
            } else {
                self.this_field(c, &format!("_{}", i + 1));
            }
            self.buf.end_length(cd);
        }
        self.term_at(None);
        let dd = self.open_tree(CASEDEF);
        let int = self.w.b.t_int;
        self.wildcard(int);
        self.term_at(None);
        self.buf.byte(THROW);
        self.term_at(None);
        let al = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let k = self.names.signed("<init>", None, &[SigParam::Type("java.lang.String".into())], "java.lang.IndexOutOfBoundsException");
        self.buf.nat(k as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        self.external_tpt("java.lang", "IndexOutOfBoundsException");
        self.external_typeref("java.lang", "IndexOutOfBoundsException");
        self.buf.end_length(sl);
        self.term_at(None);
        let tl = self.open_tree(APPLY);
        let ts = self.open_tree(SELECTIN);
        let t = self.names.signed("toString", None, &[], "java.lang.String");
        self.buf.nat(t as u64);
        self.param_ref(n);
        self.external_typeref("scala", "Any");
        self.buf.end_length(ts);
        self.buf.end_length(tl);
        self.buf.end_length(al);
        self.buf.end_length(dd);
        self.buf.end_length(m);
    }
}

impl<'w, 'a> P<'w, 'a> {
    /// `new C[targs](args)` of the parameters at `args`, clause by clause, as `Desugar`'s
    /// `copy` and companion `apply` make it.
    pub(super) fn synth_new(&mut self, c: ClassId, targs: &[TypeId], args: &[Vec<usize>]) {
        self.w.complete_class(c);
        let ctor = self.w.syms.class(c).ctor.clone();
        let Some((params, result)) = self.w.pickled_ctor_signature(c, "") else { return self.fail("a constructor's signature".to_string()) };
        let params = super::term::sig_params(params);
        // The clauses as scalac's constructor has them (`normalizeIfConstructor`), a repeated
        // parameter's argument spliced.
        let mut clauses: Vec<Vec<usize>> = args.to_vec();
        let subst: Subst = self.w.syms.class(c).own_tparams().iter().copied().zip(targs.iter().copied()).collect();
        let mut repeated: Vec<Vec<Option<TypeId>>> = ctor.iter().map(|cl| cl.params.iter().map(|p| p.repeated.then(|| self.w.types.subst(p.ty, &subst))).collect()).collect();
        if ctor.is_empty() {
            clauses = vec![Vec::new()];
            repeated = vec![Vec::new()];
        } else if ctor[0].is_implicit {
            clauses.insert(0, Vec::new());
            repeated.insert(0, Vec::new());
        } else if ctor.iter().all(|c| c.is_using) {
            clauses.push(Vec::new());
            repeated.push(Vec::new());
        }
        self.term_at(None);
        let opens: Vec<crate::tasty::write::buf::Slot> = clauses.iter().map(|_| self.open_tree(APPLY)).collect();
        let ta = (!targs.is_empty()).then(|| self.open_tree(TYPEAPPLY));
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed("<init>", None, &params, &result);
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        let t = self.w.types.class(c, targs);
        self.tpt(t);
        self.class_typeref(c);
        self.buf.end_length(sl);
        if let Some(l) = ta {
            for &t in targs {
                self.tpt(t);
            }
            self.buf.end_length(l);
        }
        for (i, clause) in clauses.iter().enumerate() {
            let none = Vec::new();
            self.param_args(clause, repeated.get(i).unwrap_or(&none));
            self.buf.end_length(opens[clauses.len() - 1 - i]);
        }
    }

    /// The addresses of the parameters written since `pmark`, clause by clause as `clauses`
    /// sizes them.
    pub(super) fn param_addrs(&self, pmark: usize, clauses: &[ClauseSig]) -> Vec<Vec<usize>> {
        let mut seen: Vec<usize> = Vec::new();
        for &(_, a) in &self.params[pmark..] {
            if !seen.contains(&a) {
                seen.push(a);
            }
        }
        let mut out = Vec::new();
        let mut at = 0;
        for cl in clauses {
            out.push(seen[at.min(seen.len())..(at + cl.params.len()).min(seen.len())].to_vec());
            at += cl.params.len();
        }
        out
    }

    /// `fromProduct(x$0)`: `{ val x$1: T1 = x$0.productElement(0).$asInstanceOf$[T1]; ..; new
    /// C[Any..](x$1, ..) }`, the class's type parameters `Any`, an element of `Any` uncast.
    pub(super) fn from_product(&mut self, c: ClassId, product: usize) {
        if self.w.syms.class(c).ctor.len() > 1 {
            return self.fail("fromProduct of a class of several clauses".to_string());
        }
        let tps: Vec<TParamId> = self.w.syms.class(c).own_tparams().to_vec();
        let subst: Subst = tps.iter().map(|&tp| (tp, ANY)).collect();
        let targs: Vec<TypeId> = tps.iter().map(|_| ANY).collect();
        let fields: Vec<(String, TypeId)> = self.case_fields(c).into_iter().map(|(n, t)| (n, self.w.types.subst(t, &subst))).collect();
        let Some((params, result)) = self.w.pickled_ctor_signature(c, "") else { return self.fail("a constructor's signature".to_string()) };
        let params = super::term::sig_params(params);
        self.term_at(None);
        let b = self.open_tree(BLOCK);
        // The block's value, `new C(..)` of the vals after it, by forward references.
        let mut refs = Vec::new();
        self.term_at(None);
        let l = self.open_tree(APPLY);
        let ta = (!targs.is_empty()).then(|| self.open_tree(TYPEAPPLY));
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed("<init>", None, &params, &result);
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        let t = self.w.types.class(c, &targs);
        self.tpt(t);
        self.class_typeref(c);
        self.buf.end_length(sl);
        if let Some(ta) = ta {
            for &t in &targs {
                self.tpt(t);
            }
            self.buf.end_length(ta);
        }
        let repeated: Vec<Option<TypeId>> = self.w.syms.class(c).ctor.first().map(|cl| cl.params.iter().map(|p| p.repeated.then(|| self.w.types.subst(p.ty, &subst))).collect()).unwrap_or_default();
        for i in 0..fields.len() {
            let mut reference = |p: &mut Self| {
                p.term_at(None);
                p.buf.byte(TERMREFDIRECT);
                refs.push(p.buf.forward_reference());
            };
            match repeated.get(i).copied().flatten() {
                Some(elem) => self.splice(elem, reference),
                None => reference(self),
            }
        }
        self.buf.end_length(l);
        for (i, (name, t)) in fields.iter().enumerate() {
            let va = self.buf.addr();
            self.buf.fill(refs[i], va);
            self.term_at(None);
            let vl = self.open_tree(VALDEF);
            let vn = self.names.simple(&format!("{}$1", name));
            self.buf.nat(vn as u64);
            self.tpt(*t);
            let element = |p: &mut Self| {
                // x$0.productElement(i)
                p.term_at(None);
                let pa = p.open_tree(APPLY);
                let ps = p.open_tree(SELECTIN);
                let pe = p.names.signed("productElement", None, &[SigParam::Type("scala.Int".into())], "java.lang.Object");
                p.buf.nat(pe as u64);
                p.param_ref(product);
                p.external_typeref("scala", "Product");
                p.buf.end_length(ps);
                p.int_literal(i as i32);
                p.buf.end_length(pa);
            };
            if *t == ANY {
                element(self);
            } else {
                // .$asInstanceOf$[T]
                self.term_at(None);
                let ta = self.open_tree(TYPEAPPLY);
                let cs = self.open_tree(SELECTIN);
                let m = self.names.signed("$asInstanceOf$", None, &[SigParam::Types(1)], "java.lang.Object");
                self.buf.nat(m as u64);
                element(self);
                self.external_typeref("scala", "Any");
                self.buf.end_length(cs);
                self.tpt(*t);
                self.buf.end_length(ta);
            }
            self.write_flags(&[SYNTHETIC]);
            self.buf.end_length(vl);
        }
        self.buf.end_length(b);
    }

    /// `writeReplace()` of an object: `new ModuleSerializationProxy(classOf[X.type])`.
    pub(super) fn write_replace_body(&mut self, module_type: impl FnOnce(&mut Self)) {
        self.term_at(None);
        let l = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed("<init>", None, &[SigParam::Type("java.lang.Class".into())], "scala.runtime.ModuleSerializationProxy");
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        self.external_tpt("scala.runtime", "ModuleSerializationProxy");
        self.external_typeref("scala.runtime", "ModuleSerializationProxy");
        self.buf.end_length(sl);
        self.mark_tree();
        self.buf.byte(CLASSCONST);
        module_type(self);
        self.buf.end_length(l);
    }
}

impl<'w, 'a> P<'w, 'a> {
    /// The companion object being written, as `C.this` of its class.
    fn companion_this(&mut self) {
        let Some(&k) = self.enclosing.last() else { return self.fail("an enum's companion".to_string()) };
        let name = self.name(self.w.syms.class(k).name);
        self.term_at(None);
        self.buf.byte(QUALTHIS);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        let n = self.names.object_class(&name);
        self.buf.nat(n as u64);
        self.class_typeref(k);
    }

    /// A val of the companion being written, by name on its `this`.
    fn companion_member(&mut self, name: &str) {
        self.term_at(None);
        self.buf.byte(SELECT);
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        self.companion_this();
    }

    /// The value cases of an enum in their order.
    fn enum_values(&self, e: ClassId) -> Vec<ClassId> {
        self.w.syms.class(e).children.iter().copied().filter(|&k| self.w.syms.class(k).singleton.is_some()).collect()
    }

    fn enum_array_type(&mut self, e: ClassId) -> TypeId {
        let array = self.w.b.array;
        let t = self.w.types.class(e, &[]);
        self.w.types.class(array, &[t])
    }

    /// `$new(ordinal, "Name")` of a value case.
    fn enum_value(&mut self, e: ClassId, case: ClassId) {
        let ordinal = self.w.syms.class(case).ordinal as i32;
        let name = self.name(self.w.syms.class(case).name);
        let Some(&k) = self.enclosing.last() else { return self.fail("an enum's companion".to_string()) };
        let result = self.w.library_class_name(e, "");
        self.term_at(None);
        let l = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed("$new", None, &[SigParam::Type("scala.Int".into()), SigParam::Type("java.lang.String".into())], &result);
        self.buf.nat(n as u64);
        self.companion_this();
        self.class_typeref(k);
        self.buf.end_length(sl);
        self.int_literal(ordinal);
        self.mark_tree();
        self.buf.byte(STRINGCONST);
        let s = self.names.simple(&name);
        self.buf.nat(s as u64);
        self.buf.end_length(l);
    }

    /// `Array.apply[E](values*)(ClassTag.apply[E](classOf[E]))`.
    fn enum_array(&mut self, e: ClassId) {
        if !self.w.syms.class(e).own_tparams().is_empty() {
            return self.fail("the values of an enum of type parameters".to_string());
        }
        let et = self.w.types.class(e, &[]);
        let values: Vec<String> = self.enum_values(e).into_iter().map(|k| self.name(self.w.syms.class(k).name)).collect();
        self.term_at(None);
        let outer = self.open_tree(APPLY);
        self.term_at(None);
        let inner = self.open_tree(APPLY);
        self.term_at(None);
        let ta = self.open_tree(TYPEAPPLY);
        let sl = self.open_tree(SELECTIN);
        let sig = [SigParam::Types(1), SigParam::Type("scala.collection.immutable.Seq".into()), SigParam::Type("scala.reflect.ClassTag".into())];
        let n = self.names.signed("apply", None, &sig, "java.lang.Object");
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(TERMREF);
        let an = self.names.simple("Array");
        self.buf.nat(an as u64);
        self.package_path("scala");
        self.buf.byte(TYPEREF);
        let ac = self.names.object_class("Array");
        self.buf.nat(ac as u64);
        self.package_path("scala");
        self.buf.end_length(sl);
        self.tpt(et);
        self.buf.end_length(ta);
        // The values as a repeated argument.
        self.term_at(None);
        let tl = self.open_tree(TYPED);
        self.term_at(None);
        let r = self.open_tree(REPEATED);
        self.tpt(et);
        for v in &values {
            self.companion_member(v);
        }
        self.buf.end_length(r);
        self.mark_tree();
        self.buf.byte(APPLIEDTYPE);
        let rl = self.buf.begin_length();
        self.external_typeref("scala", "<repeated>");
        self.ty(et);
        self.buf.end_length(rl);
        self.buf.end_length(tl);
        self.buf.end_length(inner);
        // ClassTag.apply[E](classOf[E])
        self.term_at(None);
        let cl = self.open_tree(APPLY);
        self.term_at(None);
        let cta = self.open_tree(TYPEAPPLY);
        self.library_static("scala.reflect", "ClassTag", "apply", &[], "scala.reflect.ClassTag");
        self.tpt(et);
        self.buf.end_length(cta);
        self.term_at(None);
        let coa = self.open_tree(TYPEAPPLY);
        self.library_static("scala", "Predef", "classOf", &[], "java.lang.Class");
        self.tpt(et);
        self.buf.end_length(coa);
        self.buf.end_length(cl);
        self.buf.end_length(outer);
    }

    /// `new X("<message>" + arg)` thrown.
    fn throw_message(&mut self, pkg: &str, class: &str, message: &str, arg: &mut dyn FnMut(&mut Self)) {
        self.term_at(None);
        self.buf.byte(THROW);
        self.term_at(None);
        let al = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let k = self.names.signed("<init>", None, &[SigParam::Type("java.lang.String".into())], &format!("{}.{}", pkg, class));
        self.buf.nat(k as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        self.external_tpt(pkg, class);
        self.external_typeref(pkg, class);
        self.buf.end_length(sl);
        self.term_at(None);
        let pl = self.open_tree(APPLY);
        let ps = self.open_tree(SELECTIN);
        let plus = self.names.signed("+", None, &[SigParam::Type("java.lang.Object".into())], "java.lang.String");
        self.buf.nat(plus as u64);
        self.mark_tree();
        self.buf.byte(STRINGCONST);
        let m = self.names.simple(message);
        self.buf.nat(m as u64);
        self.external_typeref("java.lang", "String");
        self.buf.end_length(ps);
        arg(self);
        self.buf.end_length(pl);
        self.buf.end_length(al);
    }

    /// `valueOf($name)`: `$name match { case "V" => this.V; ..; case _ => throw .. }`.
    fn enum_value_of(&mut self, e: ClassId, name: usize) {
        let full = scala_name(self.w, e);
        let values: Vec<String> = self.enum_values(e).into_iter().map(|k| self.name(self.w.syms.class(k).name)).collect();
        self.term_at(None);
        let m = self.open_tree(MATCH);
        self.param_ref(name);
        for v in &values {
            self.term_at(None);
            let cd = self.open_tree(CASEDEF);
            self.mark_tree();
            self.buf.byte(STRINGCONST);
            let s = self.names.simple(v);
            self.buf.nat(s as u64);
            self.companion_member(v);
            self.buf.end_length(cd);
        }
        self.term_at(None);
        let dd = self.open_tree(CASEDEF);
        let string = self.w.b.t_string;
        self.wildcard(string);
        self.throw_message("java.lang", "IllegalArgumentException", &format!("enum {} has no case with name: ", full), &mut |p| p.param_ref(name));
        self.buf.end_length(dd);
        self.buf.end_length(m);
    }

    /// `fromOrdinal(ordinal)`: `try $values.apply(ordinal) catch { case _ => throw .. }` of an
    /// enum of values, `ordinal match { case i => V; ..; case _ => throw .. }` otherwise.
    fn from_ordinal(&mut self, e: ClassId, ordinal: usize) {
        let full = scala_name(self.w, e);
        let message = format!("enum {} has no case with ordinal: ", full);
        let all = self.w.syms.class(e).children.iter().all(|&c| self.w.syms.class(c).singleton.is_some()) && !self.w.syms.class(e).children.is_empty();
        let mut ordinal_string = |p: &mut Self| {
            p.term_at(None);
            let tl = p.open_tree(APPLY);
            let ts = p.open_tree(SELECTIN);
            let t = p.names.signed("toString", None, &[], "java.lang.String");
            p.buf.nat(t as u64);
            p.param_ref(ordinal);
            p.external_typeref("scala", "Any");
            p.buf.end_length(ts);
            p.buf.end_length(tl);
        };
        if all {
            let arr = self.enum_array_type(e);
            self.term_at(None);
            let t = self.open_tree(TRY);
            self.term_at(None);
            let l = self.open_tree(APPLY);
            let sl = self.open_tree(SELECTIN);
            let n = self.names.signed("apply", None, &[SigParam::Type("scala.Int".into())], "java.lang.Object");
            self.buf.nat(n as u64);
            self.companion_member("$values");
            self.ty(arr);
            self.buf.end_length(sl);
            self.param_ref(ordinal);
            self.buf.end_length(l);
            self.term_at(None);
            let cd = self.open_tree(CASEDEF);
            // `_: Throwable` by name: the std's class enters only where a program names it.
            self.term_at(None);
            self.buf.byte(IDENT);
            let us = self.names.simple("_");
            self.buf.nat(us as u64);
            self.external_typeref("java.lang", "Throwable");
            self.throw_message("java.util", "NoSuchElementException", &message, &mut ordinal_string);
            self.buf.end_length(cd);
            self.buf.end_length(t);
            return;
        }
        self.term_at(None);
        let m = self.open_tree(MATCH);
        self.param_ref(ordinal);
        for k in self.enum_values(e) {
            let ord = self.w.syms.class(k).ordinal as i32;
            let name = self.name(self.w.syms.class(k).name);
            self.term_at(None);
            let cd = self.open_tree(CASEDEF);
            self.int_literal(ord);
            self.companion_member(&name);
            self.buf.end_length(cd);
        }
        self.term_at(None);
        let dd = self.open_tree(CASEDEF);
        let int = self.w.b.t_int;
        self.wildcard(int);
        self.throw_message("java.util", "NoSuchElementException", &message, &mut ordinal_string);
        self.buf.end_length(dd);
        self.buf.end_length(m);
    }
}

impl<'w, 'a> P<'w, 'a> {
    /// Whether the package object of `pk` has the class `owner` among its bases.
    fn package_object_inherits(&mut self, pk: PkgId, owner: ClassId) -> bool {
        let Some(n) = self.w.interner.lookup("package") else { return false };
        let entry = self.w.syms.pkg(pk).entries.get(&n).cloned();
        let object = entry.as_ref().and_then(|e| e.term).and_then(|t| match self.w.syms.sym(t).kind {
            SymKind::Object(c) => Some(c),
            _ => None,
        });
        let Some(c) = entry.and_then(|e| e.class).or(object) else { return false };
        self.w.complete_class(c);
        self.w.syms.class(c).base_types.iter().any(|&(b, _)| b == owner)
    }

    /// An export forwarder's body: the exported member selected on the qualifier by the name the
    /// qualifier has it under and applied to the forwarder's type parameters and parameters
    /// (`Exp.f[T](a)`, `Exp.v`).
    pub(super) fn forwarder_body(&mut self, sym: SymId, q: crate::typer::exports::ExportQualifier, targs: &[TypeId], clauses: &[Vec<usize>], selected: Name) {
        use crate::typer::exports::ExportQualifier;
        let info = self.sym_info(sym);
        let sig = match self.member_signature(sym) {
            Ok(sig) => sig,
            Err(kind) => return self.fail(format!("the erasure of a {} in a signature", kind)),
        };
        let o = match q {
            ExportQualifier::Object(o) => Some(o),
            ExportQualifier::Package(_) => None,
        };
        if !self.std_member_shape(sym, &sig, o) {
            return;
        }
        if let (None, Some(what)) = (o, self.std_helper(sym)) {
            return self.fail(format!("the std's {}, which scala-library has under another shape", what));
        }
        // A class's member a package exports is its package object's, inherited; one the
        // package's own top-level export reaches is that export's forwarder, which the writer
        // does not state: the selection fails below, and the body that makes it is withheld.
        // Another module's top-level definition, which its loader enters as a member of its file's
        // `X$package` object.
        let file_holder = match info.owner {
            Owner::Class(owner) => {
                let k = self.w.syms.class(owner);
                let name = self.name(k.name);
                (k.kind == ClassKind::Object && name.ends_with("$package") && matches!(q, ExportQualifier::Package(pk) if k.owner == Owner::Package(pk))).then_some(name)
            }
            _ => None,
        };
        if let (ExportQualifier::Package(pk), Owner::Class(owner), None) = (q, info.owner, &file_holder) {
            if !self.package_object_inherits(pk, owner) {
                return self.fail("an export forwarder of a package's top-level export".to_string());
            }
        }
        // A package's member: a class's object on the package, a top-level definition on its
        // file's `X$package` object.
        let qualifier = |p: &mut Self| match (q, o) {
            (_, Some(o)) => {
                p.term_at(None);
                p.module_term(o);
            }
            (ExportQualifier::Package(pk), None) => {
                p.term_at(None);
                let holder = match p.w.syms.sym(sym).owner {
                    Owner::Class(_) if file_holder.is_some() => file_holder.clone().unwrap_or_default(),
                    // A member a package object inherits.
                    Owner::Class(_) => "package".to_string(),
                    _ if matches!(info.kind, SymKind::Def | SymKind::Val | SymKind::Var | SymKind::Given) => {
                        let file = p.w.syms.sym(sym).file;
                        format!("{}$package", super::super::file_stem(&p.w.files.as_slice()[file.0 as usize].path))
                    }
                    _ => return p.package_ref(pk),
                };
                p.buf.byte(SELECT);
                let n = p.names.simple(&holder);
                p.buf.nat(n as u64);
                p.term_at(None);
                p.package_ref(pk);
            }
            _ => {}
        };
        let name = self.name(selected);
        // The applications from the selection outwards: an extension's type parameters and
        // clauses before the method's own.
        let (et, ec) = if info.is_extension { ((info.ext_tparams as usize).min(targs.len()), (info.ext_clauses as usize).min(clauses.len())) } else { (targs.len(), 0) };
        if info.is_extension && name.ends_with(':') {
            return self.fail("an export forwarder of a right-associative extension".to_string());
        }
        enum Layer {
            Types(usize, usize),
            Clause(usize),
        }
        let mut layers = Vec::new();
        if et > 0 {
            layers.push(Layer::Types(0, et));
        }
        layers.extend((0..ec).map(Layer::Clause));
        if targs.len() > et {
            layers.push(Layer::Types(et, targs.len()));
        }
        layers.extend((ec..clauses.len()).map(Layer::Clause));
        self.term_at(None);
        let opens: Vec<crate::tasty::write::buf::Slot> = layers.iter().rev().map(|l| self.open_tree(if matches!(l, Layer::Types(..)) { TYPEAPPLY } else { APPLY })).collect();
        match &sig {
            None => {
                self.buf.byte(SELECT);
                let n = self.names.simple(&name);
                self.buf.nat(n as u64);
                qualifier(self);
            }
            Some((params, result)) => {
                let target = self.target_name(sym);
                let sl = self.open_tree(SELECTIN);
                let n = self.names.signed(&name, target.as_deref(), params, result);
                self.buf.nat(n as u64);
                qualifier(self);
                // A member the qualifier exports itself is selected as its forwarder there.
                let owner = if self.is_std_sym(sym) { o.and_then(|o| self.shape_owner(sym, Some(o))) } else { o.filter(|_| !self.qualifier_has(q, info.owner)) };
                match (owner, info.owner) {
                    (Some(c), _) | (None, Owner::Class(c)) => self.owner_class_ref(c),
                    (None, Owner::Package(_)) => self.package_object_class_ref(sym),
                    _ => return self.fail("an exported member of no class".to_string()),
                }
                self.buf.end_length(sl);
            }
        }
        let sig_clauses = self.w.sig_of(sym).clauses.clone();
        for (k, layer) in layers.iter().enumerate() {
            match *layer {
                Layer::Types(from, to) => {
                    for &t in &targs[from..to] {
                        self.tpt(t);
                    }
                }
                Layer::Clause(i) => {
                    let repeated: Vec<Option<TypeId>> = sig_clauses.get(i).map(|cl| cl.params.iter().map(|p| p.repeated.then_some(p.ty)).collect()).unwrap_or_default();
                    self.param_args(&clauses[i], &repeated);
                }
            }
            self.buf.end_length(opens[layers.len() - 1 - k]);
        }
    }
}

/// What an enum value's class answers for its name and its ordinal: `$new`'s parameters, or the
/// case's own.
enum EnumLabel {
    Params(usize, usize),
    Literal(i32, String),
}

impl<'w, 'a> P<'w, 'a> {
    /// `{ final class $anon extends P, scala.runtime.EnumValue, Mirror.Singleton { .. }; new
    /// $anon(): T & scala.runtime.EnumValue }`, `DesugarEnums.enumValueCreator` or
    /// `expandEnumModule` with the members `SyntheticMembers` gives an enum value's class: `P` the
    /// enum's constructor call, or the case's parent with what the case passes it.
    fn enum_value_class(&mut self, e: ClassId, et: TypeId, case: Option<ClassId>, label: EnumLabel) {
        if case.is_none() && !self.w.syms.class(e).own_tparams().is_empty() {
            return self.fail("the values of an enum of type parameters".to_string());
        }
        let Some(&k) = self.enclosing.last() else { return self.fail("an enum's companion".to_string()) };
        let local = format!("{}._$$anon", self.w.library_class_name(k, ""));
        self.term_at(None);
        let b = self.open_tree(BLOCK);
        // (new $anon(): E & EnumValue), the class by a forward reference.
        self.term_at(None);
        let tl = self.open_tree(TYPED);
        self.term_at(None);
        let al = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed("<init>", None, &[], &local);
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        self.mark_tree();
        self.buf.byte(IDENTTPT);
        let an = self.names.simple("$anon");
        self.buf.nat(an as u64);
        let class_ref = self.buf.addr();
        self.buf.byte(TYPEREFDIRECT);
        let fwd = self.buf.forward_reference();
        self.shared_type_at(class_ref);
        self.buf.end_length(sl);
        self.buf.end_length(al);
        self.mark_tree();
        self.buf.byte(ANDTYPE);
        let and = self.buf.begin_length();
        self.ty(et);
        self.external_typeref("scala.runtime", "EnumValue");
        self.buf.end_length(and);
        self.buf.end_length(tl);
        // final class $anon extends E, scala.runtime.EnumValue, Mirror.Singleton
        let ca = self.buf.addr();
        self.buf.fill(fwd, ca);
        self.term_at(None);
        let cl = self.open_tree(TYPEDEF);
        self.buf.nat(an as u64);
        let tm = self.open_tree(TEMPLATE);
        match case {
            Some(c) => {
                let parent = self.w.syms.class(c).parents.first().copied().unwrap_or(et);
                self.parent_call(c, parent);
            }
            None => self.parent_new(e),
        }
        self.external_tpt("scala.runtime", "EnumValue");
        self.mark_tree();
        self.mirror_parent("Singleton");
        // def <init>(): Unit
        let ia = self.buf.addr();
        self.buf.byte(DEFDEF);
        let il = self.buf.begin_length();
        let init = self.names.simple("<init>");
        self.buf.nat(init as u64);
        self.buf.byte(EMPTYCLAUSE);
        let unit = self.w.b.t_unit;
        self.tpt(unit);
        // Stable without arguments to the parent, as `Namer`'s `parentsKind` has a class of no
        // initialisation.
        let passes = case.and_then(|c| self.index.tclasses.get(&c)).and_then(|&i| self.w.prog.classes[i as usize].parent_args);
        let java = self.is_java_enum(e);
        if passes.map_or(true, |l| self.w.prog.expr_list(l).is_empty()) && !java {
            self.buf.byte(STABLE);
        }
        self.synthetic_position(ia);
        self.buf.end_length(il);
        let string = self.w.b.t_string;
        let int = self.w.b.t_int;
        let any_ref = self.w.b.t_any_ref;
        // An enum over `java.lang.Enum` keeps `name`, `ordinal`, `hashCode` and `toString` of
        // Java's class: its value's only member is `productPrefix`, its `name()`.
        if java {
            self.anon_member("productPrefix", false, string, &[OVERRIDE, SYNTHETIC], &mut |p| {
                p.term_at(None);
                let l = p.open_tree(APPLY);
                let s = p.open_tree(SELECTIN);
                let n = p.names.signed("name", None, &[], "java.lang.String");
                p.buf.nat(n as u64);
                p.term_at(None);
                p.buf.byte(QUALTHIS);
                p.mark_tree();
                p.buf.byte(IDENTTPT);
                p.buf.nat(an as u64);
                p.shared_type_at(class_ref);
                p.external_typeref("java.lang", "Enum");
                p.buf.end_length(s);
                p.buf.end_length(l);
            });
            self.buf.end_length(tm);
            self.write_flags(&[FINAL, SYNTHETIC]);
            self.buf.end_length(cl);
            self.buf.end_length(b);
            return;
        }
        // private def readResolve(): AnyRef = E.fromOrdinal(this.ordinal)
        self.anon_member("readResolve", true, any_ref, &[PRIVATE, SYNTHETIC], &mut |p| {
            p.term_at(None);
            let l = p.open_tree(APPLY);
            let s = p.open_tree(SELECTIN);
            let result = p.w.library_class_name(e, "");
            let fo = p.names.signed("fromOrdinal", None, &[SigParam::Type("scala.Int".into())], &result);
            p.buf.nat(fo as u64);
            p.term_at(None);
            p.module_term(k);
            p.owner_class_ref(k);
            p.buf.end_length(s);
            p.term_at(None);
            p.buf.byte(SELECT);
            let on = p.names.simple("ordinal");
            p.buf.nat(on as u64);
            p.term_at(None);
            p.buf.byte(QUALTHIS);
            p.mark_tree();
            p.buf.byte(IDENTTPT);
            p.buf.nat(an as u64);
            p.shared_type_at(class_ref);
            p.buf.end_length(l);
        });
        let name = |p: &mut Self| match &label {
            EnumLabel::Params(_, n) => p.param_ref(*n),
            EnumLabel::Literal(_, n) => p.string_literal(n),
        };
        let ordinal = |p: &mut Self| match &label {
            EnumLabel::Params(o, _) => p.param_ref(*o),
            EnumLabel::Literal(o, _) => p.int_literal(*o),
        };
        self.anon_member("productPrefix", false, string, &[OVERRIDE, SYNTHETIC], &mut |p| name(p));
        self.anon_member("toString", true, string, &[OVERRIDE, SYNTHETIC], &mut |p| name(p));
        self.anon_member("ordinal", false, int, &[OVERRIDE, SYNTHETIC], &mut |p| ordinal(p));
        self.anon_member("hashCode", true, int, &[OVERRIDE, SYNTHETIC], &mut |p| {
            p.term_at(None);
            let l = p.open_tree(APPLY);
            let s = p.open_tree(SELECTIN);
            let h = p.names.signed("hashCode", None, &[], "scala.Int");
            p.buf.nat(h as u64);
            name(p);
            p.external_typeref("java.lang", "String");
            p.buf.end_length(s);
            p.buf.end_length(l);
        });
        self.buf.end_length(tm);
        self.write_flags(&[FINAL, SYNTHETIC]);
        self.buf.end_length(cl);
        self.buf.end_length(b);
    }

    /// `new E()` as a parent of a class: the enum's constructor call.
    fn parent_new(&mut self, e: ClassId) {
        let Some((params, result)) = self.w.pickled_ctor_signature(e, "") else { return self.fail("an enum's constructor".to_string()) };
        let params = super::term::sig_params(params);
        if self.w.syms.class(e).ctor.iter().any(|cl| !cl.params.is_empty()) {
            return self.fail("an enum of constructor parameters".to_string());
        }
        self.term_at(None);
        let l = self.open_tree(APPLY);
        let sl = self.open_tree(SELECTIN);
        let n = self.names.signed("<init>", None, &params, &result);
        self.buf.nat(n as u64);
        self.term_at(None);
        self.buf.byte(NEW);
        let t = self.w.types.class(e, &[]);
        self.tpt(t);
        self.class_typeref(e);
        self.buf.end_length(sl);
        self.buf.end_length(l);
    }

    /// A member of a synthesized class: its result and body.
    fn anon_member(&mut self, name: &'static str, empty_clause: bool, ret: TypeId, flags: &[u8], body: &mut dyn FnMut(&mut Self)) {
        let addr = self.buf.addr();
        self.buf.byte(DEFDEF);
        let len = self.buf.begin_length();
        let n = self.names.simple(name);
        self.buf.nat(n as u64);
        if empty_clause {
            self.buf.byte(EMPTYCLAUSE);
        }
        self.tpt(ret);
        body(self);
        self.write_flags(flags);
        self.synthetic_position(addr);
        self.buf.end_length(len);
    }
}
