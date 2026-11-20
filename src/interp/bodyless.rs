//! The proof a program's use of a class needs where the class has Java ancestors without a body: the interpreter
//! runs no Java code, so a static initializer the use would run (JLS 12.4.1, JVMS 5.5) or a Java constructor a
//! creation skips must be proven to do nothing the run can see, or the use is refused where it runs.
//!
//! A use that initializes a class (a creation, an object's construction, `Class.forName` with its initialization)
//! asks `initialization_refusal`: the class's superclasses and the superinterfaces that declare a method neither
//! abstract nor static (JVMS 5.5, step 7), each Java one without a body to have no static initializer in its own
//! class file (`classfile::ctor`). A class whose construction a native gives (zio's padding classes,
//! `natives::zio_queue_padding`) passes, and so does a JDK class a native answers a member of (`native_answered`):
//! the interpreter's own implementation of it, its initialization included. A creation then walks the
//! superclasses' constructors: the one its subclass calls must be empty. Above a Java level a typed one passes only
//! where its construction does nothing (`trivial_ctor`): the Java branches of `construct` and `construct_via`
//! construct nothing above them, and a body there could have written the fields a Java level stores its defaults
//! into. Natives that make JDK objects (`Path`, an exception `jdk.rs` throws) do not come here.
use crate::classfile::ctor::{Ctor, InitFacts};
use super::*;
use std::rc::Rc;

/// The Java classes whose construction a native gives: zio's padding chain, whose constructors chain to their parent
/// and leave the counters at zero, its static initializer `natives::zio_queue_padding`'s (which keeps its own artifact
/// and version restriction where the statics are read).
const NATIVE_CONSTRUCTIONS: [&str; 5] = [
    "zio.internal.MutableQueueFieldsPadding",
    "zio.internal.TailPadding",
    "zio.internal.PreTailPadding",
    "zio.internal.HeadPadding",
    "zio.internal.ClassFieldsPadding",
];

/// The proofs of one thread's runs: each Java class's facts, each creation's verdict, and per class whether its
/// creation needs a proof at all (0 not asked, 1 no, 2 yes).
pub(super) struct JavaProofs {
    facts: FxMap<ClassId, Rc<Result<InitFacts, String>>>,
    creations: FxMap<(ClassId, Option<SymId>), Option<Rc<str>>>,
    needs: crate::arena::Dense<u8>,
    /// The JDK classes a native answers a member of (`native_answered`), read off the builtins once.
    native_classes: Option<Rc<std::collections::HashSet<String, crate::intern::FxBuild>>>,
}

impl Default for JavaProofs {
    fn default() -> JavaProofs {
        JavaProofs { facts: FxMap::default(), creations: FxMap::default(), needs: crate::arena::Dense::new(0), native_classes: None }
    }
}

/// The JDK classes whose members a native answers outside the builtins' table: `jdk_member`'s (class.rs) and
/// `natives::java_statics`'.
const NATIVE_MEMBER_CLASSES: [&str; 4] = ["java.nio.file.Path", "java.time.ZoneId", "java.util.ResourceBundle", "java.io.File"];

/// A level of the superclass chain, from the created class up.
enum Level {
    Typed(ClassId),
    Native(ClassId),
    Java { class: ClassId, stores: bool },
}

impl Interp<'_, '_> {
    /// Whether `c` is a Java class the interpreter has no typed body for. A placeholder (a class a signature named)
    /// is read first, under the loader's lock; one no class file holds stays the empty trait it was, with nothing to
    /// run.
    fn bodyless_java(&mut self, c: ClassId) -> bool {
        if self.typer.is_java_placeholder(c) {
            self.typer.with_loader(|t| t.complete_class(c));
        }
        self.typer.is_java_class(c) && self.tclass_index(c).is_none()
    }

    /// Whether `c` is a JDK class a native answers a member of: a builtin named under it (`java.lang.Integer.parseInt`
    /// is `java.lang.Integer`'s) or one of `NATIVE_MEMBER_CLASSES`.
    fn native_answered(&mut self, c: ClassId) -> bool {
        // The loader's record of where a class came from is the lock holder's, as its class file is (`java_facts`).
        if !self.typer.with_loader(|t| t.java_class_from_jdk(c)) {
            return false;
        }
        let classes = match &self.java_proofs.native_classes {
            Some(s) => s.clone(),
            None => {
                let s = Rc::new(native_classes(self.builtins.keys()));
                self.java_proofs.native_classes = Some(s.clone());
                s
            }
        };
        let path = self.class_path(c);
        classes.contains(path.trim_end_matches('$'))
    }

    /// The facts of the Java class `c`'s own class file, read once.
    fn java_facts(&mut self, c: ClassId) -> Rc<Result<InitFacts, String>> {
        if let Some(f) = self.java_proofs.facts.get(&c) {
            return f.clone();
        }
        // The JDK's record and the class path are the loader lock holder's.
        let f = Rc::new(self.typer.with_loader(|t| t.java_class_bytes(c)).and_then(|b| InitFacts::read(&b)));
        self.java_proofs.facts.insert(c, f.clone());
        f
    }

    /// Why a program's creation of `c` is refused, through `via` where `c` is a Java class (the constructor the
    /// creation names); `None` where nothing in it needs a body the interpreter lacks.
    #[inline]
    pub(super) fn creation_refusal(&mut self, c: ClassId, via: Option<SymId>) -> Option<Rc<str>> {
        match self.java_proofs.needs.get(c.idx()) {
            Some(1) => None,
            _ => self.creation_refusal_slow(c, via),
        }
    }

    #[cold]
    #[inline(never)]
    fn creation_refusal_slow(&mut self, c: ClassId, via: Option<SymId>) -> Option<Rc<str>> {
        let i = c.idx();
        if self.java_proofs.needs.get(i).copied().unwrap_or(0) == 0 {
            let bases: Vec<ClassId> = self.syms().class(c).base_types.iter().map(|&(b, _)| b).collect();
            let needs = bases.into_iter().any(|b| self.bodyless_java(b));
            *self.java_proofs.needs.at(i) = if needs { 2 } else { 1 };
            if !needs {
                return None;
            }
        }
        let key = (c, via.filter(|_| self.typer.is_java_class(c)));
        if let Some(v) = self.java_proofs.creations.get(&key) {
            return v.clone();
        }
        let v = self.prove_creation(c, key.1).map(|why| Rc::from(why.as_str()));
        self.java_proofs.creations.insert(key, v.clone());
        v
    }

    /// Whether `o` is the object of a Java class's statics, which holds what the natives give (`natives::java_statics`).
    /// Every field store of the run asks it: a run that made no statics object answers by one flag.
    #[inline]
    pub(super) fn java_statics_object(&mut self, o: &Rc<Object>) -> bool {
        self.made.any_java_statics
            && self.made.modules.get(o.class.idx()).map_or(false, |m| m.as_ref().map_or(false, |m| Rc::ptr_eq(m, o)))
            && self.typer.is_java_class(o.class)
            && self.tclass_index(o.class).is_none()
    }

    /// The refusal of a static field of a Java class that no native gives, read or written.
    #[cold]
    #[inline(never)]
    pub(super) fn refuse_static<T>(&mut self, statics: ClassId, name: Name) -> R<T> {
        let class = self.class_path(statics);
        let field = self.name(name).to_string();
        self.refuse(format!("{}.{} is a static field of a Java class, which the interpreter has no body for", class.trim_end_matches('$'), field))
    }

    /// The body-less Java class or interface among `c`'s ancestors that declares `name`: where a call on an instance
    /// of `c` that finds no member ends.
    #[cold]
    #[inline(never)]
    pub(super) fn java_definition_of(&mut self, c: ClassId, name: Name) -> Option<ClassId> {
        let bases: Vec<ClassId> = self.syms().class(c).base_types.iter().map(|&(b, _)| b).collect();
        bases.into_iter().find(|&b| self.syms().class(b).members.contains_key(&name) && self.bodyless_java(b))
    }

    /// What initializing `c` initializes, in the order the JVM runs their static initializers (JLS 12.4.2, JVMS 5.5
    /// steps 7 and 9), each with whether it is an interface. A class: its superclass first (initialized the same way),
    /// then the superinterfaces, direct or indirect, that declare a method neither abstract nor static (for a class
    /// path's Java interface, a method of its class file with neither `ACC_ABSTRACT` nor `ACC_STATIC`: a default or a
    /// private instance method, `InitFacts::concrete_instance_method`), in the order of the enumeration that visits an
    /// interface's own superinterfaces before it, then the class itself. An interface: itself alone, as initializing
    /// an interface initializes none of its superinterfaces. A Java interface whose class file is not read is listed,
    /// as it may declare one.
    fn initialization_order(&mut self, c: ClassId) -> Vec<(ClassId, bool)> {
        let mut order: Vec<(ClassId, bool)> = Vec::new();
        let mut seen: Vec<ClassId> = Vec::new();
        if self.syms().class(c).kind == crate::symbols::ClassKind::Trait {
            order.push((c, true));
        } else {
            self.initialize_class(c, &mut seen, &mut order);
        }
        order
    }

    fn initialize_class(&mut self, k: ClassId, seen: &mut Vec<ClassId>, order: &mut Vec<(ClassId, bool)>) {
        if seen.contains(&k) || self.class_path(k) == "java.lang.Object" {
            return;
        }
        seen.push(k);
        if self.typer.is_java_class(k) {
            self.typer.with_loader(|t| t.complete_class(k));
        }
        if let Some(sup) = self.jvm_superclass(k) {
            self.initialize_class(sup, seen, order);
        }
        for i in self.direct_interfaces(k) {
            self.initialize_superinterface(i, seen, order);
        }
        order.push((k, false));
    }

    /// The enumeration's step for an interface a class implements: its own superinterfaces first, then itself where
    /// it declares a method neither abstract nor static.
    fn initialize_superinterface(&mut self, i: ClassId, seen: &mut Vec<ClassId>, order: &mut Vec<(ClassId, bool)>) {
        if seen.contains(&i) {
            return;
        }
        seen.push(i);
        if self.typer.is_java_class(i) {
            self.typer.with_loader(|t| t.complete_class(i));
        }
        for j in self.direct_interfaces(i) {
            self.initialize_superinterface(j, seen, order);
        }
        // A Scala trait's initialization is the interpreter's own: only a body-less Java interface can refuse.
        if self.bodyless_java(i) {
            let concrete = match self.java_facts(i).as_ref() {
                Ok(f) => f.concrete_instance_method,
                Err(_) => true,
            };
            if concrete {
                order.push((i, true));
            }
        }
    }

    /// `k`'s superclass on the JVM: the typed record's, or else its first parent that is a class. The record leaves
    /// none for a case of an enum that extends no class (the enum is its parent, a class all the same).
    fn jvm_superclass(&mut self, k: ClassId) -> Option<ClassId> {
        if let Some(sup) = self.syms().class(k).superclass {
            return Some(sup);
        }
        let parents = self.syms().class(k).parents.clone();
        for t in parents {
            if let Some(p) = self.typer.class_of(t) {
                if p != k && self.syms().class(p).kind != crate::symbols::ClassKind::Trait {
                    return Some(p);
                }
            }
        }
        None
    }

    /// The interfaces among `k`'s parents, in the order written.
    fn direct_interfaces(&mut self, k: ClassId) -> Vec<ClassId> {
        let parents = self.syms().class(k).parents.clone();
        let mut out = Vec::new();
        for t in parents {
            if let Some(p) = self.typer.class_of(t) {
                if self.syms().class(p).kind == crate::symbols::ClassKind::Trait && !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        out
    }

    /// Why a use that initializes `c` is refused: the first body-less Java class or interface the JVM's initialization
    /// of `c` runs a static initializer (`<clinit>`) of (`initialization_order`), which the interpreter cannot run, but
    /// for a JDK class a native answers a member of: the interpreter's own implementation of it, its initialization
    /// included (`native_answered`). `subject` names the use in the words (`the creation of C`,
    /// `Class.forName("C")`). A class's file not read is the constructor's walk's to report for a creation
    /// (`creation`), this one's for any other use.
    pub(super) fn initialization_refusal(&mut self, c: ClassId, subject: &str, creation: bool) -> Option<String> {
        for (k, interface) in self.initialization_order(c) {
            if !self.bodyless_java(k) {
                continue;
            }
            let name = self.class_path(k);
            if NATIVE_CONSTRUCTIONS.contains(&name.as_str()) || self.native_answered(k) {
                continue;
            }
            let what = if interface { "interface" } else { "class" };
            let facts = self.java_facts(k);
            match facts.as_ref() {
                Ok(f) if f.static_initializer => {
                    return Some(format!("{} initializes the Java {} {}, whose static initializer the interpreter has no body for", subject, what, name))
                }
                Ok(_) => {}
                Err(_) if creation && !interface => {}
                Err(why) if interface && k != c => {
                    return Some(format!("{} can initialize the Java interface {}, whose class file is not read: {}", subject, name, why))
                }
                Err(why) => return Some(format!("{} initializes the Java {} {}, whose class file is not read: {}", subject, what, name, why)),
            }
        }
        None
    }

    /// The walk: the initialization the creation makes (`initialization_refusal`), then the superclass chain's
    /// constructors.
    fn prove_creation(&mut self, c: ClassId, via: Option<SymId>) -> Option<String> {
        let made = self.class_path(c);
        if let Some(why) = self.initialization_refusal(c, &format!("the creation of {}", made), true) {
            return Some(why);
        }
        let mut levels: Vec<Level> = Vec::new();
        // The constructor the level below calls: the creation's own for a Java class created directly, the parent
        // call's for a typed class's Java superclass, `()V` above a proven Java level (the pattern's super call).
        let mut called: Option<String> = via.and_then(|s| self.typer.with_loader(|t| t.java_ctor_descriptor(s)));
        let mut at = Some(c);
        while let Some(k) = at {
            if self.typer.is_java_class(k) {
                self.typer.with_loader(|t| t.complete_class(k));
            }
            let sup = self.syms().class(k).superclass;
            if self.class_path(k) == "java.lang.Object" {
                break;
            }
            if !self.bodyless_java(k) {
                // `called` for the superclass: its parent call's constructor where it is Java.
                called = match self.tclass_index(k) {
                    Some(ti) => self.prog().classes[ti].parent_via.and_then(|s| self.typer.with_loader(|t| t.java_ctor_descriptor(s))),
                    None => None,
                };
                if self.tclass_index(k).is_none() && sup.is_none() {
                    // `Object` or `Any`: the root.
                    break;
                }
                levels.push(Level::Typed(k));
                at = sup;
                continue;
            }
            let name = self.class_path(k);
            if NATIVE_CONSTRUCTIONS.contains(&name.as_str()) {
                levels.push(Level::Native(k));
                called = Some("()V".to_string());
                at = sup;
                continue;
            }
            let facts = self.java_facts(k);
            let facts = match facts.as_ref() {
                Ok(f) => f,
                Err(why) => return Some(format!("the creation of {} runs the constructor of the Java class {}, whose class file is not read: {}", made, name, why)),
            };
            let descriptor = match called.take() {
                Some(d) => d,
                None if facts_ctor_count(facts) == 1 => facts_only_ctor(facts),
                None => return Some(format!("the creation of {} runs a constructor of the Java class {} the interpreter does not know", made, name)),
            };
            match facts.ctor(&descriptor) {
                Some(Ctor::Empty { stores }) => levels.push(Level::Java { class: k, stores: *stores }),
                Some(Ctor::Other(_)) | None => {
                    return Some(format!("the creation of {} runs the constructor of the Java class {}, which does more than call its superclass's and store defaults", made, name))
                }
            }
            // The class file's superclass is the one of the class file the loader typed it from.
            if self.typer.java_class_superclass(k).as_ref() != Some(&facts.superclass) {
                return Some(format!("the creation of {} runs the constructor of the Java class {}, whose class file names another superclass than the one typed", made, name));
            }
            called = Some("()V".to_string());
            at = sup;
        }
        // Above a Java level, a typed level passes only where its construction does nothing (the interpreter's own
        // test of a trivial level, `trivial_ctor`): the Java branches of `construct` and `construct_via` run nothing
        // above them, so a body there would be skipped, and none can have written a field a Java level stores into.
        // Above a storing Java level, a native one does not pass either: it is a construction the proof does not see.
        for (i, level) in levels.iter().enumerate() {
            let java = match *level {
                Level::Java { class, .. } | Level::Native(class) => class,
                Level::Typed(_) => continue,
            };
            let stores = matches!(*level, Level::Java { stores: true, .. });
            for above in &levels[i + 1..] {
                match *above {
                    Level::Typed(t) if !self.trivial_ctor(t, 0) => {
                        let (name, typed) = (self.class_path(java), self.class_path(t));
                        return Some(format!(
                            "the creation of {} runs the Java class {}, above which the interpreter does not run the constructor of {}",
                            made, name, typed
                        ));
                    }
                    Level::Native(n) if stores => {
                        let (name, native) = (self.class_path(java), self.class_path(n));
                        return Some(format!(
                            "the creation of {} runs the constructor of the Java class {}, which stores defaults into its fields after the construction of {}, which can have written them",
                            made, name, native
                        ));
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// The JDK classes a native answers a member of: each builtin named under a JDK package (`java.lang.Integer.parseInt`
/// is `java.lang.Integer`'s), and `NATIVE_MEMBER_CLASSES`.
fn native_classes<'a>(builtins: impl Iterator<Item = &'a String>) -> std::collections::HashSet<String, crate::intern::FxBuild> {
    let mut s: std::collections::HashSet<String, crate::intern::FxBuild> = NATIVE_MEMBER_CLASSES.iter().map(|c| c.to_string()).collect();
    for name in builtins {
        if name.starts_with("java.") || name.starts_with("javax.") || name.starts_with("jdk.") {
            if let Some((class, _)) = name.rsplit_once('.') {
                s.insert(class.to_string());
            }
        }
    }
    s
}

fn facts_ctor_count(f: &InitFacts) -> usize {
    f.ctor_descriptors().count()
}

fn facts_only_ctor(f: &InitFacts) -> String {
    f.ctor_descriptors().next().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    /// The names under which the interpreter's natives stand for a JDK class's initialization (`native_answered`):
    /// pinned, so that a builtin added under another JDK class widens the boundary only with this list. Some are
    /// packages (a builtin named under a package) or the std's own classes, which no JDK class's path equals.
    #[test]
    fn the_jdk_classes_a_native_answers() {
        let table = super::super::builtins::table();
        let mut got: Vec<String> = super::native_classes(table.keys()).into_iter().collect();
        got.sort();
        let want: Vec<&str> = vec![
            "java.io", "java.io.File", "java.lang", "java.lang.Boolean", "java.lang.Byte", "java.lang.Character",
            "java.lang.Class", "java.lang.Comparable", "java.lang.Double", "java.lang.Float", "java.lang.Integer",
            "java.lang.Long", "java.lang.Math", "java.lang.Number", "java.lang.Runtime", "java.lang.Short",
            "java.lang.String", "java.lang.System", "java.lang.Throwable", "java.lang.invoke", "java.lang.reflect",
            "java.math", "java.net", "java.nio.file", "java.nio.file.Path", "java.security", "java.time",
            "java.time.Duration", "java.time.Instant", "java.time.ZoneId", "java.util", "java.util.Arrays",
            "java.util.Objects", "java.util.ResourceBundle", "java.util.Store", "java.util.TimeZone",
            "java.util.concurrent", "java.util.concurrent.atomic", "java.util.concurrent.locks",
            "java.util.regex.Engine", "java.util.zip"
        ];
        assert_eq!(got, want);
    }
}
