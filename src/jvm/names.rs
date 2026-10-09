//! JVM names: scalac's `$`-encoding of operator characters, the binary names of classes, and
//! the erased types the backend computes with.

use std::rc::Rc;

/// scalac's NameTransformer: operator characters become `$plus`, `$colon` and so on.
pub fn encode(name: &str) -> String {
    if name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$') {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len() + 8);
    for ch in name.chars() {
        let op = match ch {
            '~' => "$tilde",
            '=' => "$eq",
            '<' => "$less",
            '>' => "$greater",
            '!' => "$bang",
            '#' => "$hash",
            '%' => "$percent",
            '^' => "$up",
            '&' => "$amp",
            '|' => "$bar",
            '*' => "$times",
            '/' => "$div",
            '+' => "$plus",
            '-' => "$minus",
            ':' => "$colon",
            '\\' => "$bslash",
            '?' => "$qmark",
            '@' => "$at",
            _ => "",
        };
        if !op.is_empty() {
            out.push_str(op);
        } else if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            out.push(ch);
        } else {
            out.push_str(&format!("$u{:04X}", ch as u32));
        }
    }
    out
}

/// The source name of an encoded one: `$plus` is `+` again.
pub fn decode(name: &str) -> String {
    const OPS: [(&str, char); 18] = [
        ("$tilde", '~'),
        ("$eq", '='),
        ("$less", '<'),
        ("$greater", '>'),
        ("$bang", '!'),
        ("$hash", '#'),
        ("$percent", '%'),
        ("$up", '^'),
        ("$amp", '&'),
        ("$bar", '|'),
        ("$times", '*'),
        ("$div", '/'),
        ("$plus", '+'),
        ("$minus", '-'),
        ("$colon", ':'),
        ("$bslash", '\\'),
        ("$qmark", '?'),
        ("$at", '@'),
    ];
    if !name.contains('$') {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    'text: while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        for (code, ch) in OPS {
            if let Some(after) = rest.strip_prefix(code) {
                out.push(ch);
                rest = after;
                continue 'text;
            }
        }
        let escaped = rest.strip_prefix("$u").and_then(|hex| hex.get(..4)).and_then(|hex| u32::from_str_radix(hex, 16).ok()).and_then(char::from_u32);
        match escaped {
            Some(ch) => {
                out.push(ch);
                rest = &rest[6..];
            }
            None => {
                out.push('$');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub const OBJECT: &str = "java/lang/Object";
pub const STRING: &str = "java/lang/String";
pub const STRING_BUILDER: &str = "java/lang/StringBuilder";
pub const BOXED_UNIT: &str = "scala/runtime/BoxedUnit";
pub const OBJECT_REF: &str = "scala/runtime/ObjectRef";
pub const MATCH_ERROR: &str = "scala/MatchError";
pub const THROWABLE: &str = "java/lang/Throwable";
pub const ARRAY: &str = "java/util/ArrayList";
pub const OBJECT_ARRAY: &str = "[Ljava/lang/Object;";
pub const SEQ: &str = "scala/collection/immutable/Seq";
pub const SCALA_RUNTIME: &str = "scala/runtime/ScalaRunTime";
pub const PRODUCT: &str = "scala/Product";
pub const SERIALIZABLE: &str = "java/io/Serializable";
pub const TUPLE_XXL: &str = "scala/runtime/TupleXXL";
pub const EQUALS: &str = "scala/Equals";
pub const RUNTIME_MODULE: &str = "scala/runtime/jvm$package$";
pub const ENUM: &str = "scala/reflect/Enum";

/// An erased type: what a value is on the JVM stack, in a local, a field or a descriptor.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum JType {
    /// No value; the result of a method that returns `Unit`.
    V,
    Z,
    B,
    S,
    C,
    I,
    J,
    F,
    D,
    /// A class or interface by its internal name.
    L(Rc<str>),
}

impl JType {
    pub fn object() -> JType {
        JType::L(Rc::from(OBJECT))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, JType::L(n) if &**n == OBJECT)
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, JType::L(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JType::L(n) if &**n == STRING)
    }

    pub fn wide(&self) -> bool {
        matches!(self, JType::J | JType::D)
    }

    /// The typer's numeric ranks (`prims::R_BYTE` to `R_DOUBLE`).
    pub fn numeric_rank(&self) -> Option<u8> {
        use crate::typer::prims::*;
        match self {
            JType::B => Some(R_BYTE),
            JType::S => Some(R_SHORT),
            JType::C => Some(R_CHAR),
            JType::I => Some(R_INT),
            JType::J => Some(R_LONG),
            JType::F => Some(R_FLOAT),
            JType::D => Some(R_DOUBLE),
            _ => None,
        }
    }

    /// What a value of this type is on the operand stack: the small integer types are ints.
    pub fn stack_type(&self) -> JType {
        match self {
            JType::Z | JType::B | JType::S | JType::C => JType::I,
            other => other.clone(),
        }
    }

    pub fn desc(&self, out: &mut String) {
        match self {
            JType::V => out.push('V'),
            JType::Z => out.push('Z'),
            JType::B => out.push('B'),
            JType::S => out.push('S'),
            JType::C => out.push('C'),
            JType::I => out.push('I'),
            JType::J => out.push('J'),
            JType::F => out.push('F'),
            JType::D => out.push('D'),
            JType::L(n) if n.starts_with('[') => out.push_str(n),
            JType::L(n) => {
                out.push('L');
                out.push_str(n);
                out.push(';');
            }
        }
    }

    /// The class that boxes a primitive.
    pub fn box_class(&self) -> &'static str {
        match self {
            JType::Z => "java/lang/Boolean",
            JType::B => "java/lang/Byte",
            JType::S => "java/lang/Short",
            JType::C => "java/lang/Character",
            JType::I => "java/lang/Integer",
            JType::J => "java/lang/Long",
            JType::F => "java/lang/Float",
            JType::D => "java/lang/Double",
            _ => OBJECT,
        }
    }
}

pub fn method_desc(params: &[JType], ret: &JType) -> String {
    let mut s = String::with_capacity(16 + params.len() * 18);
    s.push('(');
    for p in params {
        p.desc(&mut s);
    }
    s.push(')');
    ret.desc(&mut s);
    s
}

/// Parses a field descriptor; arrays come back as `Object`-like references under their
/// descriptor text.
pub fn parse_type(desc: &str) -> (JType, &str) {
    let b = desc.as_bytes();
    match b.first() {
        Some(b'V') => (JType::V, &desc[1..]),
        Some(b'Z') => (JType::Z, &desc[1..]),
        Some(b'B') => (JType::B, &desc[1..]),
        Some(b'S') => (JType::S, &desc[1..]),
        Some(b'C') => (JType::C, &desc[1..]),
        Some(b'I') => (JType::I, &desc[1..]),
        Some(b'J') => (JType::J, &desc[1..]),
        Some(b'F') => (JType::F, &desc[1..]),
        Some(b'D') => (JType::D, &desc[1..]),
        Some(b'L') => {
            let end = desc.find(';').unwrap_or(desc.len() - 1);
            (JType::L(Rc::from(&desc[1..end])), &desc[end + 1..])
        }
        Some(b'[') => {
            let (_, rest) = parse_type(&desc[1..]);
            let len = desc.len() - rest.len();
            (JType::L(Rc::from(&desc[..len])), rest)
        }
        _ => (JType::object(), ""),
    }
}

/// The parameter types and the result of a method descriptor.
pub fn parse_method_desc(desc: &str) -> (Vec<JType>, JType) {
    let mut params = Vec::new();
    let mut rest = desc.strip_prefix('(').unwrap_or(desc);
    while !rest.is_empty() && !rest.starts_with(')') {
        let (t, r) = parse_type(rest);
        params.push(t);
        rest = r;
    }
    let (ret, _) = parse_type(rest.strip_prefix(')').unwrap_or("V"));
    (params, ret)
}
