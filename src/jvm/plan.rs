//! The class-level plan (`lir::ClassDef`) of the class being written, as the class-file writer
//! takes it: a member's facts as a class file's access flags, the members recorded as they are
//! declared, and what the emitters ask of the class's members.

use crate::lir::{ClassDef, ClassFlags, ErasedType, FieldDef, FieldFlags, FieldRole, MethodDef, MethodFlags, MethodParam, Visibility};
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

/// A method of the class by its flags, its name and its signature as a class file spells it, each parameter one no
/// symbol stands for.
pub fn method_def(flags: MethodFlags, name: &str, desc: &str) -> MethodDef {
    let (params, ret) = parse_method_desc(desc);
    MethodDef::new(Rc::from(name), params.into_iter().map(|t| MethodParam::declared(None, t)).collect(), ret, flags, None)
}

/// A method of the class with the parameters of the frame its code is begun with, at the frame's types where they
/// spell `desc`: they do but for a jar's method whose code holds a value class's underlying value as the erasure
/// marks it, which takes its class file's descriptor's (`Gen::linked_mref`).
pub fn begun_method_def(flags: MethodFlags, name: &str, desc: &str, frame: &[MethodParam], ret: &ErasedType, place: Option<u32>) -> MethodDef {
    if !spells_method_desc(desc, frame.iter().map(|p| &p.ty), ret) {
        let (types, ret) = parse_method_desc(desc);
        let mut params: Vec<MethodParam> = types.into_iter().map(|t| MethodParam::declared(None, t)).collect();
        for (p, framed) in params.iter_mut().zip(frame) {
            p.role = framed.role.clone();
        }
        return MethodDef::new(Rc::from(name), params, ret, flags, place);
    }
    MethodDef::new(Rc::from(name), frame.to_vec(), ret.clone(), flags, place)
}

impl ClassDef {
    /// Whether the class holds a method of this name whose types `desc` spells (`spells_method_desc`): compared by the
    /// types, with no descriptor made.
    pub fn has_method(&self, name: &str, desc: &str) -> bool {
        self.methods.iter().any(|m| &**m.name() == name && spells_method_desc(desc, m.params().iter().map(|p| &p.ty), m.ret()))
    }
}

impl<'a> Gen<'a> {
    /// Starts a class: its plan with no member yet, and its writer from the plan. `listed` is what
    /// its class file names after the interfaces its parents give it: a module class's
    /// `java/io/Serializable` (`module_interfaces`).
    pub fn begin_class(&mut self, name: &str, flags: ClassFlags, superclass: &str, interfaces: &[String], listed: &[String], source: Option<&str>, place: Option<(crate::source::FileId, crate::source::Span)>) {
        let class = ClassDef {
            name: Rc::from(name),
            flags,
            superclass: Rc::from(superclass),
            interfaces: interfaces.iter().map(|i| Rc::from(i.as_str())).collect(),
            source: source.map(Rc::from),
            place,
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let all: Vec<String> = interfaces.iter().chain(listed).cloned().collect();
        self.cw = ClassWriter::new(class_access(class.flags), &class.name, &class.superclass, &all);
        if let Some(file) = &class.source {
            self.cw.source_file(file);
        }
        self.cw.cp.plan_frozen = true;
        debug_assert!(self.entries.is_empty(), "a class begun while the plan of {} is open", self.plan.name);
        self.walking = false;
        self.plan = class;
        self.module_tree = None;
        self.writing_mirror = false;
        self.long_name_refused = false;
        self.checked_methods.clear();
    }

    /// Declares a field of the class, after those declared so far, holding what its `role` says where it holds a
    /// hidden value of the class's code.
    pub fn field(&mut self, flags: FieldFlags, name: &str, ty: &ErasedType, role: Option<FieldRole>) {
        let field = FieldDef { name: Rc::from(name), ty: ty.clone(), flags, role };
        self.entries.push(super::classes::Entry::Field(field.clone()));
        self.plan.fields.push(field);
    }

    /// The class's field of the role `role`.
    pub fn role_field(&self, role: &FieldRole) -> Option<&FieldDef> {
        self.plan.fields.iter().find(|f| f.role.as_ref() == Some(role))
    }

    /// Declares a field of the class before those declared so far, as dotty's
    /// `LazyVals.addInFront` puts a lazy member's `VarHandle` before the template's statements.
    pub fn field_first(&mut self, flags: FieldFlags, name: &str, ty: &ErasedType) {
        let field = FieldDef { name: Rc::from(name), ty: ty.clone(), flags, role: None };
        self.entries.push(super::classes::Entry::FieldFirst(field.clone()));
        self.plan.fields.insert(0, field);
    }

    /// Enters a method in the class plan unless its name and descriptor already occur.
    pub(super) fn enter_method(&mut self, method: &MethodDef, desc: &str, what: &str) -> bool {
        if self.plan.has_method(method.name(), desc) {
            self.unsupported(&format!("internal error: a second {} {}{} in the class", what, method.name(), desc));
            return false;
        }
        self.plan.methods.push(method.clone());
        true
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
        if self.enter_method(&m, desc, "method") {
            self.entries.push(super::classes::Entry::Abstract(m, desc.to_string()));
        }
    }

    /// The class file of the class being written, which is the plan's class, as the class's walk writes it from the
    /// plan's entries (`Gen::write_entries`): its name, its flags and its superclass, the plan's interfaces among its
    /// own in their order, and the plan's fields and methods and no other, each one's flags and descriptor as its facts
    /// and its erased types give them, the methods in the plan's order (`ClassDef::order_lifted`).
    pub fn finish_writer(&mut self) -> Vec<u8> {
        let written = self.write_entries();
        self.lir.clear();
        if !written {
            return Vec::new();
        }
        self.cw.cp.plan_frozen = false;
        self.finish_writer_now()
    }

    fn finish_writer_now(&mut self) -> Vec<u8> {
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
        self.plan.order_lifted();
        let descs: Vec<String> = self.plan.methods.iter().map(|m| method_desc(&m.param_types(), m.ret())).collect();
        if let Some((m, desc)) = self.plan.methods.iter().zip(&descs).find(|(m, desc)| !self.checked_methods.contains_key(&(m.name().clone(), (*desc).clone()))) {
            panic!("the method {}{} of {} entered the plan past its gate (`Gen::check_method`)", m.name(), desc, self.plan.name);
        }
        cw.order_methods(self.plan.methods.iter().zip(&descs).map(|(m, desc)| (&**m.name(), desc.as_str())));
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
        match self.plan.place {
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
        match at.or(self.plan.place) {
            Some((file, span)) => self.refused.push((file, span, message.to_string())),
            None => self.unsupported(message),
        }
    }
}

/// The class file of a class that declares its methods and has no code, field or source.
pub fn write_declarations(class: &ClassDef, output_version: u32) -> Vec<u8> {
    let interfaces: Vec<String> = class.interfaces.iter().map(|i| i.to_string()).collect();
    let mut cw = ClassWriter::new(class_access(class.flags), &class.name, &class.superclass, &interfaces);
    for m in &class.methods {
        cw.method(method_access(m.flags()), m.name(), &method_desc(&m.param_types(), m.ret()), None);
    }
    cw.finish(output_version).expect("a class of declarations alone, within a class file's bounds")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lir::ParamRole;

    fn param(ty: ErasedType, role: ParamRole) -> MethodParam {
        MethodParam { ty, role }
    }

    fn object() -> ErasedType {
        ErasedType::L(Rc::from("java/lang/Object"))
    }

    fn parts(m: &MethodDef) -> Vec<(ErasedType, ParamRole)> {
        m.params().iter().map(|p| (p.ty.clone(), p.role.clone())).collect()
    }

    #[test]
    fn a_frame_shorter_than_its_descriptor_keeps_every_parameter() {
        let m = begun_method_def(MethodFlags::PUBLIC, "m", "(Ljava/lang/Object;IJ)V", &[param(object(), ParamRole::Instance)], &ErasedType::V, None);
        assert_eq!(parts(&m), vec![(object(), ParamRole::Instance), (ErasedType::I, ParamRole::Declared(None)), (ErasedType::J, ParamRole::Declared(None))]);
        assert_eq!(*m.ret(), ErasedType::V);
        assert_eq!(method_desc(&m.param_types(), m.ret()), "(Ljava/lang/Object;IJ)V");
    }

    #[test]
    fn a_frame_of_another_type_takes_the_descriptors_with_its_roles() {
        let frame = [param(object(), ParamRole::Key { holder: None }), param(ErasedType::I, ParamRole::Instance)];
        let m = begun_method_def(MethodFlags::PUBLIC, "m", "(Lp/Box;I)Lp/Box;", &frame, &object(), None);
        let boxed = ErasedType::L(Rc::from("p/Box"));
        assert_eq!(parts(&m), vec![(boxed.clone(), ParamRole::Key { holder: None }), (ErasedType::I, ParamRole::Instance)]);
        assert_eq!(*m.ret(), boxed);
        assert_eq!(method_desc(&m.param_types(), m.ret()), "(Lp/Box;I)Lp/Box;");
    }

    #[test]
    fn a_frame_that_spells_its_descriptor_is_the_methods() {
        let frame = [param(ErasedType::I, ParamRole::Instance), param(ErasedType::J, ParamRole::Key { holder: None })];
        let m = begun_method_def(MethodFlags::PUBLIC, "m", "(IJ)D", &frame, &ErasedType::D, None);
        assert_eq!(parts(&m), vec![(ErasedType::I, ParamRole::Instance), (ErasedType::J, ParamRole::Key { holder: None })]);
        assert_eq!(*m.ret(), ErasedType::D);
        assert_eq!(method_desc(&m.param_types(), m.ret()), "(IJ)D");
    }
}
