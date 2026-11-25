//! The class-level plan (`lir::ClassDef`) of the class being written, as the class-file writer
//! takes it: a member's facts as a class file's access flags, the members recorded as they are
//! declared, and what the emitters ask of the class's members.

use crate::lir::{ClassDef, ClassFlags, ErasedType, FieldDef, FieldFlags, MethodDef, MethodFlags, Visibility};
use crate::types::{ClassId, SymId};
use super::classfile::*;
use super::gen::Gen;
use super::names::*;
use std::rc::Rc;

/// `MODULE$` of a module class, which its `<clinit>` alone stores, and a field of the same flags.
pub const MODULE_FIELD: FieldFlags = FieldFlags { visibility: Visibility::Public, is_static: true, is_final: true, ..FieldFlags::PRIVATE };

fn visibility_access(v: Visibility) -> u16 {
    match v {
        Visibility::Public => ACC_PUBLIC,
        Visibility::Package => 0,
        Visibility::Private => ACC_PRIVATE,
    }
}

fn bit(set: bool, flag: u16) -> u16 {
    if set {
        flag
    } else {
        0
    }
}

pub fn class_access(f: ClassFlags) -> u16 {
    ACC_PUBLIC | if f.interface { ACC_INTERFACE } else { ACC_SUPER } | bit(f.is_abstract, ACC_ABSTRACT) | bit(f.is_final, ACC_FINAL) | bit(f.is_enum, ACC_ENUM)
}

pub fn field_access(f: FieldFlags) -> u16 {
    visibility_access(f.visibility) | bit(f.is_static, ACC_STATIC) | bit(f.is_final, ACC_FINAL) | bit(f.volatile, ACC_VOLATILE) | bit(f.synthetic, ACC_SYNTHETIC) | bit(f.enum_value, ACC_ENUM)
}

pub fn method_access(f: MethodFlags) -> u16 {
    visibility_access(f.visibility) | bit(f.is_static, ACC_STATIC) | bit(f.is_final, ACC_FINAL) | bit(f.is_abstract, ACC_ABSTRACT) | bit(f.synthetic, ACC_SYNTHETIC) | bit(f.bridge, ACC_BRIDGE)
}

/// A method of the class by its flags, its name and its signature as a class file spells it.
pub fn method_def(flags: MethodFlags, name: &str, desc: &str) -> MethodDef {
    let (params, ret) = parse_method_desc(desc);
    MethodDef { name: Rc::from(name), params, ret, desc: desc.to_string(), flags, place: None }
}

/// A method of the class as `method_def` makes it, from the types of the frame its code is begun
/// with where they spell `desc`: they do but for a jar's method whose code holds a value class's
/// underlying value as the erasure marks it, under its class file's descriptor (`Gen::linked_mref`).
pub fn begun_method_def(flags: MethodFlags, name: &str, desc: &str, frame: &[(Option<SymId>, ErasedType)], ret: &ErasedType) -> MethodDef {
    if !spells_method_desc(desc, frame.iter().map(|p| &p.1), ret) {
        return method_def(flags, name, desc);
    }
    MethodDef { name: Rc::from(name), params: frame.iter().map(|p| p.1.clone()).collect(), ret: ret.clone(), desc: desc.to_string(), flags, place: None }
}

impl<'a> Gen<'a> {
    /// Starts a class: its plan with no member yet, and its writer from the plan. `listed` is what
    /// its class file names after the interfaces its parents give it: a module class's
    /// `java/io/Serializable` (`module_interfaces`).
    pub fn begin_class(&mut self, name: &str, flags: ClassFlags, superclass: &str, interfaces: &[String], listed: &[String], source: Option<&str>) {
        let class = ClassDef {
            name: Rc::from(name),
            flags,
            superclass: Rc::from(superclass),
            interfaces: interfaces.iter().map(|i| Rc::from(i.as_str())).collect(),
            source: source.map(Rc::from),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let all: Vec<String> = interfaces.iter().chain(listed).cloned().collect();
        self.cw = ClassWriter::new(class_access(class.flags), &class.name, &class.superclass, &all);
        if let Some(file) = &class.source {
            self.cw.source_file(file);
        }
        self.plan = class;
        self.module_tree = None;
        self.writing_mirror = false;
        self.long_name_refused = false;
        self.checked_methods.clear();
    }

    /// Declares a field of the class, after those declared so far.
    pub fn field(&mut self, flags: FieldFlags, name: &str, ty: &ErasedType) {
        let mut desc = String::new();
        ty.desc(&mut desc);
        self.cw.field(field_access(flags), name, &desc);
        self.plan.fields.push(FieldDef { name: Rc::from(name), ty: ty.clone(), flags });
    }

    /// Declares a field of the class before those declared so far, as dotty's
    /// `LazyVals.addInFront` puts a lazy member's `VarHandle` before the template's statements.
    pub fn field_first(&mut self, flags: FieldFlags, name: &str, ty: &ErasedType) {
        let mut desc = String::new();
        ty.desc(&mut desc);
        self.cw.field_first(field_access(flags), name, &desc);
        self.plan.fields.insert(0, FieldDef { name: Rc::from(name), ty: ty.clone(), flags });
    }

    /// A method the class declares without a body.
    pub fn abstract_method(&mut self, flags: MethodFlags, name: &str, desc: &str) {
        self.abstract_method_at(flags, name, desc, None);
    }

    /// `abstract_method` of a method whose definition the writer knows.
    pub fn abstract_method_at(&mut self, flags: MethodFlags, name: &str, desc: &str, at: Option<(crate::source::FileId, crate::source::Span)>) {
        self.abstract_method_site(flags, name, desc, super::classes::MethodSite { at, ..Default::default() });
    }

    /// `abstract_method` with what the writer knows of the method's definition (`MethodSite`).
    pub fn abstract_method_site(&mut self, flags: MethodFlags, name: &str, desc: &str, site: super::classes::MethodSite) {
        let m = method_def(flags, name, desc);
        self.check_method(&m, &site);
        self.cw.method(method_access(m.flags), &m.name, &m.desc, None);
        self.plan.methods.push(m);
    }

    /// The class file of the class being written, which is the plan's class: its name, its flags
    /// and its superclass, the plan's interfaces among its own in their order, and the plan's
    /// fields and methods and no other, each one's flags and descriptor as its facts and its
    /// erased types give them, the methods in the plan's order (`ClassDef::order_lifted`).
    pub fn finish_writer(&mut self) -> Vec<u8> {
        let mut cw = std::mem::replace(&mut self.cw, ClassWriter::new(0, "x", OBJECT, &[]));
        // dotty checks the name as the class begins (BCodeSkelBuilder.scala 341-345, for a mirror
        // BCodeHelpers.scala 529-531) and writes nothing of the class.
        if !constant_text_fits(&self.plan.name, "") {
            let message = if self.writing_mirror { "Mirror class name is too long for the JVM" } else { "Class name is too long for the JVM" };
            self.name_too_long(message, None);
            return Vec::new();
        }
        if let Some(limit) = cw.cp.unwritable() {
            return self.class_too_large(limit);
        }
        if let Some(m) = self.plan.methods.iter().find(|m| !self.checked_methods.contains_key(&(m.name.clone(), m.desc.clone()))) {
            panic!("the method {}{} of {} entered the plan past its gate (`Gen::check_method`)", m.name, m.desc, self.plan.name);
        }
        self.plan.order_lifted();
        cw.order_methods(self.plan.methods.iter().map(|m| (&*m.name, m.desc.as_str())));
        let class = &self.plan;
        if !cw.is_class(class_access(class.flags), &class.name, &class.superclass) || !cw.holds_interfaces(class.interfaces.iter().map(|i| &**i)) {
            panic!("the plan of {} is not its class file's: {:?} {} {:?}", class.name, class.flags, class.superclass, class.interfaces);
        }
        let fields: Vec<(u16, &str, String)> = class.fields.iter().map(|f| (field_access(f.flags), &*f.name, self.desc_of(&f.ty))).collect();
        if let Some(at) = cw.fields_differ(&fields) {
            panic!("the plan of {} is not its class file's from field {} on: {:?}", class.name, at, fields.get(at));
        }
        let methods: Vec<(u16, &str, String)> = class.methods.iter().map(|m| (method_access(m.flags), &*m.name, method_desc(&m.params, &m.ret))).collect();
        if let Some(at) = cw.methods_differ(&methods) {
            panic!("the plan of {} is not its class file's from method {} on: {:?}", class.name, at, methods.get(at));
        }
        match cw.finish(self.cx.input.output_version) {
            Ok(bytes) => bytes,
            Err(limit) => self.class_too_large(limit),
        }
    }

    /// A class whose constant pool passes what a class file holds is not written: scalac's error
    /// (dotty's `PostProcessor.sendToDisk` reporting ASM's `ClassTooLargeException`, or the
    /// `IllegalArgumentException` of a text past a constant's bytes), at the class's definition.
    fn class_too_large(&mut self, limit: PoolLimit) -> Vec<u8> {
        // The text is a name already refused where dotty refuses it.
        if limit == PoolLimit::Text && self.long_name_refused {
            return Vec::new();
        }
        let name = self.plan.name.clone();
        let message = match limit {
            PoolLimit::Entries => format!("Error while emitting {}\nClass too large: {}", name, name),
            PoolLimit::Text => format!("Error while emitting {}\nUTF8 string too large", name),
        };
        match self.class_place(&name) {
            Some((file, span)) => self.refused.push((file, span, message)),
            None => self.unsupported(&message),
        }
        self.pool_too_large = true;
        Vec::new()
    }

    /// scalac's error for a name or a signature past a text constant's bytes, at `at` or else at
    /// the class's definition.
    pub(super) fn name_too_long(&mut self, message: &str, at: Option<(crate::source::FileId, crate::source::Span)>) {
        self.refuse_written(message, at);
        self.long_name_refused = true;
    }

    /// scalac's error `message` for what of the class being written a class file cannot hold, at `at` or
    /// else at the class's definition.
    pub(super) fn refuse_written(&mut self, message: &str, at: Option<(crate::source::FileId, crate::source::Span)>) {
        let name = self.plan.name.clone();
        match at.or_else(|| self.class_place(&name)) {
            Some((file, span)) => self.refused.push((file, span, message.to_string())),
            None => self.unsupported(message),
        }
    }

    /// Where the class of the binary name `name` is defined: the class being written's own place,
    /// the class, the companion or the module that name stands for, or the file of a file's
    /// module class.
    pub(super) fn class_place(&self, name: &str) -> Option<(crate::source::FileId, crate::source::Span)> {
        let cx = self.cx;
        let of_class = |c: ClassId| {
            let info = cx.input.syms.class(c);
            (info.file, info.span)
        };
        if let Some(c) = self.this_class {
            return Some(of_class(c));
        }
        let module = format!("{name}$");
        if let Some(&c) = cx.class_by_name.get(name).or_else(|| cx.class_by_name.get(name.strip_suffix('$')?)).or_else(|| cx.class_by_name.get(&module)) {
            return Some(of_class(c));
        }
        let f = cx.file_modules.iter().position(|m| m == name)?;
        Some((crate::source::FileId(f as u32), crate::source::Span::default()))
    }
}

/// The class file of a class that declares its methods and has no code, field or source.
pub fn write_declarations(class: &ClassDef, output_version: u32) -> Vec<u8> {
    let interfaces: Vec<String> = class.interfaces.iter().map(|i| i.to_string()).collect();
    let mut cw = ClassWriter::new(class_access(class.flags), &class.name, &class.superclass, &interfaces);
    for m in &class.methods {
        cw.method(method_access(m.flags), &m.name, &m.desc, None);
    }
    cw.finish(output_version).expect("a class of declarations alone, within a class file's bounds")
}
