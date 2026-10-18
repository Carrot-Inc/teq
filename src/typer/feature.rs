//! The feature warning of a definition of an old-style implicit conversion, dotty's
//! `Checking.checkImplicitConversionDefOK` and `checkFeature`: an `implicit def` whose first
//! parameter list holds one parameter and is no implicit or using clause, written as such (the
//! conversion an implicit class makes is not), warns `Definition of implicit conversion method f
//! should be enabled`, at its name, unless the feature `implicitConversions` is enabled: by
//! `--language implicitConversions` (scalac's `-language:implicitConversions`) or by an
//! `import scala.language.implicitConversions` in scope (`Feature.enabledByImport`, the imports
//! the parser keeps with their scopes in `Ast::language_scopes`). A `FeatureWarning`, which the
//! reporting policy summarizes unless `--feature`; the first of each top-level definition, the
//! use site scalac's reporter remembers (`isReportedFeatureUseSite`), explains the feature
//! (`Diagnostics::report`).

use super::Worker;
use crate::ast::{self, mods, DefKind};
use crate::intern::Name;
use crate::source::{FileId, Span};

/// The explanation the first feature warning of a use site adds (`report.featureWarning`).
pub fn explanation(feature: &str) -> String {
    format!("\nSee the Scala docs for value scala.language.{} for a discussion\nwhy the feature should be explicitly enabled.", feature)
}

impl<'a> Worker<'a> {
    /// The definition `d` of `file`, checked as an implicit conversion's.
    #[inline]
    pub(super) fn check_conversion_def(&mut self, file: FileId, d: ast::DefId) {
        if self.ast(file).def(d).mods & mods::IMPLICIT != 0 {
            self.check_implicit_def(file, d);
            self.check_scala2_implicit(file, d);
        }
    }

    /// `Checking.checkScala2Implicit`: an implicit definition of the program, reported by its unit's
    /// version (`MigrationVersion.Scala2Implicits`) with a remedy in its own terms: a class's
    /// extension and `Conversion`, a conversion's `Conversion`, a method's given of its function type,
    /// a value's given, with the note on a given's laziness but for a lazy value or an object. A
    /// parameter is exempt (`TermParamOrAccessor`), and so is the conversion an implicit class makes
    /// (`isOldStyleImplicitConversion(forImplicitClassOnly = true)`).
    #[cold]
    #[inline(never)]
    fn check_scala2_implicit(&mut self, file: FileId, d: ast::DefId) {
        use crate::source_version::{Migration, Outcome};
        if !self.program_source(file) || Migration::Scala2Implicits.outcome(self.ast(file).source_version) == Outcome::Silent {
            return;
        }
        let ast = self.ast(file);
        let def = ast.def(d);
        let name = self.name_str(def.name);
        let made = |ast: &ast::Ast| ast.defs.iter().any(|k| matches!(k.kind, DefKind::Class(_)) && k.name == def.name && k.span == def.span && k.mods & mods::IMPLICIT != 0);
        let (span, msg) = match &def.kind {
            DefKind::Class(cls) if cls.kind != ast::ClassKind::Object => {
                let Some(&c) = self.def_classes.get(file.0 as usize, &d) else { return };
                let first = self.syms.class(c).ctor.first().and_then(|cl| cl.params.first()).cloned();
                let (pname, ptype) = match first {
                    Some(p) => (self.name_str(p.name), self.param_type_text(&p)),
                    None => ("x".to_string(), "T".to_string()),
                };
                let msg = format!(
                    "`implicit` classes are no longer supported. They can usually be replaced\nby extension methods. Example:\n\n    extension ({pname}: {ptype})\n      // class methods go here, replace `this` by `{pname}`\n\nAlternatively, convert to a regular class and define\na given `Conversion` instance into that class. Example:\n\n    class {name} ...\n    given Conversion[{ptype}, {name}] = {name}({pname})"
                );
                (def.span, msg)
            }
            DefKind::Class(_) => {
                let Some(&c) = self.def_classes.get(file.0 as usize, &d) else { return };
                let shown = format!("object {}", self.owned_path(c));
                let start = ast.def_range(d).start;
                (Span::new(start, start), format!("`implicit` vals are no longer supported, use a `given` clause instead. Example:\n\n   given {name}: {shown} = ..."))
            }
            DefKind::Fun(f) => {
                if made(ast) {
                    return;
                }
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                let sig = self.sig_arc(sym);
                let conversion = !f.is_extension && matches!(sig.clauses.first(), Some(c) if c.params.len() == 1 && !c.is_using);
                let msg = if conversion {
                    let p = &sig.clauses[0].params[0];
                    let (pname, ptype) = (self.name_str(p.name), self.param_type_text(p));
                    let result = self.show(sig.ret);
                    format!("`implicit` conversion methods are no longer supported. They can usually be\nreplaced by given instances of class `Conversion`. Example:\n\n    given Conversion[{ptype}, {result}] = {pname} => ...")
                } else {
                    let fun = self.function_text(&sig);
                    format!("`implicit` defs are no longer supported, use a `given` clause instead. Example:\n\n   given {name}: {fun} = ...")
                };
                (def.span, msg)
            }
            DefKind::Val { .. } => {
                let Some(&sym) = self.def_syms.get(file.0 as usize, &d) else { return };
                let ty = self.sig_arc(sym).ret;
                let shown = self.show(ty);
                let note = if def.mods & mods::LAZY != 0 {
                    String::new()
                } else {
                    format!("\n\nNote: given clauses are evaluated lazily unless the right hand side is\na simple reference.  If eager evaluation of the value's right hand side\nis important, you can define a regular val and a given instance like this:\n\n    val {name} = ...\n    given {shown} = {name}")
                };
                (def.span, format!("`implicit` vals are no longer supported, use a `given` clause instead. Example:\n\n   given {name}: {shown} = ...{note}"))
            }
            _ => return,
        };
        self.migration_in(file, Migration::Scala2Implicits, span, crate::warnings::NO_ID, 0, || msg);
    }

    /// A class's name with its owners', outermost first (an object's type `object p.O` shows it).
    fn owned_path(&self, c: crate::types::ClassId) -> String {
        let info = self.syms.class(c);
        let own = self.name_str(info.name);
        match info.owner {
            crate::symbols::Owner::Class(o) => format!("{}.{}", self.owned_path(o), own),
            crate::symbols::Owner::Package(p) => {
                let path = self.pkg_path(p);
                if path.is_empty() { own } else { format!("{}.{}", path, own) }
            }
            crate::symbols::Owner::Local => own,
        }
    }

    /// A parameter's whole type as dotty shows it: a by-name one's `=> T` (`ExprType`), a repeated one's `T*`.
    fn param_type_text(&mut self, p: &crate::symbols::ParamSig) -> String {
        let ty = self.show(p.ty);
        format!("{}{}{}", if p.by_name { "=> " } else { "" }, ty, if p.repeated { "*" } else { "" })
    }

    /// A method's type as a function type (`Checking.checkScala2Implicit`'s `toFunctionStr`): its type
    /// parameters, then each clause, as `[A] => (a: A) => R`, a parameterless method's `() => R` (`ExprType`).
    fn function_text(&mut self, sig: &crate::symbols::MethodSig) -> String {
        let mut out = String::new();
        if !sig.tparams.is_empty() {
            let mut tparams = self.sig_text(&crate::symbols::MethodSig { tparams: sig.tparams.clone(), clauses: Vec::new(), ret: crate::types::ANY, declared: sig.declared });
            // `[A]: Any` of the type parameters alone.
            tparams.truncate(tparams.rfind("]: ").map_or(tparams.len(), |i| i + 1));
            out.push_str(&tparams);
            out.push_str(" => ");
        }
        // `ExprType`'s `() => T` is a parameterless method's; one with type parameters is a `PolyType` over
        // its result itself (`def f[A]: Int` is `[A] => Int`).
        if sig.clauses.is_empty() && sig.tparams.is_empty() {
            out.push_str("() => ");
        }
        for cl in &sig.clauses {
            out.push('(');
            for (i, p) in cl.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let ty = self.param_type_text(p);
                out.push_str(&format!("{}: {}", self.name_str(p.name), ty));
            }
            out.push_str(") => ");
        }
        let ret = self.show(sig.ret);
        out.push_str(&ret);
        out
    }

    /// An implicit definition's check, apart from the common path.
    #[cold]
    #[inline(never)]
    fn check_implicit_def(&mut self, file: FileId, d: ast::DefId) {
        let ast = self.ast(file);
        let def = ast.def(d);
        if !self.program_source(file) {
            return;
        }
        let DefKind::Fun(f) = &def.kind else { return };
        if f.is_extension || !matches!(f.clauses.first(), Some(c) if c.params.len() == 1 && !c.is_using && !c.is_implicit) {
            return;
        }
        // The conversion of an implicit class, which the parser makes beside it at its place.
        let made = ast.defs.iter().any(|k| matches!(k.kind, DefKind::Class(_)) && k.name == def.name && k.span == def.span && k.mods & mods::IMPLICIT != 0);
        if made || self.feature_enabled(file, def.span.start, crate::names::IMPLICIT_CONVERSIONS) {
            return;
        }
        let use_site = self.top_level_def_of(file, d);
        let msg = format!(
            "Definition of implicit conversion method {} should be enabled\nby adding the import clause 'import scala.language.implicitConversions'\nor by setting the compiler option --language implicitConversions.",
            self.name_str(def.name)
        );
        let w = crate::source::Warning { id: crate::warnings::NO_ID, category: crate::warnings::Category::Feature, origin: Some(format!("{}:{}", file.0, use_site).into_boxed_str()), phase: 0, unplaced: false };
        let file_was = std::mem::replace(&mut self.env.file, file);
        self.warn_as(Span::new(def.span.start, def.span.start + 1), msg, w);
        self.env.file = file_was;
    }

    /// Whether `feature` is enabled at `pos` of `file`: by the command line, or by the innermost
    /// `scala.language` import in scope there that names it or all features.
    pub(super) fn feature_enabled(&self, file: FileId, pos: u32, feature: Name) -> bool {
        if self.diags.policy.language.contains(&self.name_str(feature)) {
            return true;
        }
        // The innermost clause that names it decides (`ImportInfo.featureImported`): the latest
        // to start among those in scope, of one statement its last clause.
        let ast = self.ast(file);
        ast.language_scopes
            .iter()
            .filter(|(n, _, scope)| *n == feature && scope.start <= pos && pos < scope.end)
            .max_by_key(|(_, _, scope)| scope.start)
            .is_some_and(|&(_, enables, _)| enables)
    }

    /// The top-level class of `file` that holds `d`, scalac's `ctx.owner.topLevelClass`: its
    /// index among the definitions, or `u32::MAX` for the file's package object, which holds its
    /// top-level methods and values.
    fn top_level_def_of(&self, file: FileId, d: ast::DefId) -> u32 {
        let ast = self.ast(file);
        let at = ast.def_range(d);
        ast.top_level
            .iter()
            .find(|&&t| {
                let r = ast.def_range(t);
                matches!(ast.def(t).kind, DefKind::Class(_)) && r.start <= at.start && at.end <= r.end
            })
            .map_or(u32::MAX, |t| t.0)
    }
}
