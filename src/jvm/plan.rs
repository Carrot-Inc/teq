//! The class-level plan (`lir::ClassDef`) of the class being written, as the class-file writer
//! takes it: a member's facts as a class file's access flags, the members recorded as they are
//! declared, and what the emitters ask of the class's members.

use crate::lir::{ClassDef, ClassFlags, ErasedType, FieldDef, FieldFlags, MethodDef, MethodFlags, Visibility};
use crate::types::SymId;
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
    MethodDef { name: Rc::from(name), params, ret, desc: desc.to_string(), flags }
}

/// A method of the class as `method_def` makes it, from the types of the frame its code is begun
/// with where they spell `desc`: they do but for a jar's method whose code holds a value class's
/// underlying value as the erasure marks it, under its class file's descriptor (`Gen::linked_mref`).
pub fn begun_method_def(flags: MethodFlags, name: &str, desc: &str, frame: &[(Option<SymId>, ErasedType)], ret: &ErasedType) -> MethodDef {
    if !spells_method_desc(desc, frame.iter().map(|p| &p.1), ret) {
        return method_def(flags, name, desc);
    }
    MethodDef { name: Rc::from(name), params: frame.iter().map(|p| p.1.clone()).collect(), ret: ret.clone(), desc: desc.to_string(), flags }
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
        let m = method_def(flags, name, desc);
        self.cw.method(method_access(m.flags), &m.name, &m.desc, None);
        self.plan.methods.push(m);
    }

    /// The class file of the class being written, which is the plan's class: its name, its flags
    /// and its superclass, the plan's interfaces among its own in their order, and the plan's
    /// fields and methods and no other, each one's flags and descriptor as its facts and its
    /// erased types give them.
    pub fn finish_writer(&mut self) -> Vec<u8> {
        let cw = std::mem::replace(&mut self.cw, ClassWriter::new(0, "x", OBJECT, &[]));
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
        cw.finish(self.cx.input.output_version)
    }
}

/// The class file of a class that declares its methods and has no code, field or source.
pub fn write_declarations(class: &ClassDef, output_version: u32) -> Vec<u8> {
    let interfaces: Vec<String> = class.interfaces.iter().map(|i| i.to_string()).collect();
    let mut cw = ClassWriter::new(class_access(class.flags), &class.name, &class.superclass, &interfaces);
    for m in &class.methods {
        cw.method(method_access(m.flags), &m.name, &m.desc, None);
    }
    cw.finish(output_version)
}
