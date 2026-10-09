//! What the JavaScript runtime provides at run time, over interpreter values: string conversion,
//! cooperative equality and hashing, the members of `Product` by rule, sequence patterns and
//! `getClass`.

use super::value::*;
use super::*;
use crate::symbols::ClassKind;

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
        let mut out = display.to_string();
        out.push('(');
        for (i, &key) in fields.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let field = self.read_slot(v, key);
            out.push_str(&self.to_str(&field)?);
        }
        out.push(')');
        Ok(out)
    }

    pub(super) fn case_equals(&mut self, a: &Value, b: &Value) -> R<bool> {
        if a.same(b) {
            return Ok(true);
        }
        let (Value::Obj(x), Value::Obj(y)) = (a, b) else { return Ok(false) };
        // A case object equals itself alone: one nested in a class has an instance per
        // enclosing instance, and two of them are not equal.
        if matches!(self.shape(x.class), Shape::CaseObject(_)) {
            return Ok(false);
        }
        let Some((fields, _)) = self.case_info(x.class) else { return Ok(false) };
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
        Ok(true)
    }

    fn case_class_of(&mut self, c: ClassId) -> Option<ClassId> {
        let mut at = Some(c);
        let mut steps = 0;
        while let Some(k) = at {
            if matches!(self.shape(k), Shape::Case(..) | Shape::CaseObject(_)) {
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
        for &key in fields.iter() {
            let field = self.read_slot(v, key);
            let k = self.hash(&field)?;
            h = mix(h, k);
        }
        Ok(finalize_hash(h, fields.len() as i32))
    }

    pub(super) fn product_arity(&mut self, v: &Value) -> R<i32> {
        let Value::Obj(o) = v else { return Ok(0) };
        if let Some((fields, _)) = self.case_info(o.class) {
            return Ok(fields.len() as i32);
        }
        let name = self.typer.interner.intern("productArity");
        let m = self.member(o.class, name);
        match m {
            Member::Missing => Ok(0),
            m => Ok(self.invoke_member(v.clone(), m, name, Vec::new())?.as_i32().unwrap_or(0)),
        }
    }

    pub(super) fn product_element(&mut self, v: &Value, i: i32) -> R {
        let Value::Obj(o) = v else { return self.throw_named("IndexOutOfBoundsException", &i.to_string()) };
        if let Some((fields, _)) = self.case_info(o.class) {
            // A body of `productElement` the class has, an override of the case class's, answers
            // as the JVM dispatches it; the case fields stand for the synthesized one alone.
            let name = self.typer.interner.intern("productElement");
            if let m @ Member::Fun(..) = self.member(o.class, name) {
                return self.invoke_member(v.clone(), m, name, vec![Value::Int(i)]);
            }
            if i < 0 || i as usize >= fields.len() {
                return self.throw_named("IndexOutOfBoundsException", &i.to_string());
            }
            let x = self.read_slot(v, fields[i as usize]);
            return Ok(if let Value::Absent = x { Value::Null } else { x });
        }
        let name = self.typer.interner.intern("productElement");
        let m = self.member(o.class, name);
        match m {
            Member::Missing => self.throw_named("IndexOutOfBoundsException", &i.to_string()),
            m => self.invoke_member(v.clone(), m, name, vec![Value::Int(i)]),
        }
    }

    /// A case class's field name, a class's own `productElementName`, or the empty name of any
    /// other product, as the runtime's `$productElementName` answers.
    pub(super) fn product_element_name(&mut self, v: &Value, i: i32) -> R {
        let Value::Obj(o) = v else { return self.throw_named("IndexOutOfBoundsException", &i.to_string()) };
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

    pub(super) fn product_prefix(&mut self, v: &Value) -> R<String> {
        let Value::Obj(o) = v else { return Ok(String::new()) };
        if let Some(name) = &o.name {
            return Ok(name.to_string());
        }
        match self.case_info(o.class) {
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
            Value::Obj(o) => (self.runtime_class_name(o.class), Some(o.class)),
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
