//! The part of the JVM runtime that the subset cannot express: the launcher's name and the
//! function interfaces past scala-library's `Function22`. Everything else is `std/jvm.scala`,
//! compiled with the program, or scala-library's own classes; `Helpers` finds the definitions
//! of `std/jvm.scala`.

use super::classfile::*;
use super::names::*;
use super::Cx;
use crate::intern::FxMap;
use crate::symbols::*;
use crate::types::*;

pub const LAUNCHER: &str = "TeqMain";

/// The definitions of package `scala.runtime` that generated code calls.
#[derive(Default)]
pub struct Helpers {
    pub defs: FxMap<String, SymId>,
    pub classes: FxMap<String, ClassId>,
}

impl Helpers {
    pub fn find(cx: &Cx) -> Helpers {
        let syms = cx.input.syms;
        let interner = cx.input.interner;
        let mut out = Helpers::default();
        let sub = |p: PkgId, name: &str| {
            syms.pkg(p).entries.iter().find(|(n, e)| e.pkg.is_some() && interner.get(**n) == name).and_then(|(_, e)| e.pkg)
        };
        let Some(runtime) = sub(ROOT_PKG, "scala").and_then(|s| sub(s, "runtime")) else { return out };
        for (name, entry) in &syms.pkg(runtime).entries {
            if let Some(s) = entry.term {
                out.defs.insert(interner.get(*name).to_string(), s);
            }
            if let Some(c) = entry.class {
                out.classes.insert(interner.get(*name).to_string(), c);
            }
        }
        out
    }
}

/// The classes scalac erases `Nothing` and `Null` to, which scala-library holds: the component
/// of their arrays, and their type in a descriptor.
pub const ARRAY_MARKERS: [&str; 2] = ["scala/runtime/Nothing$", "scala/runtime/Null$"];

/// `scala.FunctionN` for an arity scala-library has no interface for (above 22): lambdas of
/// that many parameters need one all the same.
pub fn function_interface(arity: usize, output_version: u32) -> (String, Vec<u8>) {
    let name = format!("scala/Function{}", arity);
    let mut cw = ClassWriter::new(ACC_PUBLIC | ACC_INTERFACE | ACC_ABSTRACT, &name, OBJECT, &[]);
    let desc = format!("({})Ljava/lang/Object;", "Ljava/lang/Object;".repeat(arity));
    cw.method(ACC_PUBLIC | ACC_ABSTRACT, "apply", &desc, None);
    (name, cw.finish(output_version))
}

