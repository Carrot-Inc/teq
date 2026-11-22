//! The program's Java sources, read for their declarations as the reference reads them (dotty 3.8.4's
//! `JavaScanners` and `JavaParsers`): a class's members and their types, the bodies and initializers skipped.
//! Each top-level class or interface becomes the record a class file gives (`crate::classfile::ClassFile`), filled
//! with what the source says (`record`), which the loader enters as it enters a class of the class path
//! (`Worker::locate_java_class`): every consumer reads one kind of record whatever its origin. The bodies are
//! javac's, after teq: on the JVM the build of a program with Java sources writes teq's own classes.

pub mod ident;
pub mod parser;
pub mod record;
pub mod scanner;

use parser::{Member, TypeDecl};

/// Namer's conflicts among the names one declaration enters in one scope (`checkNoConflict`, E161), each at the later
/// name: the class's type parameters, and a method's or a constructor's type parameters and value parameters.
pub fn conflicts(decl: &TypeDecl) -> Vec<(u32, String)> {
    fn twice<'a>(names: impl Iterator<Item = (&'a str, u32)>, kind: &str, out: &mut Vec<(u32, String)>) {
        let mut seen: Vec<&str> = Vec::new();
        for (n, at) in names {
            if seen.contains(&n) {
                out.push((at, format!("{} is already defined as {} {}", n, kind, n)));
            } else {
                seen.push(n);
            }
        }
    }
    let mut out = Vec::new();
    twice(decl.tparams.iter().map(|p| (p.name.as_str(), p.at)), "type", &mut out);
    for m in &decl.members {
        let (tparams, params) = match m {
            Member::Method { tparams, params, .. } | Member::Ctor { tparams, params, .. } => (tparams, params),
            _ => continue,
        };
        twice(tparams.iter().map(|p| (p.name.as_str(), p.at)), "type", &mut out);
        twice(params.iter().map(|p| (p.name.as_str(), p.at)), "parameter", &mut out);
    }
    out
}
