//! What the JavaScript runtime provides at run time, over interpreter values: string conversion,
//! cooperative equality and hashing, the members of `Product` by rule, sequence patterns and
//! `getClass`.

use super::value::*;
use super::*;
use crate::symbols::ClassKind;

/// The members of `Product` and `Equals` the rule's product functions read a class's definition of,
/// by the bit `Interp::product_defs` keeps; any other name is asked for at once.
const PRODUCT_DEFS: [(u16, crate::intern::Name); 6] = [
    (1, crate::names::PRODUCT_ITERATOR),
    (2, crate::names::PRODUCT_ARITY),
    (4, crate::names::PRODUCT_ELEMENT),
    (8, crate::names::PRODUCT_PREFIX),
    (16, crate::names::PRODUCT_ELEMENT_NAME),
    (32, crate::names::CAN_EQUAL_METHOD),
];
const PRODUCT_DEFS_KNOWN: u16 = 1 << 15;

fn product_def_bit(name: crate::intern::Name) -> u16 {
    PRODUCT_DEFS.iter().find(|&&(_, n)| n == name).map_or(u16::MAX, |&(bit, _)| bit)
}

impl<'a, 't> Interp<'a, 't> {
    pub(super) fn to_str(&mut self, v: &Value) -> R<String> {
        Ok(match v {
            Value::Unit | Value::Absent => "()".to_string(),
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Long(l) => l.to_string(),
            Value::Byte(b) => b.to_string(),
            Value::Short(s) => s.to_string(),
            Value::Char(c) => char_to_string(*c),
            Value::Double(d) => java_double(*d),
            Value::Float(f) => java_float(*f),
            Value::Str(s) => s.to_string(),
            Value::Obj(o) => {
                let class = o.class;
                let m = self.member(class, crate::names::TO_STRING);
                let r = self.invoke_member(v.clone(), m, crate::names::TO_STRING, Vec::new())?;
                match r {
                    Value::Str(s) => s.to_string(),
                    Value::Null => "null".to_string(),
                    other => self.to_str(&other)?,
                }
            }
            Value::Fun(_) => "<function>".to_string(),
            Value::Array(a) => {
                let items = a.borrow().clone();
                let mut out = String::from("Array(");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&self.to_str(item)?);
                }
                out.push(')');
                out
            }
            Value::Map(_) => "<map>".to_string(),
            Value::Trie(_) => "<trie>".to_string(),
            Value::Class(c) => crate::interp::value::class_text(&c.qname),
            Value::Match(m) => self.match_group_text(m, 0).unwrap_or_default(),
            Value::Tree(t) => self.show_tree(*t),
            Value::Type(t) => self.show_type_repr(*t),
            Value::Sym(s) => {
                let s = self.live_sym(*s)?;
                self.symbol_name(s)
            }
            Value::Pos(f, s, e) => self.show_position(*f, *s, *e),
            Value::Src(f) => self.typer.source(*f).path.clone(),
        })
    }

    /// `Object.toString`: the class name and the hexadecimal `hashCode`, the class's own where
    /// it defines one.
    pub(super) fn any_to_string(&mut self, v: &Value) -> String {
        match v {
            Value::Obj(o) => {
                let h = self.object_hash(v).unwrap_or_else(|_| self.identity_hash(o));
                format!("{}@{:x}", self.runtime_class_name(o.class), h)
            }
            other => format!("{}@{:x}", self.type_name(other), self.identity_hash_of(other)),
        }
    }

    /// `==`: cooperative equality between numbers and `equals` on objects.
    pub(super) fn equal(&mut self, a: &Value, b: &Value) -> R<bool> {
        if a.same(b) {
            return Ok(true);
        }
        Ok(match (a, b) {
            (Value::Null, _) | (_, Value::Null) => false,
            (Value::Obj(o), _) => {
                let class = o.class;
                let m = self.member(class, crate::names::EQUALS);
                match self.invoke_member(a.clone(), m, crate::names::EQUALS, vec![b.clone()])? {
                    Value::Bool(r) => r,
                    _ => false,
                }
            }
            (Value::Char(c), n) if n.is_integral() => Some(*c as i64) == n.as_i64(),
            (n, Value::Char(c)) if n.is_integral() => Some(*c as i64) == n.as_i64(),
            // A number against a `BigInt` or `BigDecimal`: the object compares.
            (n, Value::Obj(o)) if n.is_number() => {
                let m = self.member(o.class, crate::names::EQUALS);
                matches!(self.invoke_member(b.clone(), m, crate::names::EQUALS, vec![a.clone()])?, Value::Bool(true))
            }
            (Value::Class(x), Value::Class(y)) => x.qname == y.qname,
            _ => false,
        })
    }

    /// `x.equals(y)`, the method rather than `==`: a boxed number equals a box of its own class
    /// and value (a floating point's bits: a NaN equals itself, `-0.0` is not `0.0`), an object
    /// answers by its `equals`, called even on itself; a null receiver throws.
    pub(super) fn equals_method(&mut self, a: &Value, b: &Value) -> R<bool> {
        Ok(match (a, b) {
            (Value::Null, _) => return self.throw_named("NullPointerException", "Cannot invoke \"Object.equals(Object)\" because the value is null"),
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Long(x), Value::Long(y)) => x == y,
            (Value::Short(x), Value::Short(y)) => x == y,
            (Value::Byte(x), Value::Byte(y)) => x == y,
            (Value::Char(x), Value::Char(y)) => x == y,
            (Value::Double(x), Value::Double(y)) => x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()),
            (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()),
            (x, _) if x.is_number() || matches!(x, Value::Char(_)) => false,
            (Value::Obj(o), _) => {
                let m = self.member(o.class, crate::names::EQUALS);
                matches!(self.invoke_member(a.clone(), m, crate::names::EQUALS, vec![b.clone()])?, Value::Bool(true))
            }
            _ => self.equal(a, b)?,
        })
    }

    /// `##`, and the hash every collection and case class uses for its elements.
    pub(super) fn hash(&mut self, v: &Value) -> R<i32> {
        if let Some(h) = prim_hash(v) {
            return Ok(h);
        }
        self.object_hash(v)
    }

    /// `x.hashCode`, where the boxed classes of the JVM hash their bits.
    pub(super) fn hash_code(&mut self, v: &Value) -> R<i32> {
        if let Some(h) = prim_hash_code(v) {
            return Ok(h);
        }
        self.object_hash(v)
    }

    fn object_hash(&mut self, v: &Value) -> R<i32> {
        match v {
            Value::Obj(o) => {
                let class = o.class;
                let m = self.member(class, crate::names::HASH_CODE);
                match self.invoke_member(v.clone(), m, crate::names::HASH_CODE, Vec::new())? {
                    Value::Int(h) => Ok(h),
                    other => Ok(other.as_i32().unwrap_or(0)),
                }
            }
            Value::Class(c) => Ok(str_hash(&c.qname)),
            other => Ok(self.identity_hash_of(other)),
        }
    }

    // ---- products ----

    /// A case class's `toString` by rule, `ScalaRunTime._toString`: `productIterator.mkString(
    /// productPrefix + "(", ",", ")")`, each read from the fields where the class takes it by
    /// rule too; a case object's or an enum value's is its name.
    pub(super) fn case_to_string(&mut self, v: &Value) -> R<String> {
        let Value::Obj(o) = v else { return self.to_str(v) };
        if let Some(name) = &o.name {
            return Ok(name.to_string());
        }
        let Some((fields, display)) = self.case_info(o.class) else { return Ok(self.any_to_string(v)) };
        let info = self.syms().class(o.class);
        if info.kind == ClassKind::Object || info.local_module.is_some() || info.inner_object.is_some() {
            return Ok(display.to_string());
        }
        // A tuple, of no display name, renders as its own `toString` does.
        let mut out = if display.is_empty() { String::new() } else { self.product_prefix(v)? };
        out.push('(');
        let class = o.class;
        let defs = self.product_defs(class);
        if defs & product_def_bit(crate::names::PRODUCT_ITERATOR) != 0 {
            let iterator = self.call_by_name(v.clone(), "productIterator", Vec::new())?;
            let mut first = true;
            while self.call_by_name(iterator.clone(), "hasNext", Vec::new())?.as_bool().unwrap_or(false) {
                let x = self.call_by_name(iterator.clone(), "next", Vec::new())?;
                if !std::mem::replace(&mut first, false) {
                    out.push(',');
                }
                out.push_str(&self.to_str(&x)?);
            }
        } else if defs & (product_def_bit(crate::names::PRODUCT_ARITY) | product_def_bit(crate::names::PRODUCT_ELEMENT)) != 0 {
            let arity = self.product_arity(v)?;
            for i in 0..arity {
                if i > 0 {
                    out.push(',');
                }
                let x = self.product_element(v, i)?;
                out.push_str(&self.to_str(&x)?);
            }
        } else {
            for (i, &key) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let field = self.read_slot(v, key);
                out.push_str(&self.to_str(&field)?);
            }
        }
        out.push(')');
        Ok(out)
    }

    /// The definition of `name` an instance of `c` has in the program or a library, which wins
    /// over what the rule gives a product (dotty's `existingDef`): not `Product`'s or `Equals`'s
    /// own, nor a template without a body.
    pub(super) fn defined_member(&mut self, c: ClassId, name: crate::intern::Name) -> Option<Member> {
        if self.product_defs(c) & product_def_bit(name) == 0 {
            return None;
        }
        self.defined_member_now(c, name)
    }

    /// Which of the members `PRODUCT_DEFS` names `c` defines (`defined_member_now`), by bit,
    /// computed once per class: the rule's `toString`, `equals` and `hashCode` ask for them at
    /// every call.
    pub(super) fn product_defs(&mut self, c: ClassId) -> u16 {
        let known = self.product_defs[c.idx()];
        if known != 0 {
            return known;
        }
        let mut bits = PRODUCT_DEFS_KNOWN;
        for (bit, name) in PRODUCT_DEFS {
            if self.defined_member_now(c, name).is_some() {
                bits |= bit;
            }
        }
        self.product_defs[c.idx()] = bits;
        bits
    }

    fn defined_member_now(&mut self, c: ClassId, name: crate::intern::Name) -> Option<Member> {
        let (product, equals) = (self.typer.b.product, self.typer.equals_class());
        match self.member(c, name) {
            Member::Fun(_, k) if Some(k) == product || Some(k) == equals => None,
            m @ (Member::Fun(..) | Member::Field(_) | Member::Lazy(..)) => Some(m),
            _ => None,
        }
    }

    /// `p.canEqual(that)`: the class's definition, else what the rule gives a case class or case
    /// object (`that.isInstanceOf[C]`) and `EnumValue` an enum value (`this eq that`); a product
    /// of no shape compares classes.
    pub(super) fn can_equal(&mut self, p: &Value, that: &Value) -> R<bool> {
        let Value::Obj(o) = p else { return Ok(false) };
        let name = crate::names::CAN_EQUAL_METHOD;
        if let Some(m) = self.defined_member(o.class, name) {
            return Ok(self.invoke_member(p.clone(), m, name, vec![that.clone()])?.as_bool().unwrap_or(false));
        }
        if o.name.is_some() {
            return Ok(p.same(that));
        }
        let Value::Obj(t) = that else { return Ok(false) };
        Ok(match self.case_class_of(o.class) {
            Some(c) => self.is_subclass(t.class, c),
            None => o.class == t.class,
        })
    }

    pub(super) fn case_equals(&mut self, a: &Value, b: &Value) -> R<bool> {
        if a.same(b) {
            return Ok(true);
        }
        let (Value::Obj(x), Value::Obj(y)) = (a, b) else { return Ok(false) };
        // A case object equals itself alone: one nested in a class has an instance per
        // enclosing instance, and two of them are not equal.
        if matches!(self.shape_ref(x.class), Shape::CaseObject(_)) {
            return Ok(false);
        }
        // The fields in `SyntheticMembers.equalsBody`'s order, the primitive ones first.
        let Some(fields) = self.case_compared_fields(x.class) else { return Ok(false) };
        // An instance of a subclass may equal one of the case class itself, as under scalac,
        // when the case class defines no equals of its own.
        let related = if x.class == y.class {
            true
        } else {
            let case_class = self.case_class_of(x.class);
            case_class.map_or(false, |c| self.is_subclass(y.class, c) && !self.syms().class(c).subclasses.is_empty())
        };
        if !related {
            return Ok(false);
        }
        for &key in fields.iter() {
            let (p, q) = (self.read_slot(a, key), self.read_slot(b, key));
            if !self.equal(&p, &q)? {
                return Ok(false);
            }
        }
        // `SyntheticMembers.equalsBody`: `that.canEqual(this)` after the fields.
        self.can_equal(b, a)
    }

    fn case_class_of(&mut self, c: ClassId) -> Option<ClassId> {
        let mut at = Some(c);
        let mut steps = 0;
        while let Some(k) = at {
            if matches!(self.shape_ref(k), Shape::Case(..) | Shape::CaseObject(_)) {
                return Some(k);
            }
            steps += 1;
            if steps > self.syms().classes.len() {
                break;
            }
            at = self.syms().class(k).superclass;
        }
        None
    }

    /// MurmurHash3's `productHash`; an enum value hashes as its name, a tuple under `TupleN`.
    pub(super) fn case_hash(&mut self, v: &Value) -> R<i32> {
        let Value::Obj(o) = v else { return self.hash(v) };
        if let Some(name) = &o.name {
            return Ok(str_hash(name));
        }
        let Some((fields, display)) = self.case_info(o.class) else { return Ok(self.identity_hash(o)) };
        let prefix = if display.is_empty() { format!("Tuple{}", fields.len()) } else { display.to_string() };
        if fields.is_empty() {
            return Ok(str_hash(&prefix));
        }
        let mut h = mix(PRODUCT_SEED, str_hash(&prefix));
        // dotty's `SyntheticMembers.chooseHashcode`: reference fields hash through `productArity`
        // and `productElement` (`MurmurHash3.productHash`), which a class may define.
        let class = o.class;
        let defs = self.product_defs(class);
        let reads_elements = defs & (product_def_bit(crate::names::PRODUCT_ARITY) | product_def_bit(crate::names::PRODUCT_ELEMENT)) != 0;
        if reads_elements && self.case_class_of(class).is_some_and(|c| crate::tir::hashes_through_elements(&self.typer.syms, &self.typer.types, c)) {
            let n = self.product_arity(v)?;
            if n == 0 {
                return Ok(h);
            }
            for i in 0..n {
                let x = self.product_element(v, i)?;
                let k = self.hash(&x)?;
                h = mix(h, k);
            }
            return Ok(finalize_hash(h, n));
        }
        for &key in fields.iter() {
            let field = self.read_slot(v, key);
            let k = self.hash(&field)?;
            h = mix(h, k);
        }
        Ok(finalize_hash(h, fields.len() as i32))
    }

    /// `productArity`: the class's definition, else the case fields' count by rule.
    pub(super) fn product_arity(&mut self, v: &Value) -> R<i32> {
        let Value::Obj(o) = v else { return Ok(0) };
        let name = crate::names::PRODUCT_ARITY;
        if let Some(m) = self.defined_member(o.class, name) {
            return Ok(self.invoke_member(v.clone(), m, name, Vec::new())?.as_i32().unwrap_or(0));
        }
        Ok(self.case_info(o.class).map_or(0, |(fields, _)| fields.len() as i32))
    }

    /// `productElement(i)`: the class's definition, else the case field by rule.
    pub(super) fn product_element(&mut self, v: &Value, i: i32) -> R {
        let Value::Obj(o) = v else { return self.throw_named("IndexOutOfBoundsException", &i.to_string()) };
        let name = crate::names::PRODUCT_ELEMENT;
        if let Some(m) = self.defined_member(o.class, name) {
            return self.invoke_member(v.clone(), m, name, vec![Value::Int(i)]);
        }
        let Some((fields, _)) = self.case_info(o.class) else { return self.throw_named("IndexOutOfBoundsException", &i.to_string()) };
        if i < 0 || i as usize >= fields.len() {
            return self.throw_named("IndexOutOfBoundsException", &i.to_string());
        }
        let x = self.read_slot(v, fields[i as usize]);
        Ok(if let Value::Absent = x { Value::Null } else { x })
    }

    /// A case class's field name, a class's own `productElementName`, or the empty name of any
    /// other product, as the runtime's `$productElementName` answers.
    pub(super) fn product_element_name(&mut self, v: &Value, i: i32) -> R {
        let Value::Obj(o) = v else { return self.throw_named("IndexOutOfBoundsException", &i.to_string()) };
        let name = crate::names::PRODUCT_ELEMENT_NAME;
        if let Some(m) = self.defined_member(o.class, name) {
            return self.invoke_member(v.clone(), m, name, vec![Value::Int(i)]);
        }
        let mut at = Some(o.class);
        let mut steps = 0;
        while let Some(k) = at {
            match self.shape(k) {
                Shape::Case(..) => {
                    let names: Vec<crate::intern::Name> = self.syms().class(k).ctor.first().map(|c| c.params.iter().map(|p| p.name).collect()).unwrap_or_default();
                    if i < 0 || i as usize >= names.len() {
                        return self.throw_named("IndexOutOfBoundsException", &i.to_string());
                    }
                    return Ok(Value::string(self.name(names[i as usize]).to_string()));
                }
                Shape::CaseObject(_) => return self.throw_named("IndexOutOfBoundsException", &i.to_string()),
                _ => {}
            }
            steps += 1;
            if steps > self.syms().classes.len() {
                break;
            }
            at = self.syms().class(k).superclass;
        }
        let name = self.typer.interner.intern("productElementName");
        match self.member(o.class, name) {
            Member::Missing | Member::Builtin(..) => {
                let arity = self.product_arity(v)?;
                if i < 0 || i >= arity {
                    return self.throw_named("IndexOutOfBoundsException", &format!("{} is out of bounds (min 0, max {})", i, arity - 1));
                }
                Ok(Value::string(String::new()))
            }
            m => self.invoke_member(v.clone(), m, name, vec![Value::Int(i)]),
        }
    }

    /// A member of an object by its name, as a reflective structural call reaches it.
    pub(super) fn reflective_member(&mut self, recv: Value, name: &str, args: Vec<Value>) -> R {
        let missing = |it: &mut Self| it.throw_named("RuntimeException", &format!("{} is not a member", name));
        let Value::Obj(o) = &recv else { return missing(self) };
        let n = self.typer.interner.intern(name);
        match self.member(o.class, n) {
            Member::Missing => missing(self),
            m => self.invoke_member(recv.clone(), m, n, args),
        }
    }

    /// The method a reflective call by signature reaches: by the first of the names it goes by
    /// that the object's class has (`typer::reflective_names`), as the output's call tries them.
    pub(super) fn reflective_call(&mut self, recv: Value, names: &[String], args: Vec<Value>) -> R {
        if let Value::Obj(o) = &recv {
            let class = o.class;
            for name in names {
                let Some(n) = self.typer.interner.lookup(name) else { continue };
                match self.member(class, n) {
                    Member::Missing => {}
                    m => return self.invoke_member(recv.clone(), m, n, args),
                }
            }
        }
        let name = names.last().map_or("", |n| n.as_str());
        self.throw_named("RuntimeException", &format!("{} is not a member", name))
    }

    /// `productPrefix`: the class's definition, else an enum value's name or the class's.
    pub(super) fn product_prefix(&mut self, v: &Value) -> R<String> {
        let Value::Obj(o) = v else { return Ok(String::new()) };
        let name = crate::names::PRODUCT_PREFIX;
        if let Some(m) = self.defined_member(o.class, name) {
            let prefix = self.invoke_member(v.clone(), m, name, Vec::new())?;
            return self.to_str(&prefix);
        }
        if let Some(name) = &o.name {
            return Ok(name.to_string());
        }
        match self.case_info(o.class) {
            Some((fields, display)) if display.is_empty() => Ok(format!("Tuple{}", fields.len())),
            Some((_, display)) => Ok(display.to_string()),
            None => Ok(self.name(self.syms().class(o.class).name).to_string()),
        }
    }

    // ---- sequences ----

    /// The first `n` elements of a sequence, with the rest of it behind them when `rest` is
    /// set; `None` when the sequence is shorter, or longer without `rest`.
    pub(super) fn seq_pattern(&mut self, v: &Value, n: usize, rest: bool) -> R<Option<Vec<Value>>> {
        if let Value::Array(a) = v {
            let items = a.borrow().clone();
            if if rest { items.len() < n } else { items.len() != n } {
                return Ok(None);
            }
            let mut out: Vec<Value> = items[..n].to_vec();
            if rest {
                out.push(self.array_seq_of(Value::array(items[n..].to_vec()))?);
            }
            return Ok(Some(out));
        }
        if !matches!(v, Value::Obj(_)) {
            return Ok(None);
        }
        let iterator = self.call_by_name(v.clone(), "iterator", Vec::new())?;
        let mut out = Vec::with_capacity(n + 1);
        for _ in 0..n {
            if !self.call_by_name(iterator.clone(), "hasNext", Vec::new())?.as_bool().unwrap_or(false) {
                return Ok(None);
            }
            out.push(self.call_by_name(iterator.clone(), "next", Vec::new())?);
        }
        if rest {
            out.push(self.call_by_name(v.clone(), "drop", vec![Value::Int(n as i32)])?);
        } else if self.call_by_name(iterator, "hasNext", Vec::new())?.as_bool().unwrap_or(false) {
            return Ok(None);
        }
        Ok(Some(out))
    }

    /// Calls the member of the receiver by its source name.
    pub(super) fn call_by_name(&mut self, recv: Value, name: &'static str, args: Vec<Value>) -> R {
        let n = match self.names.get(name) {
            Some(&n) => n,
            None => {
                let n = self.typer.interner.intern(name);
                self.names.insert(name, n);
                n
            }
        };
        match &recv {
            Value::Obj(o) => {
                let class = o.class;
                let m = self.member(class, n);
                self.invoke_member(recv, m, n, args)
            }
            _ => {
                let s = self.find_member_sym(&recv, n);
                match s {
                    Some(s) => self.invoke(recv, s, args),
                    None => {
                        let kind = self.type_name(&recv);
                        self.unsupported(format!("{} has no member {}", kind, name))
                    }
                }
            }
        }
    }

    /// Calls the member of the receiver named at run time, as `Method.invoke` does.
    pub(super) fn call_named(&mut self, recv: Value, name: Name, args: Vec<Value>) -> R {
        match &recv {
            Value::Obj(o) => {
                let class = o.class;
                let m = self.member(class, name);
                self.invoke_member(recv, m, name, args)
            }
            _ => match self.find_member_sym(&recv, name) {
                Some(s) => self.invoke(recv, s, args),
                None => {
                    let kind = self.type_name(&recv);
                    let n = self.name(name).to_string();
                    self.throw_named("NoSuchMethodException", &format!("{}.{}()", kind, n))
                }
            },
        }
    }

    fn find_member_sym(&mut self, v: &Value, name: Name) -> Option<SymId> {
        match v {
            Value::Fun(_) => {
                let pf = self.prog().partial_function?;
                self.syms().class(pf).members.get(&name).copied()
            }
            _ => None,
        }
    }

    /// The elements of an `Array`, a sequence of the standard library or a `Map`, in order.
    pub(super) fn elements_of(&mut self, v: &Value) -> R<Vec<Value>> {
        match v {
            Value::Array(a) => Ok(a.borrow().clone()),
            Value::Obj(_) => {
                if let Some(items) = self.elements_fast(v)? {
                    return Ok(items);
                }
                let iterator = self.call_by_name(v.clone(), "iterator", Vec::new())?;
                let mut out = Vec::new();
                while self.call_by_name(iterator.clone(), "hasNext", Vec::new())?.as_bool().unwrap_or(false) {
                    out.push(self.call_by_name(iterator.clone(), "next", Vec::new())?);
                    if out.len() > 100_000_000 {
                        return self.unsupported("a sequence without an end");
                    }
                }
                Ok(out)
            }
            Value::Null => self.throw_named("NullPointerException", "Cannot iterate over null"),
            other => {
                let kind = self.type_name(other);
                self.unsupported(format!("{} is no sequence", kind))
            }
        }
    }

    // ---- classes ----

    /// `classOf[C]`: the class value named as the JVM names it.
    pub(super) fn class_value_of(&mut self, c: ClassId) -> Value {
        let qname: Rc<str> = match crate::emit::class_of_name(self.syms(), self.typer.interner, c) {
            Some(n) => Rc::from(n),
            None => self.runtime_class_name(c),
        };
        if let Some(v) = self.class_values.get(&qname) {
            return Value::Class(v.clone());
        }
        let v = Rc::new(ClassValue { class: Some(c), qname: qname.clone() });
        self.class_values.insert(qname, v.clone());
        Value::Class(v)
    }

    pub(super) fn class_value_named(&mut self, qname: &str) -> Value {
        if let Some(c) = self.class_values.get(qname) {
            return Value::Class(c.clone());
        }
        let qname: Rc<str> = Rc::from(qname);
        let c = Rc::new(ClassValue { class: None, qname: qname.clone() });
        self.class_values.insert(qname, c.clone());
        Value::Class(c)
    }

    pub(super) fn class_of_value(&mut self, v: &Value) -> Value {
        let (qname, class): (Rc<str>, Option<ClassId>) = match v {
            Value::Int(_) => (Rc::from("java.lang.Integer"), None),
            Value::Long(_) => (Rc::from("java.lang.Long"), None),
            Value::Double(_) => (Rc::from("java.lang.Double"), None),
            Value::Float(_) => (Rc::from("java.lang.Float"), None),
            Value::Byte(_) => (Rc::from("java.lang.Byte"), None),
            Value::Short(_) => (Rc::from("java.lang.Short"), None),
            Value::Char(_) => (Rc::from("java.lang.Character"), None),
            Value::Bool(_) => (Rc::from("java.lang.Boolean"), None),
            Value::Str(_) => (Rc::from("java.lang.String"), None),
            Value::Unit | Value::Absent => (Rc::from("scala.runtime.BoxedUnit"), None),
            Value::Null => (Rc::from("null"), None),
            Value::Fun(_) => (Rc::from("scala.Function1"), None),
            Value::Array(_) => (Rc::from("java.util.ArrayList"), None),
            Value::Map(_) => (Rc::from("java.util.LinkedHashMap"), None),
            Value::Trie(_) => (Rc::from("scala.TrieNode"), None),
            Value::Class(_) => (Rc::from("java.lang.Class"), None),
            Value::Match(_) => (Rc::from("java.util.regex.MatchResult"), None),
            Value::Tree(_) => (Rc::from("scala.quoted.Expr"), None),
            Value::Type(_) => (Rc::from("scala.quoted.Type"), None),
            Value::Sym(_) => (Rc::from("scala.quoted.Quotes.reflectModule.Symbol"), None),
            Value::Pos(..) => (Rc::from("scala.quoted.Quotes.reflectModule.Position"), None),
            Value::Src(_) => (Rc::from("scala.quoted.Quotes.reflectModule.SourceFile"), None),
            // A builtin's instance (`new Object()`) is named as its `classOf` is.
            Value::Obj(o) => match crate::emit::class_of_name(self.syms(), self.typer.interner, o.class) {
                Some(n) => (Rc::from(n), Some(o.class)),
                None => (self.runtime_class_name(o.class), Some(o.class)),
            },
        };
        if let Some(c) = self.class_values.get(&qname) {
            return Value::Class(c.clone());
        }
        let c = Rc::new(ClassValue { class, qname: qname.clone() });
        self.class_values.insert(qname, c.clone());
        Value::Class(c)
    }

    /// The simple name of a class value, as `getSimpleName` gives it.
    pub(super) fn simple_class_name(qname: &str) -> String {
        let last = qname.rsplit('.').next().unwrap_or(qname);
        let last = last.rsplit('$').filter(|s| !s.is_empty()).next().unwrap_or(last);
        last.to_string()
    }

    pub(super) fn match_group_text(&self, m: &MatchValue, i: usize) -> Option<String> {
        let (a, b) = (*m.groups.get(i)?)?;
        Some(m.slice(a, b).into_owned())
    }

    pub(super) fn match_group(&self, m: &MatchValue, i: usize) -> Value {
        match self.match_group_text(m, i) {
            Some(s) => Value::string(s),
            None => Value::Null,
        }
    }
}
