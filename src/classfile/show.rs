//! Prints a class file's signatures in Scala-like syntax, for `teq classfile` and its golden
//! tests: the class as Scala sees it from Java, with `static` members marked, varargs as `T*`,
//! wildcards as `?`, `? <: T` and `? >: T`.

use super::sig::{self, ClassType, JType, MethodSig, TypeArg, TypeParam};
use super::*;

/// Boxed classes whose simple name is a builtin's: written in full so that `java.lang.Boolean`
/// and `Boolean` stay apart, as a Scala source has to write them.
const BOXED_CLASHES: &[&str] =
    &["java/lang/Boolean", "java/lang/Byte", "java/lang/Short", "java/lang/Long", "java/lang/Float", "java/lang/Double"];

pub struct Printer<'a> {
    cf: &'a ClassFile,
    full_names: bool,
    pub out: String,
}

impl<'a> Printer<'a> {
    pub fn new(cf: &'a ClassFile, full_names: bool) -> Printer<'a> {
        Printer { cf, full_names, out: String::new() }
    }

    /// The package of the class, `java.lang`.
    pub fn package(&self) -> String {
        self.cf.name.rfind('/').map_or(String::new(), |at| self.cf.name[..at].replace('/', "."))
    }

    /// A class by its binary name: the simple name, or the full one with `--full`; a nested
    /// class as `Outer.Inner` when the file's `InnerClasses` table names it, which it does for
    /// every nested class the file refers to.
    fn class_name(&self, binary: &str) -> String {
        self.class_name_depth(binary, 0)
    }

    fn class_name_depth(&self, binary: &str, depth: u32) -> String {
        if depth < 32 {
            if let Some(entry) = self.cf.inner_classes.iter().find(|i| i.inner == binary) {
                if let (Some(outer), Some(name)) = (&entry.outer, &entry.name) {
                    return format!("{}.{}", self.class_name_depth(outer, depth + 1), name);
                }
            }
        }
        if self.full_names || BOXED_CLASHES.contains(&binary) {
            binary.replace('/', ".")
        } else {
            binary.rfind('/').map_or(binary, |at| &binary[at + 1..]).to_string()
        }
    }

    pub fn ty(&self, t: &JType) -> String {
        match t {
            JType::Prim(p) => p.scala_name().to_string(),
            JType::Void => "Unit".to_string(),
            JType::Var(v) => v.clone(),
            JType::Array(elem) => format!("Array[{}]", self.ty(elem)),
            JType::Class(c) => self.class_type(c),
        }
    }

    fn class_type(&self, c: &ClassType) -> String {
        let mut s = match &c.outer {
            Some(outer) => {
                let inner = c.name.rfind('$').map_or(c.name.as_str(), |at| &c.name[at + 1..]);
                format!("{}#{}", self.class_type(outer), inner)
            }
            None => self.class_name(&c.name),
        };
        if !c.args.is_empty() {
            let args: Vec<String> = c.args.iter().map(|a| self.type_arg(a)).collect();
            s.push('[');
            s.push_str(&args.join(", "));
            s.push(']');
        }
        s
    }

    fn type_arg(&self, a: &TypeArg) -> String {
        match a {
            TypeArg::Wild => "?".to_string(),
            TypeArg::Extends(t) => format!("? <: {}", self.ty(t)),
            TypeArg::Super(t) => format!("? >: {}", self.ty(t)),
            TypeArg::Exact(t) => self.ty(t),
        }
    }

    fn tparams(&self, ps: &[TypeParam]) -> String {
        if ps.is_empty() {
            return String::new();
        }
        let items: Vec<String> = ps
            .iter()
            .map(|p| {
                let bounds: Vec<String> = p.bounds().map(|b| self.ty(b)).collect();
                if bounds.is_empty() {
                    p.name.clone()
                } else {
                    format!("{} <: {}", p.name, bounds.join(" & "))
                }
            })
            .collect();
        format!("[{}]", items.join(", "))
    }

    fn access(&self, flags: u16) -> String {
        let mut s = String::new();
        if flags & ACC_PRIVATE != 0 {
            s.push_str("private ");
        } else if flags & ACC_PROTECTED != 0 {
            s.push_str("protected ");
        } else if flags & ACC_PUBLIC == 0 {
            let pkg = self.package();
            let last = pkg.rsplit('.').next().unwrap_or("");
            if last.is_empty() {
                s.push_str("private[<root>] ");
            } else {
                s.push_str(&format!("private[{}] ", last));
            }
        }
        s
    }

    fn annotations(&self, names: &[String], deprecated: bool) -> String {
        let mut s = String::new();
        for n in names {
            s.push('@');
            s.push_str(&self.class_name(n));
            s.push(' ');
        }
        if deprecated && !names.iter().any(|n| n == "java/lang/Deprecated") {
            s.push_str("@Deprecated ");
        }
        s
    }

    pub fn class(&mut self) {
        let cf = self.cf;
        let header = match sig::class_sig(cf) {
            Ok(h) => h,
            Err(e) => {
                self.out.push_str(&format!("// {}: {}\n", cf.name, e));
                return;
            }
        };
        let own = cf.own_inner_entry();
        let flags = own.map_or(cf.access, |i| i.access);
        let mut line = self.annotations(&cf.annotations, cf.deprecated);
        line.push_str(&self.access(flags));
        let kind = if flags & ACC_MODULE != 0 {
            "module"
        } else if flags & ACC_ANNOTATION != 0 {
            "annotation"
        } else if flags & ACC_INTERFACE != 0 {
            "trait"
        } else if flags & ACC_ENUM != 0 {
            "enum"
        } else if cf.record.is_some() {
            "record"
        } else {
            "class"
        };
        if flags & ACC_STATIC != 0 {
            line.push_str("static ");
        }
        if flags & ACC_FINAL != 0 && kind == "class" {
            line.push_str("final ");
        }
        if flags & ACC_ABSTRACT != 0 && kind == "class" {
            line.push_str("abstract ");
        }
        if !cf.permitted.is_empty() {
            line.push_str("sealed ");
        }
        if flags & ACC_SYNTHETIC != 0 {
            line.push_str("<synthetic> ");
        }
        line.push_str(kind);
        line.push(' ');
        line.push_str(&self.class_name(&cf.name));
        line.push_str(&self.tparams(&header.tparams));
        if let Some(components) = &cf.record {
            let items: Vec<String> = components
                .iter()
                .map(|c| {
                    let t = sig::parse_type(c.signature.as_deref().unwrap_or(&c.descriptor));
                    format!("{}: {}", c.name, t.map_or_else(|e| format!("<{}>", e), |t| self.ty(&t)))
                })
                .collect();
            line.push_str(&format!("({})", items.join(", ")));
        }
        let mut parents: Vec<String> = Vec::new();
        if !matches!(header.superclass, JType::Void) && !(kind != "class" && header.superclass.is_class("java/lang/Object")) {
            parents.push(self.ty(&header.superclass));
        }
        parents.extend(header.interfaces.iter().map(|i| self.ty(i)));
        if !parents.is_empty() {
            line.push_str(" extends ");
            line.push_str(&parents.join(", "));
        }
        if !cf.permitted.is_empty() {
            let names: Vec<String> = cf.permitted.iter().map(|p| self.class_name(p)).collect();
            line.push_str(" permits ");
            line.push_str(&names.join(", "));
        }
        let member_classes: Vec<&InnerClass> = cf.member_classes().collect();
        let members = cf.fields.len() + cf.methods.iter().filter(|m| m.name != "<clinit>").count() + member_classes.len();
        if members == 0 {
            line.push('\n');
            self.out.push_str(&line);
            return;
        }
        line.push_str(" {\n");
        self.out.push_str(&line);
        for f in &cf.fields {
            let l = self.field(f);
            self.out.push_str("  ");
            self.out.push_str(&l);
            self.out.push('\n');
        }
        for m in &cf.methods {
            if m.name == "<clinit>" {
                continue;
            }
            let l = self.method(m);
            self.out.push_str("  ");
            self.out.push_str(&l);
            self.out.push('\n');
        }
        for i in member_classes {
            let l = self.nested(i);
            self.out.push_str("  ");
            self.out.push_str(&l);
            self.out.push('\n');
        }
        self.out.push_str("}\n");
    }

    fn nested(&self, i: &InnerClass) -> String {
        let mut s = self.access(i.access);
        if i.access & ACC_STATIC != 0 {
            s.push_str("static ");
        }
        if i.access & ACC_FINAL != 0 && i.access & (ACC_ENUM | ACC_INTERFACE) == 0 {
            s.push_str("final ");
        }
        if i.access & ACC_ABSTRACT != 0 && i.access & ACC_INTERFACE == 0 {
            s.push_str("abstract ");
        }
        s.push_str(if i.access & ACC_ANNOTATION != 0 {
            "annotation "
        } else if i.access & ACC_INTERFACE != 0 {
            "trait "
        } else if i.access & ACC_ENUM != 0 {
            "enum "
        } else {
            "class "
        });
        s.push_str(i.name.as_deref().unwrap_or("<anonymous>"));
        s
    }

    fn field(&self, f: &Field) -> String {
        let mut s = self.annotations(&f.annotations, f.deprecated);
        s.push_str(&self.access(f.access));
        if f.access & ACC_STATIC != 0 {
            s.push_str("static ");
        }
        if f.access & ACC_SYNTHETIC != 0 {
            s.push_str("<synthetic> ");
        }
        s.push_str(if f.access & ACC_FINAL != 0 { "val " } else { "var " });
        s.push_str(&f.name);
        s.push_str(": ");
        match sig::field_type(f) {
            Ok(t) => s.push_str(&self.ty(&t)),
            Err(e) => s.push_str(&format!("<{}>", e)),
        }
        if let Some(c) = &f.constant {
            s.push_str(" = ");
            s.push_str(&constant(c));
        }
        s
    }

    pub fn method(&self, m: &Method) -> String {
        let mut s = self.annotations(&m.annotations, m.deprecated);
        s.push_str(&self.access(m.access));
        if m.access & ACC_STATIC != 0 {
            s.push_str("static ");
        }
        if m.access & ACC_FINAL != 0 {
            s.push_str("final ");
        }
        if m.access & ACC_SYNTHETIC != 0 {
            s.push_str(if m.access & ACC_BRIDGE != 0 { "<bridge> " } else { "<synthetic> " });
        }
        s.push_str("def ");
        let sig = match sig::member_sig(self.cf, m) {
            Ok(sig) => sig,
            Err(e) => {
                s.push_str(&format!("{} <{}>", m.name, e));
                return s;
            }
        };
        s.push_str(if m.name == "<init>" { "this" } else { &m.name });
        s.push_str(&self.tparams(&sig.tparams));
        s.push_str(&self.params(m, &sig));
        if m.name != "<init>" {
            s.push_str(": ");
            s.push_str(&self.ty(&sig.ret));
        }
        if !sig.throws.is_empty() {
            let names: Vec<String> = sig.throws.iter().map(|t| self.ty(t)).collect();
            s.push_str(" throws ");
            s.push_str(&names.join(", "));
        }
        if m.access & ACC_ABSTRACT == 0 {
            s.push_str(" = ...");
        }
        s
    }

    fn params(&self, m: &Method, sig: &MethodSig) -> String {
        let names = param_names(m, sig.params.len());
        let last = sig.params.len().wrapping_sub(1);
        let items: Vec<String> = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| match p {
                JType::Array(elem) if i == last && m.access & ACC_VARARGS != 0 => format!("{}: {}*", names[i], self.ty(elem)),
                p => format!("{}: {}", names[i], self.ty(p)),
            })
            .collect();
        format!("({})", items.join(", "))
    }
}

/// The names of the declared parameters: from `MethodParameters` when the compiler wrote it,
/// aligned from the end so that the compiler-added leading parameters a signature leaves out
/// (an inner class's outer instance, an enum constant's name and ordinal) take no name; `x$1`
/// and on otherwise, which is what scalac calls them.
pub fn param_names(m: &Method, count: usize) -> Vec<String> {
    let given = m.param_names.as_deref().filter(|n| n.len() >= count).map(|n| &n[n.len() - count..]);
    (0..count)
        .map(|i| match given.and_then(|g| g[i].clone()) {
            Some(n) => n,
            None => format!("x${}", i + 1),
        })
        .collect()
}

pub fn constant(c: &ConstValue) -> String {
    match c {
        ConstValue::Int(v) => v.to_string(),
        ConstValue::Long(v) => format!("{}L", v),
        ConstValue::Float(v) => format!("{:?}f", v),
        ConstValue::Double(v) => format!("{:?}", v),
        ConstValue::Str(s) => format!("{:?}", s),
    }
}

/// One class file as `teq classfile` prints it.
pub fn print_class(path: &str, bytes: &[u8], full_names: bool, out: &mut String) -> Result<(), String> {
    let cf = ClassFile::parse(bytes)?;
    out.push_str(&format!("// {} (class file {}.{}, {} bytes)\n", path, cf.major, cf.minor, bytes.len()));
    let mut p = Printer::new(&cf, full_names);
    let pkg = p.package();
    if !pkg.is_empty() {
        out.push_str(&format!("package {}\n", pkg));
    }
    p.class();
    out.push_str(&p.out);
    Ok(())
}
