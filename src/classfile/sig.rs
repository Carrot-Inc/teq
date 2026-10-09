//! The generic signatures of JVMS 4.7.9.1 and the descriptors of 4.3, which are a subset of
//! them: a descriptor has no type arguments, type variables or formal type parameters. One
//! parser reads both, and a descriptor stands in where a `Signature` attribute is absent.

/// Nesting of type arguments and array dimensions the parser follows.
const MAX_DEPTH: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
}

impl Prim {
    pub fn scala_name(self) -> &'static str {
        match self {
            Prim::Byte => "Byte",
            Prim::Char => "Char",
            Prim::Double => "Double",
            Prim::Float => "Float",
            Prim::Int => "Int",
            Prim::Long => "Long",
            Prim::Short => "Short",
            Prim::Boolean => "Boolean",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeArg {
    /// `?`
    Wild,
    /// `? extends T`
    Extends(JType),
    /// `? super T`
    Super(JType),
    Exact(JType),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassType {
    /// Binary name, `java/util/Map$Entry`.
    pub name: String,
    pub args: Vec<TypeArg>,
    /// For `Outer<A>.Inner<B>`, the outer class type with its arguments; a nested class of a
    /// non-generic outer, or a static one, is written without it.
    pub outer: Option<Box<ClassType>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum JType {
    Prim(Prim),
    Void,
    Class(ClassType),
    Var(String),
    Array(Box<JType>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeParam {
    pub name: String,
    /// `None` for `Object`, the bound every parameter has.
    pub class_bound: Option<JType>,
    pub interface_bounds: Vec<JType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassSig {
    pub tparams: Vec<TypeParam>,
    pub superclass: JType,
    pub interfaces: Vec<JType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MethodSig {
    pub tparams: Vec<TypeParam>,
    pub params: Vec<JType>,
    pub ret: JType,
    pub throws: Vec<JType>,
}

impl JType {
    pub fn class_name(name: &str) -> JType {
        JType::Class(ClassType { name: name.to_string(), args: Vec::new(), outer: None })
    }

    pub fn is_class(&self, name: &str) -> bool {
        matches!(self, JType::Class(c) if c.name == name && c.args.is_empty())
    }
}

impl TypeParam {
    pub fn bounds(&self) -> impl Iterator<Item = &JType> {
        self.class_bound.iter().chain(self.interface_bounds.iter())
    }
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("bad signature `{}` at {}: {}", String::from_utf8_lossy(self.s), self.pos, what))
    }

    fn expect(&mut self, b: u8) -> Result<(), String> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            self.err(&format!("expected `{}`", b as char))
        }
    }

    fn identifier(&mut self) -> Result<String, String> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if matches!(b, b'.' | b';' | b'[' | b'/' | b'<' | b'>' | b':') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return self.err("expected an identifier");
        }
        Ok(String::from_utf8_lossy(&self.s[start..self.pos]).into_owned())
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.err("nested too deeply");
        }
        Ok(())
    }

    fn ty(&mut self) -> Result<JType, String> {
        let Some(b) = self.peek() else { return self.err("expected a type") };
        self.pos += 1;
        let prim = match b {
            b'B' => Prim::Byte,
            b'C' => Prim::Char,
            b'D' => Prim::Double,
            b'F' => Prim::Float,
            b'I' => Prim::Int,
            b'J' => Prim::Long,
            b'S' => Prim::Short,
            b'Z' => Prim::Boolean,
            b'V' => return Ok(JType::Void),
            b'L' => return self.class_type().map(JType::Class),
            b'T' => {
                let name = self.identifier()?;
                self.expect(b';')?;
                return Ok(JType::Var(name));
            }
            b'[' => {
                self.enter()?;
                let elem = self.ty()?;
                self.depth -= 1;
                return Ok(JType::Array(Box::new(elem)));
            }
            _ => return self.err("expected a type"),
        };
        Ok(JType::Prim(prim))
    }

    /// After the `L`: `pkg/Outer<args>.Inner<args>;`
    fn class_type(&mut self) -> Result<ClassType, String> {
        self.enter()?;
        let start = self.pos;
        while let Some(b) = self.peek() {
            if matches!(b, b'<' | b';' | b'.') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return self.err("expected a class name");
        }
        let mut current = ClassType { name: String::from_utf8_lossy(&self.s[start..self.pos]).into_owned(), args: Vec::new(), outer: None };
        if self.peek() == Some(b'<') {
            current.args = self.type_args()?;
        }
        while self.peek() == Some(b'.') {
            self.pos += 1;
            let inner = self.identifier()?;
            let name = format!("{}${}", current.name, inner);
            let outer = if current.args.is_empty() && current.outer.is_none() { None } else { Some(Box::new(current)) };
            current = ClassType { name, args: Vec::new(), outer };
            if self.peek() == Some(b'<') {
                current.args = self.type_args()?;
            }
        }
        self.expect(b';')?;
        self.depth -= 1;
        Ok(current)
    }

    fn type_args(&mut self) -> Result<Vec<TypeArg>, String> {
        self.expect(b'<')?;
        let mut args = Vec::new();
        while self.peek() != Some(b'>') {
            let arg = match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    TypeArg::Wild
                }
                Some(b'+') => {
                    self.pos += 1;
                    TypeArg::Extends(self.ty()?)
                }
                Some(b'-') => {
                    self.pos += 1;
                    TypeArg::Super(self.ty()?)
                }
                Some(_) => TypeArg::Exact(self.ty()?),
                None => return self.err("unterminated type arguments"),
            };
            args.push(arg);
        }
        self.pos += 1;
        if args.is_empty() {
            return self.err("empty type arguments");
        }
        Ok(args)
    }

    fn type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        if self.peek() != Some(b'<') {
            return Ok(Vec::new());
        }
        self.pos += 1;
        let mut params = Vec::new();
        while self.peek() != Some(b'>') {
            let name = self.identifier()?;
            self.expect(b':')?;
            let class_bound = match self.peek() {
                Some(b':') => None,
                _ => Some(self.ty()?),
            };
            let mut interface_bounds = Vec::new();
            while self.peek() == Some(b':') {
                self.pos += 1;
                interface_bounds.push(self.ty()?);
            }
            let class_bound = class_bound.filter(|b| !b.is_class("java/lang/Object") || !interface_bounds.is_empty());
            params.push(TypeParam { name, class_bound, interface_bounds });
            if self.peek().is_none() {
                return self.err("unterminated type parameters");
            }
        }
        self.pos += 1;
        Ok(params)
    }

    fn done(&self) -> Result<(), String> {
        if self.pos == self.s.len() {
            Ok(())
        } else {
            self.err("trailing characters")
        }
    }
}

fn parser(s: &str) -> Parser<'_> {
    Parser { s: s.as_bytes(), pos: 0, depth: 0 }
}

/// A field signature or descriptor.
pub fn parse_type(s: &str) -> Result<JType, String> {
    let mut p = parser(s);
    let t = p.ty()?;
    p.done()?;
    Ok(t)
}

pub fn parse_class(s: &str) -> Result<ClassSig, String> {
    let mut p = parser(s);
    let tparams = p.type_params()?;
    let superclass = p.ty()?;
    let mut interfaces = Vec::new();
    while p.peek().is_some() {
        interfaces.push(p.ty()?);
    }
    Ok(ClassSig { tparams, superclass, interfaces })
}

pub fn parse_method(s: &str) -> Result<MethodSig, String> {
    let mut p = parser(s);
    let tparams = p.type_params()?;
    p.expect(b'(')?;
    let mut params = Vec::new();
    while p.peek() != Some(b')') {
        if p.peek().is_none() {
            return p.err("unterminated parameter list");
        }
        params.push(p.ty()?);
    }
    p.pos += 1;
    let ret = p.ty()?;
    let mut throws = Vec::new();
    while p.peek() == Some(b'^') {
        p.pos += 1;
        throws.push(p.ty()?);
    }
    p.done()?;
    Ok(MethodSig { tparams, params, ret, throws })
}

/// The class header from its `Signature` attribute, or from the superclass and interface names
/// of the file when it has none.
pub fn class_sig(cf: &super::ClassFile) -> Result<ClassSig, String> {
    match &cf.signature {
        Some(s) => parse_class(s),
        None => Ok(ClassSig {
            tparams: Vec::new(),
            superclass: cf.superclass.as_deref().map_or(JType::Void, JType::class_name),
            interfaces: cf.interfaces.iter().map(|i| JType::class_name(i)).collect(),
        }),
    }
}

/// A method's signature, or its descriptor when it has none; `throws` comes from the
/// `Exceptions` attribute in that case, and is what javac writes there in both cases when no
/// thrown type is a type variable.
pub fn method_sig(m: &super::Method) -> Result<MethodSig, String> {
    let mut sig = parse_method(m.signature.as_deref().unwrap_or(&m.descriptor))?;
    if sig.throws.is_empty() {
        sig.throws = m.exceptions.iter().map(|e| JType::class_name(e)).collect();
    }
    Ok(sig)
}

/// A member's signature as Scala sees it: for a constructor read from its descriptor, without
/// the parameters the compiler added in front, which a `Signature` attribute leaves out
/// already: the outer instance of an inner class, the name and ordinal of an enum.
pub fn member_sig(cf: &super::ClassFile, m: &super::Method) -> Result<MethodSig, String> {
    let mut sig = method_sig(m)?;
    if m.name != "<init>" || m.signature.is_some() {
        return Ok(sig);
    }
    let enum_prefix = cf.access & super::ACC_ENUM != 0
        && sig.params.len() >= 2
        && sig.params[0].is_class("java/lang/String")
        && sig.params[1] == JType::Prim(Prim::Int);
    if enum_prefix {
        sig.params.drain(..2);
    } else if let Some(outer) = cf.inner_outer() {
        if sig.params.first().map_or(false, |p| p.is_class(outer)) {
            sig.params.remove(0);
        }
    }
    Ok(sig)
}

pub fn field_type(f: &super::Field) -> Result<JType, String> {
    parse_type(f.signature.as_deref().unwrap_or(&f.descriptor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_generics_and_wildcards() {
        let t = parse_type("Ljava/util/Map<Ljava/lang/String;Ljava/util/List<+Ljava/lang/Number;>;>;").unwrap();
        let JType::Class(c) = t else { panic!() };
        assert_eq!(c.name, "java/util/Map");
        assert_eq!(c.args.len(), 2);
        assert!(matches!(&c.args[1], TypeArg::Exact(JType::Class(l)) if matches!(&l.args[0], TypeArg::Extends(_))));
    }

    #[test]
    fn reads_inner_of_generic_outer() {
        let t = parse_type("Lp/Outer<TA;>.Inner<TB;>;").unwrap();
        let JType::Class(c) = t else { panic!() };
        assert_eq!(c.name, "p/Outer$Inner");
        assert_eq!(c.outer.as_ref().unwrap().name, "p/Outer");
        let t = parse_type("Lp/Outer.Inner;").unwrap();
        let JType::Class(c) = t else { panic!() };
        assert_eq!(c.name, "p/Outer$Inner");
        assert!(c.outer.is_none());
    }

    #[test]
    fn reads_method_with_bounds_and_throws() {
        let m = parse_method("<T::Ljava/lang/Comparable<-TT;>;>(Ljava/util/List<TT;>;)TT;^Ljava/io/IOException;^TX;").unwrap();
        assert_eq!(m.tparams[0].name, "T");
        assert!(m.tparams[0].class_bound.is_none());
        assert_eq!(m.tparams[0].interface_bounds.len(), 1);
        assert_eq!(m.params.len(), 1);
        assert_eq!(m.throws.len(), 2);
    }

    #[test]
    fn object_bound_is_dropped() {
        let c = parse_class("<T:Ljava/lang/Object;U:Ljava/lang/Object;:Ljava/lang/Runnable;>Ljava/lang/Object;").unwrap();
        assert!(c.tparams[0].class_bound.is_none() && c.tparams[0].interface_bounds.is_empty());
        assert!(c.tparams[1].class_bound.is_some());
    }

    #[test]
    fn refuses_malformed() {
        for s in ["", "L", "Ljava/lang/String", "Lx<>;", "Lx<Ly;", "(I", "()", "<T:>()V", "[", "Q", "Lx;;", "TT", "<T:I>()V;"] {
            assert!(parse_method(s).is_err() && parse_type(s).is_err() && parse_class(s).is_err(), "{}", s);
        }
        let deep = "[".repeat(100) + "I";
        assert!(parse_type(&deep).is_err());
        let nested = "Lx<".repeat(100) + "I" + &">;".repeat(100);
        assert!(parse_type(&nested).is_err());
    }
}
